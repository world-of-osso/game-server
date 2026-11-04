//! WCL post-mitigation damage receipts, normalized to native recipient HP.
use bevy::prelude::{Entity, World};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::components::Health;
use std::path::Path;

pub(super) enum Delivery {
    Applied(Value),
    Avoided,
    TargetDead,
}

#[derive(Deserialize)]
pub(super) struct Trace {
    pub provenance: String,
    pub excluded_self: usize,
    pub party_specs: [u32; 5],
    pub actor_slots: std::collections::BTreeMap<i64, usize>,
    pub bloodlust: Vec<TimedBloodlust>,
    pub opening_auras: Vec<OpeningAura>,
    #[serde(default)]
    pub replayed_casts: Vec<ReplayedCast>,
    pub hits: Vec<RecordedHit>,
}

/// Matched-pull replay input: a party member's logged cast of a spell its APL must not choose.
#[derive(Deserialize)]
pub(super) struct ReplayedCast {
    time_ms: u64,
    slot: usize,
    spell_id: u32,
    evidence: String,
}

/// Longer than any replay, so Red Thirst-style reductions cannot make a held spell ready.
const REPLAY_HOLD_MS: f32 = 1.0e9;

/// Keep each replayed spell on cooldown so only `cast_due_replayed_casts` casts it.
pub(super) fn hold_replayed_casts(world: &mut World, slots: &[Entity; 5], casts: &[ReplayedCast]) {
    for cast in casts {
        crate::spell_cast::scripts::start_hasted_cooldown(
            world,
            slots[cast.slot],
            cast.spell_id,
            REPLAY_HOLD_MS,
        );
    }
}

