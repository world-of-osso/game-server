//! Logged aura state. Damage is paid only by recorded hits, never by these auras.
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

use super::healer_wcl_trace::Trace;
use crate::auras::observations::ObservedAuraRemovals;
use crate::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};

#[derive(Deserialize)]
pub(super) struct DebuffEpisode {
    source_actor: i64,
    source_instance: u64,
    target_actor: i64,
    spell_id: u32,
    end_ms: u64,
    updates: Vec<DebuffUpdate>,
}

impl DebuffEpisode {
    pub fn accepts_tick(&self, source: (i64, u64), target: i64, time_ms: u64) -> bool {
        source == (self.source_actor, self.source_instance)
            && target == self.target_actor
            && self
                .updates
                .first()
                .is_some_and(|update| update.time_ms <= time_ms)
            && time_ms <= self.end_ms
    }
}

#[derive(Deserialize)]
struct DebuffUpdate {
    time_ms: u64,
    stacks: u8,
    evidence: String,
}

pub(super) fn validate(
    episodes: &[DebuffEpisode],
    slots: &BTreeMap<i64, usize>,
) -> Result<(), String> {
    for episode in episodes {
        if episode.source_actor <= 0
            || episode.spell_id == 0
            || !slots
                .get(&episode.target_actor)
                .is_some_and(|slot| *slot < 5)
            || episode.updates.is_empty()
            || episode.updates.iter().any(|update| {
                update.stacks == 0 || update.evidence.is_empty() || update.time_ms >= episode.end_ms
            })
            || episode
                .updates
                .windows(2)
                .any(|pair| pair[0].time_ms > pair[1].time_ms)
        {
            return Err("WCL_DEBUFF_EPISODE_INVALID".into());
        }
    }
    Ok(())
}

struct ReplayedEpisode {
    source: Entity,
    target: Entity,
    instance: Option<u32>,
    next_update: usize,
    dispelled: bool,
}

pub(super) struct DebuffReplay {
    episodes: Vec<ReplayedEpisode>,
    removal_cursor: usize,
    pub dropped_ticks: usize,
}

impl DebuffReplay {
    pub fn new(world: &mut World, trace: &Trace, slots: &[Entity; 5]) -> Self {
        world.init_resource::<ObservedAuraRemovals>();
        let mut sources = BTreeMap::new();
        let episodes = trace
            .debuffs
            .iter()
            .map(|episode| {
                let source = if let Some(&slot) = trace.actor_slots.get(&episode.source_actor) {
                    slots[slot]
                } else {
                    *sources
                        .entry((episode.source_actor, episode.source_instance))
                        .or_insert_with(|| {
                            // Aura-only source identities must not become extra attackable enemies.
                            world
                                .spawn(shared::components::UnitFactionTemplate(14))
                                .id()
                        })
                };
                ReplayedEpisode {
                    source,
                    target: slots[trace.actor_slots[&episode.target_actor]],
                    instance: None,
                    next_update: 0,
                    dispelled: false,
                }
            })
            .collect();
        Self {
            episodes,
            removal_cursor: world.resource::<ObservedAuraRemovals>().0.len(),
            dropped_ticks: 0,
        }
    }

    pub fn advance(
        &mut self,
        world: &mut World,
        episodes: &[DebuffEpisode],
        elapsed_ms: u64,
    ) -> Result<(), String> {
        self.observe_dispels(world);
        for (episode, state) in episodes.iter().zip(&mut self.episodes) {
            if episode.end_ms <= elapsed_ms {
                if let Some(instance) = state.instance.take() {
                    auras::remove_instance(world, state.target, instance, RemoveReason::Expired);
                }
                continue;
            }
            while let Some(update) = episode
                .updates
                .get(state.next_update)
                .filter(|update| update.time_ms < elapsed_ms + 50)
            {
                state.next_update += 1;
                if state.dispelled || !crate::spell_cast::targets::is_alive(world, state.target) {
                    continue;
                }
                apply_update(world, episode, state, update, elapsed_ms)?;
            }
        }
        Ok(())
    }

    fn observe_dispels(&mut self, world: &World) {
        let receipts = &world.resource::<ObservedAuraRemovals>().0;
        for &(target, instance, reason) in &receipts[self.removal_cursor..] {
            if reason != RemoveReason::Dispelled {
                continue;
            }
            for state in &mut self.episodes {
                if state.target == target && state.instance == Some(instance) {
                    state.dispelled = true;
                }
            }
        }
        self.removal_cursor = receipts.len();
    }

    pub fn drop_tick(&mut self, episode: Option<usize>) -> bool {
        let drop = episode.is_some_and(|index| self.episodes[index].dispelled);
        self.dropped_ticks += usize::from(drop);
        drop
    }
}