pub(super) fn cast_due_replayed_casts(
    world: &mut World,
    slots: &[Entity; 5],
    casts: &[ReplayedCast],
    next: &mut usize,
    elapsed_ms: u64,
) -> Result<(), String> {
    use server_follower_apl::literal_rotation::execution::{Execution, execute};
    while let Some(cast) = casts
        .get(*next)
        .filter(|cast| cast.time_ms < elapsed_ms + 50)
    {
        let unit = slots[cast.slot];
        // Clears the hold exactly; a finite reduction can leave rounding residue.
        crate::spell_cast::scripts::reduce_cooldown(world, unit, cast.spell_id, f64::INFINITY);
        execute(world, unit, Execution::Spell(cast.spell_id, unit, None)).map_err(|error| {
            format!(
                "WCL_REPLAYED_CAST_REFUSED:{}:{}:{error:?}",
                cast.time_ms, cast.spell_id
            )
        })?;
        crate::spell_cast::scripts::start_hasted_cooldown(
            world,
            unit,
            cast.spell_id,
            REPLAY_HOLD_MS,
        );
        *next += 1;
    }
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct OpeningAura {
    source_actor: i64,
    target_actor: i64,
    spell_id: u32,
    stacks: u8,
    evidence: String,
    remove_ms: Option<u64>,
}

pub(super) struct OpeningRemoval {
    time_ms: u64,
    source: Entity,
    target: Entity,
    spell_id: u32,
    instance_id: u32,
    refreshed: bool,
}

pub(super) fn preserve_refreshed_opening_auras(
    removals: &mut [OpeningRemoval],
    events: &[shared::protocol::CombatLogEvent],
) {
    for event in events {
        if event.kind != shared::protocol::CombatLogKind::AuraRefreshed {
            continue;
        }
        for removal in removals.iter_mut() {
            if event.source == Some(removal.source.to_bits())
                && event.target == Some(removal.target.to_bits())
                && event.spell_id == Some(removal.spell_id)
            {
                removal.refreshed = true;
            }
        }
    }
}

pub(super) fn remove_due_opening_auras(
    world: &mut World,
    removals: &[OpeningRemoval],
    next: &mut usize,
    elapsed_ms: u64,
) {
    while let Some(removal) = removals.get(*next).filter(|r| r.time_ms <= elapsed_ms) {
        if !removal.refreshed {
            crate::auras::remove_instance(
                world,
                removal.target,
                removal.instance_id,
                crate::auras::RemoveReason::Expired,
            );
        }
        *next += 1;
    }
}

impl Trace {
    pub fn apply_opening_auras(
        &self,
        world: &mut World,
        slots: &[Entity; 5],
    ) -> Result<Vec<OpeningRemoval>, String> {
        use crate::auras::{self, AuraApplication, AuraSource};
        use crate::spell_info::SpellStore;
        let mut removals = Vec::new();
        let mut opening_auras: Vec<_> = self.opening_auras.iter().collect();
        // Apply Bulwark's parent before its separately recorded absorb aura.
        opening_auras.sort_by_key(|aura| aura.spell_id == 432607);
        for aura in opening_auras {
            let source = *slots
                .get(*self.actor_slots.get(&aura.source_actor).ok_or_else(|| {
                    format!("WCL_OPENING_AURA_UNKNOWN_SOURCE:{}", aura.source_actor)
                })?)
                .ok_or("WCL_OPENING_AURA_INVALID_SOURCE_SLOT")?;
            let target = *slots
                .get(*self.actor_slots.get(&aura.target_actor).ok_or_else(|| {
                    format!("WCL_OPENING_AURA_UNKNOWN_TARGET:{}", aura.target_actor)
                })?)
                .ok_or("WCL_OPENING_AURA_INVALID_TARGET_SLOT")?;
            let spell = world
                .resource::<SpellStore>()
                .get(aura.spell_id)
                .ok_or_else(|| format!("WCL_OPENING_AURA_MISSING_SPELL:{}", aura.spell_id))?;
            let mask = auras::aura_effect_mask(spell);
            if mask == 0 && aura.spell_id == 372014 {
                eprintln!(
                    "WCL_OPENING_AURA_SKIPPED:{}:{}:unmodeled_cosmetic_form",
                    aura.target_actor, aura.spell_id
                );
                continue;
            }
            if mask == 0 || aura.stacks == 0 || aura.evidence.is_empty() {
                return Err(format!("WCL_OPENING_AURA_INVALID:{}", aura.spell_id));
            }
            let application = AuraApplication {
                caster: source,
                target,
                spell_id: aura.spell_id,
                effect_mask: mask,
                source: AuraSource::Cast,
            };
            if aura.spell_id == 432607 {
                apply_opening_bulwark(world, &application);
            } else {
                auras::apply_aura(world, &application);
            }
            if !auras::set_owned_stacks(world, source, target, aura.spell_id, aura.stacks) {
                return Err(format!("WCL_OPENING_AURA_NOT_APPLIED:{}", aura.spell_id));
            }
            if let Some(time_ms) = aura.remove_ms {
                let instance_id = world
                    .get::<auras::ActiveAuras>(target)
                    .and_then(|active| {
                        active
                            .iter()
                            .find(|a| a.caster == source && a.spell_id == aura.spell_id)
                    })
                    .ok_or_else(|| format!("WCL_OPENING_AURA_NOT_APPLIED:{}", aura.spell_id))?
                    .instance_id;
                removals.push(OpeningRemoval {
                    time_ms,
                    source,
                    target,
                    spell_id: aura.spell_id,
                    instance_id,
                    refreshed: false,
                });
            }
        }
        removals.sort_by_key(|removal| removal.time_ms);
        Ok(removals)
    }
}

fn apply_opening_bulwark(world: &mut World, application: &crate::auras::AuraApplication) {
    use crate::auras::ActiveAuras;
    use crate::spell_info::SpellStore;
    if world
        .get::<ActiveAuras>(application.target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == 432607 && aura.caster == application.caster)
        })
    {
        return;
    }
    // INFERRED_WCL: remaining opening capacity is not recorded. Reconstruct a
    // fresh grant, as for other opening shields, without changing periodic grants.
    // DB2 70448 432496s2 / 10 is the initial percentage of target max health.
    let store = world.resource::<SpellStore>();
    let percent = store
        .get(432496)
        .expect("Holy Bulwark source")
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Holy Bulwark initial amount")
        .base_points;
    let mut shield = store.get(432607).expect("Holy Bulwark absorb").clone();
    let health = world
        .get::<Health>(application.target)
        .expect("Holy Bulwark target health")
        .max;
    let versatility = crate::spell_cast::effects::versatility_multiplier(world, application.caster);
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Holy Bulwark absorb effect")
        .base_points = health * percent / 1000.0 * versatility;
    crate::auras::apply_aura_with_spell(world, application, &shield);
}

#[derive(Deserialize)]
pub(super) struct TimedBloodlust {
    pub time_ms: u64,
    pub slot: usize,
}

#[derive(Deserialize)]
pub(super) struct RecordedHit {
    pub time_ms: u64,
    slot: usize,
    source_actor: i64,
    target_actor: i64,
    spell_id: u32,
    school: u32,
    defense: Option<u32>,
    hit_type: u8,
    periodic: bool,
    #[serde(default)]
    deferred: bool,
    amount: u64,
    absorbed: u64,
    max_hp: u64,
    health_after: Option<u64>,
}

impl Trace {
    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let trace: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        if trace.provenance.is_empty()
            || trace.hits.is_empty()
            || trace.hits.windows(2).any(|p| p[0].time_ms > p[1].time_ms)
            || trace
                .bloodlust
                .windows(2)
                .any(|p| p[0].time_ms > p[1].time_ms)
            || trace.bloodlust.iter().any(|buff| buff.slot >= 5)
            || trace
                .replayed_casts
                .windows(2)
                .any(|p| p[0].time_ms > p[1].time_ms)
            || trace
                .replayed_casts
                .iter()
                .any(|cast| cast.slot >= 5 || cast.evidence.is_empty())
        {
            return Err("WCL_INTAKE_INVALID_HEADER_OR_ORDER".into());
        }
        for hit in &trace.hits {
            hit.validate()?;
        }
        Ok(trace)
    }
}

#[cfg(test)]
mod tests {
    use super::RecordedHit;

    #[test]
    fn critical_wcl_hits_accept_recorded_post_mitigation_damage() {
        let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":1,"target_actor":2,"spell_id":1,"school":1,"defense":2,"hit_type":2,"periodic":false,"amount":150,"absorbed":50,"max_hp":1000,"health_after":800}"#).unwrap();
        assert!(hit.validate().is_ok());
    }

    #[test]
    fn blocked_wcl_hits_accept_recorded_post_mitigation_damage() {
        let hit: RecordedHit = serde_json::from_str(r#"{"time_ms":0,"slot":0,"source_actor":1,"target_actor":2,"spell_id":1,"school":1,"defense":2,"hit_type":4,"periodic":false,"amount":150,"absorbed":50,"max_hp":1000,"health_after":800}"#).unwrap();
        assert!(hit.validate().is_ok());
    }
}

impl RecordedHit {
    fn validate(&self) -> Result<(), String> {
        if self.slot >= 5
            || self.school == 0
            || self.spell_id == 0
            || self.max_hp == 0
            || (self.source_actor == self.target_actor
                && !(self.deferred && matches!(self.spell_id, 124255 | 361029)))
            || (self.deferred && self.source_actor != self.target_actor)
        {
            return Err("WCL_INTAKE_INVALID_HIT".into());
        }
        match (self.hit_type, self.amount + self.absorbed) {
            (1 | 2 | 4, amount) if amount > 0 => Ok(()),
            (0 | 7 | 8 | 9 | 10, 0) => Ok(()),
            _ => Err("WCL_INTAKE_UNSUPPORTED_AMOUNT_OR_HIT_TYPE".into()),
        }
    }

    pub fn deliver(
        &self,
        world: &mut World,
        enemy: Entity,
        slots: &[Entity; 5],
    ) -> Result<Delivery, String> {
        let pressure = self.amount + self.absorbed;
        let target = slots[self.slot];
        let health = world
            .get::<Health>(target)
            .ok_or("WCL_INTAKE_TARGET_HEALTH_MISSING")?;
        if health.current <= 0.0 {
            return Ok(Delivery::TargetDead);
        }
        let native_max = health.max;
        let amount = pressure as f32 / self.max_hp as f32 * native_max;
        let spell_id = if matches!(self.spell_id, 1 | 3) {
            None
        } else {
            let spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(self.spell_id)
                .ok_or_else(|| format!("WCL_INTAKE_MISSING_SPELL:{}", self.spell_id))?;
            if spell.school_mask() != self.school
                || spell.categories.as_ref().map(|c| c.defense_type) != self.defense
            {
                return Err(format!(
                    "WCL_INTAKE_SPELL_METADATA_CHANGED:{}",
                    self.spell_id
                ));
            }
            Some(self.spell_id)
        };
        if pressure == 0 {
            let outcome = match self.hit_type {
                0 => shared::protocol::MissKind::Miss,
                7 => shared::protocol::MissKind::Dodge,
                8 => shared::protocol::MissKind::Parry,
                9 => shared::protocol::MissKind::Deflect,
                10 => shared::protocol::MissKind::Immune,
                _ => return Err("WCL_INTAKE_UNSUPPORTED_AVOIDANCE".into()),
            };
            crate::combat::replay_avoided_hit(
                world,
                enemy,
                target,
                spell_id,
                self.school,
                self.periodic,
                outcome,
            )?;
            return Ok(Delivery::Avoided);
        }
        let receipt = crate::combat::replay_post_mitigation_hit(
            world,
            enemy,
            target,
            spell_id,
            self.school,
            self.periodic,
            self.hit_type == 2,
            amount,
        )?;
        let after = world
            .get::<Health>(target)
            .ok_or("WCL_INTAKE_TARGET_HEALTH_MISSING")?
            .current;
        Ok(Delivery::Applied(
            json!({"time_ms":self.time_ms,"slot":self.slot,"spell_id":self.spell_id,
            "wcl_hp_pct":self.health_after.map(|hp| 100.0 * hp as f64 / self.max_hp as f64),
            "native_hp_pct":100.0 * f64::from(after / native_max),
            "wcl_loss_pct":100.0 * self.amount as f64 / self.max_hp as f64,
            "wcl_absorbed_pct":100.0 * self.absorbed as f64 / self.max_hp as f64,
            "native_loss_pct":100.0 * f64::from(receipt.damage_hp / native_max),
            "native_absorbed_pct":100.0 * f64::from(receipt.absorbed_hp / native_max),
            "deferred":self.deferred}),
        ))
    }
}