fn apply_update(
    world: &mut World,
    episode: &DebuffEpisode,
    state: &mut ReplayedEpisode,
    update: &DebuffUpdate,
    elapsed_ms: u64,
) -> Result<(), String> {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(episode.spell_id)
        .ok_or_else(|| format!("WCL_DEBUFF_SPELL_MISSING:{}", episode.spell_id))?
        .clone();
    if spell
        .categories
        .as_ref()
        .is_none_or(|category| category.dispel_type == 0)
    {
        return Err(format!("WCL_DEBUFF_NOT_DISPELLABLE:{}", spell.id));
    }
    for effect in &mut spell.effects {
        effect.aura_period_ms = 0;
    }
    let application = AuraApplication {
        caster: state.source,
        target: state.target,
        spell_id: spell.id,
        effect_mask: auras::aura_effect_mask(&spell),
        source: AuraSource::Cast,
    };
    auras::apply_aura_with_spell(world, &application, &spell);
    if !auras::set_owned_stacks(world, state.source, state.target, spell.id, update.stacks) {
        return Err(format!("WCL_DEBUFF_NOT_APPLIED:{}", spell.id));
    }
    let remaining =
        u32::try_from(episode.end_ms - elapsed_ms).map_err(|_| "WCL_DEBUFF_DURATION_OVERFLOW")?;
    auras::set_owned_duration(world, state.source, state.target, spell.id, remaining);
    state.instance = world
        .get::<ActiveAuras>(state.target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == state.source && aura.spell_id == spell.id)
        })
        .map(|aura| aura.instance_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_cast::test_harness::{harness, spawn_player};
    use shared::protocol::CombatLogKind;

    #[test]
    #[ignore = "requires native Paralyzing Shots and Purify data"]
    fn logged_debuff_state_preserves_control_without_damage_and_drops_only_dispelled_episode_ticks()
    {
        for seed in [1, 2] {
            let _rng = crate::random::seed_scope(seed);
            let mut h = harness(Vec::new(), &[1294569, 527], 5).unwrap();
            let other = spawn_player(&mut h.app, 5, 3.0);
            let slots = [h.caster, other, h.caster, h.caster, h.caster];
            let trace: Trace = serde_json::from_value(serde_json::json!({
            "provenance":"native debuff lifecycle fixture", "excluded_self":0,
            "party_specs":[256,256,256,256,256], "actor_slots":{"1":0,"2":1},
            "bloodlust":[], "opening_auras":[], "hits":[], "debuffs":[
              {"source_actor":10,"source_instance":1,"target_actor":1,"spell_id":1294569,
               "end_ms":2000,"updates":[{"time_ms":0,"stacks":1,"evidence":"applydebuff"},
                 {"time_ms":1500,"stacks":1,"evidence":"refreshdebuff"}]},
              {"source_actor":10,"source_instance":2,"target_actor":2,"spell_id":1294569,
               "end_ms":2000,"updates":[{"time_ms":0,"stacks":1,"evidence":"applydebuff"}]},
              {"source_actor":10,"source_instance":1,"target_actor":1,"spell_id":1294569,
               "end_ms":4000,"updates":[{"time_ms":2500,"stacks":1,"evidence":"applydebuff"}]}
            ]}))
            .unwrap();
            let mut replay = DebuffReplay::new(h.app.world_mut(), &trace, &slots);
            replay
                .advance(h.app.world_mut(), &trace.debuffs, 0)
                .unwrap();
            let aura = h
                .app
                .world()
                .get::<ActiveAuras>(h.caster)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 1294569)
                .unwrap();
            assert!(aura.harmful && aura.dispel_type == 1);
            assert!(
                aura.effects
                    .iter()
                    .any(|effect| effect.aura_type == 33 && effect.amount != 0.0)
            );
            h.advance(25);
            assert!(!h.inbox.log.iter().any(
                |event| event.kind == CombatLogKind::Damage && event.spell_id == Some(1294569)
            ));
            // The recorded tick is the only damage paid before the native dispel.
            let hit: super::super::healer_wcl_trace::RecordedHit =
                serde_json::from_value(serde_json::json!({
                    "time_ms":1000,"slot":0,"source_actor":10,"source_instance":1,"target_actor":1,
                    "spell_id":1294569,"school":8,"defense":1,"hit_type":1,"periodic":true,
                    "amount":100,"absorbed":0,"max_hp":1000,"health_after":900,"debuff_episode":0
                }))
                .unwrap();
            assert!(!replay.drop_tick(hit.debuff_episode));
            assert!(matches!(
                hit.deliver(h.app.world_mut(), h.npc, &slots).unwrap(),
                super::super::healer_wcl_trace::Delivery::Applied(_)
            ));
            h.cast(527, Some(h.caster));
            h.advance(2);
            replay
                .advance(h.app.world_mut(), &trace.debuffs, 1350)
                .unwrap();
            assert!(
                h.inbox
                    .log
                    .iter()
                    .any(|event| event.kind == CombatLogKind::Dispel
                        && event.extra_spell_id == Some(1294569))
            );
            assert!(replay.drop_tick(Some(0)));
            assert!(!replay.drop_tick(Some(1)));
            assert!(!replay.drop_tick(None));
            assert_eq!(replay.dropped_ticks, 1);
            replay
                .advance(h.app.world_mut(), &trace.debuffs, 1500)
                .unwrap();
            assert!(
                !auras::has_aura(h.app.world(), h.caster, 1294569),
                "recorded refresh cannot resurrect dispelled episode"
            );
            assert!(auras::has_aura(h.app.world(), other, 1294569));
            replay
                .advance(h.app.world_mut(), &trace.debuffs, 2000)
                .unwrap();
            assert!(!auras::has_aura(h.app.world(), other, 1294569));
            replay
                .advance(h.app.world_mut(), &trace.debuffs, 2500)
                .unwrap();
            assert!(auras::has_aura(h.app.world(), h.caster, 1294569));
            assert!(
                !replay.drop_tick(Some(2)),
                "a new application is a new episode"
            );
            println!(
                "{}",
                serde_json::json!({"seed":seed,"ticks_dropped_after_native_dispel":replay.dropped_ticks})
            );
        }
    }
}
