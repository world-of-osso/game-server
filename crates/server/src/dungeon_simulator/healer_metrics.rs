//! Observation-only native frame receipts, shared by paired before/after runs.
use bevy::prelude::*;
use serde::Serialize;
use shared::components::{CombatStatus, Health, Mana};
use shared::protocol::{CombatLogEvent, CombatLogKind};
use std::collections::{BTreeMap, BTreeSet};

use super::graph::Graph;
use crate::auras::{ActiveAuras, handlers};
use crate::spell_cast::test_harness::ObservedCombatEvents;

struct PackState {
    living: BTreeSet<Entity>,
    engaged_at: Option<f64>,
}

#[derive(Serialize)]
struct PullEnd {
    pack: u32,
    engaged_at: f64,
    last_native_death_at: f64,
    healer_mana: Option<f32>,
    healer_max_mana: Option<f32>,
    healer_alive: Option<bool>,
}

pub(super) struct Recorder {
    healer: Entity,
    tank: Entity,
    members: Vec<Entity>,
    actor_packs: BTreeMap<Entity, u32>,
    packs: BTreeMap<u32, PackState>,
    event_cursor: usize,
    last_frame: f64,
    combat_seconds: f64,
    alive_tank_combat_seconds: f64,
    tank_hot_seconds: BTreeMap<u32, f64>,
    any_hot_seconds: f64,
    party_deaths: u64,
    raw_healing: u64,
    overhealing: u64,
    idle_gcd_seconds: f64,
    idle_gcds: f64,
    pull_ends: Vec<PullEnd>,
    frames: Vec<super::healer_intake::Frame>,
    source_deaths: Vec<(u32, u32)>,
    map_id: u32,
    difficulty_id: u32,
    level: u8,
}

pub(super) fn start(
    world: &mut World,
    graph: &Graph,
    party: &crate::followers::simulator::Party,
    actors: &BTreeMap<u32, Entity>,
) -> Recorder {
    let healer = *party
        .members
        .iter()
        .find(|unit| {
            world
                .get::<crate::followers::Follower>(**unit)
                .is_some_and(|follower| follower.profile.role == shared::group::GroupRole::Healer)
        })
        .expect("native simulator party includes a healer");
    let members: Vec<_> = party.members.iter().copied().chain([party.owner]).collect();
    let sources = graph
        .packs
        .iter()
        .flat_map(|pack| {
            pack.spawns
                .iter()
                .map(|spawn| (actors[&spawn.guid], (spawn.guid, pack.id)))
        })
        .collect();
    let start_tick = super::healer_intake::tick(world);
    world.insert_resource(super::healer_intake::Capture {
        members: members.clone(),
        sources,
        start_tick,
        ..Default::default()
    });
    let actor_packs = graph
        .packs
        .iter()
        .flat_map(|pack| {
            pack.spawns
                .iter()
                .map(|spawn| (actors[&spawn.guid], pack.id))
        })
        .collect();
    let packs = graph
        .packs
        .iter()
        .map(|pack| {
            let living = pack
                .spawns
                .iter()
                .map(|spawn| actors[&spawn.guid])
                .collect();
            (
                pack.id,
                PackState {
                    living,
                    engaged_at: None,
                },
            )
        })
        .collect();
    crate::auras::observations::start(world, healer);
    let level = world.get::<shared::components::Level>(healer).unwrap().0;
    let initial_frame = super::healer_intake::Frame {
        tick: 0,
        positions: members
            .iter()
            .map(|&unit| {
                world
                    .get::<shared::components::Position>(unit)
                    .map(|at| [at.x, at.y, at.z])
            })
            .collect(),
        in_combat: members.iter().any(|&unit| {
            world
                .get::<CombatStatus>(unit)
                .is_some_and(|status| status.0)
        }),
    };
    Recorder {
        healer,
        tank: party.tank,
        members,
        actor_packs,
        packs,
        event_cursor: world.resource::<ObservedCombatEvents>().0.len(),
        last_frame: world.resource::<Time<Virtual>>().elapsed_secs_f64(),
        combat_seconds: 0.0,
        alive_tank_combat_seconds: 0.0,
        tank_hot_seconds: available_periodic_heals(world, healer),
        any_hot_seconds: 0.0,
        party_deaths: 0,
        raw_healing: 0,
        overhealing: 0,
        idle_gcd_seconds: 0.0,
        idle_gcds: 0.0,
        pull_ends: Vec::new(),
        frames: vec![initial_frame],
        source_deaths: Vec::new(),
        map_id: graph.map_id,
        difficulty_id: graph.difficulty_id(),
        level,
    }
}

fn available_periodic_heals(world: &World, healer: Entity) -> BTreeMap<u32, f64> {
    let store = world.resource::<crate::spell_info::SpellStore>();
    crate::followers::simulator::available_kit_spells(world, healer)
        .into_iter()
        .filter(|spell| {
            store.get(*spell).is_some_and(|spell| {
                spell.effects.iter().any(|effect| {
                    crate::auras::is_aura_effect(effect)
                        && effect.aura == handlers::PERIODIC_HEAL
                        && effect.aura_period_ms > 0
                })
            })
        })
        .map(|spell| (spell, 0.0))
        .collect()
}

impl Recorder {
    /// Called once after each actual app update, never by the route controller's decisions.
    pub fn observe_frame(&mut self, world: &World) {
        let now = world.resource::<Time<Virtual>>().elapsed_secs_f64();
        let dt = now - self.last_frame;
        self.last_frame = now;
        let capture = world.resource::<super::healer_intake::Capture>();
        self.frames.push(super::healer_intake::Frame {
            tick: super::healer_intake::tick(world).saturating_sub(capture.start_tick),
            positions: self
                .members
                .iter()
                .map(|&unit| {
                    world
                        .get::<shared::components::Position>(unit)
                        .map(|at| [at.x, at.y, at.z])
                })
                .collect(),
            in_combat: self.members.iter().any(|&unit| {
                world
                    .get::<CombatStatus>(unit)
                    .is_some_and(|status| status.0)
            }),
        });
        self.observe_combat_coverage(world, dt);
        let events = &world.resource::<ObservedCombatEvents>().0;
        for event in &events[self.event_cursor..] {
            self.observe_event(world, event, now);
        }
        self.event_cursor = events.len();
    }

    fn observe_combat_coverage(&mut self, world: &World, dt: f64) {
        let in_combat = self.members.iter().any(|unit| {
            world
                .get::<CombatStatus>(*unit)
                .is_some_and(|status| status.0)
        });
        if !in_combat {
            return;
        }
        self.combat_seconds += dt;
        let alive = world
            .get::<Health>(self.healer)
            .is_some_and(|health| health.current > 0.0);
        if alive
            && let Some((false, duration)) = crate::spell_cast::healer_idle_gcd(world, self.healer)
        {
            self.idle_gcd_seconds += dt;
            self.idle_gcds += dt / duration;
        }
        if !world
            .get::<Health>(self.tank)
            .is_some_and(|health| health.current > 0.0)
        {
            return;
        }
        self.alive_tank_combat_seconds += dt;
        let now = crate::auras::aura_now(world);
        let spells: BTreeSet<_> = world
            .get::<ActiveAuras>(self.tank)
            .into_iter()
            .flat_map(|auras| auras.iter())
            .filter(|aura| {
                aura.caster == self.healer && aura.expires_at.is_none_or(|expiry| expiry > now)
            })
            .filter(|aura| {
                aura.effects.iter().any(|effect| {
                    effect.aura_type == handlers::PERIODIC_HEAL && effect.period_ms > 0
                })
            })
            .map(|aura| aura.spell_id)
            .collect();
        for spell in &spells {
            *self.tank_hot_seconds.entry(*spell).or_default() += dt;
        }
        if !spells.is_empty() {
            self.any_hot_seconds += dt;
        }
    }

    fn observe_event(&mut self, world: &World, event: &CombatLogEvent, now: f64) {
        let source = event.source.and_then(Entity::try_from_bits);
        let target = event.target.and_then(Entity::try_from_bits);
        match event.kind {
            CombatLogKind::Heal
                if source == Some(self.healer)
                    && target.is_some_and(|unit| self.members.contains(&unit)) =>
            {
                let raw = event.amount.max(0) as u64;
                self.raw_healing += raw;
                self.overhealing += (event.overflow.max(0) as u64).min(raw);
            }
            CombatLogKind::Damage => self.observe_pack_engagement(source, target, now),
            CombatLogKind::Death => {
                if let Some(target) = target {
                    self.party_deaths += u64::from(self.members.contains(&target));
                    self.observe_pack_death(world, target, now);
                }
            }
            _ => {}
        }
    }

    fn observe_pack_engagement(
        &mut self,
        source: Option<Entity>,
        target: Option<Entity>,
        now: f64,
    ) {
        let party_source = source.is_some_and(|source| self.members.contains(&source));
        let party_target = target.is_some_and(|target| self.members.contains(&target));
        let enemy = if party_source {
            target
        } else if party_target {
            source
        } else {
            None
        };
        if let Some(pack) = enemy.and_then(|enemy| self.actor_packs.get(&enemy)) {
            self.packs
                .get_mut(pack)
                .expect("observed source pack")
                .engaged_at
                .get_or_insert(now);
        }
    }

    fn observe_pack_death(&mut self, world: &World, target: Entity, now: f64) {
        let capture = world.resource::<super::healer_intake::Capture>();
        if let Some(&(guid, _)) = capture.sources.get(&target) {
            self.source_deaths.push((
                super::healer_intake::tick(world).saturating_sub(capture.start_tick),
                guid,
            ));
        }
        let Some(&pack_id) = self.actor_packs.get(&target) else {
            return;
        };
        let pack = self.packs.get_mut(&pack_id).expect("observed source pack");
        if !pack.living.remove(&target) || !pack.living.is_empty() {
            return;
        }
        let Some(engaged_at) = pack.engaged_at.take() else {
            return;
        };
        let mana = world.get::<Mana>(self.healer);
        self.pull_ends.push(PullEnd {
            pack: pack_id,
            engaged_at,
            last_native_death_at: now,
            healer_mana: mana.map(|mana| mana.current),
            healer_max_mana: mana.map(|mana| mana.max),
            healer_alive: world
                .get::<Health>(self.healer)
                .map(|health| health.current > 0.0),
        });
    }

    pub fn report(&self, world: &World) -> serde_json::Value {
        let uptime: BTreeMap<_, _> = self
            .tank_hot_seconds
            .iter()
            .map(|(spell, seconds)| {
                let fraction = (self.alive_tank_combat_seconds > 0.0)
                    .then(|| seconds / self.alive_tank_combat_seconds);
                (*spell, fraction)
            })
            .collect();
        serde_json::json!({
            "recorded_intake": {
                "schema": 1,
                "map_id": self.map_id,
                "difficulty_id": self.difficulty_id,
                "level": self.level,
                "source_deaths": self.source_deaths,
                "end_tick": self.frames.last().map_or(0, |frame| frame.tick),
                "hits": world.resource::<super::healer_intake::Capture>().hits,
                "frames": self.frames,
                "pulls": self.pull_ends,
                "gaps": world.resource::<super::healer_intake::Capture>().gaps,
            },
            "dungeon_healing": {
                "party_deaths": self.party_deaths,
                "raw_healing": self.raw_healing,
                "effective_healing": self.raw_healing - self.overhealing,
                "overhealing": self.overhealing,
                "overheal_pct": (self.raw_healing > 0).then(|| 100.0 * self.overhealing as f64 / self.raw_healing as f64),
                "effective_hps": (self.combat_seconds > 0.0).then(|| (self.raw_healing - self.overhealing) as f64 / self.combat_seconds),
                "combat_seconds": self.combat_seconds,
                "mana_at_end": world.get::<Mana>(self.healer).map(|mana| mana.current),
                "max_mana": world.get::<Mana>(self.healer).map(|mana| mana.max),
                "idle_gcd_seconds": self.idle_gcd_seconds,
                "idle_gcds": self.idle_gcds,
                "idle_definition": "alive, in party combat, no active cast/channel or native GCD; equivalents use profile heal native hasted GCD"
            },
            "sample_period_seconds": world.resource::<Time<Fixed>>().timestep().as_secs_f64(),
            "healer": self.healer.to_bits(), "tank": self.tank.to_bits(),
            "pull_ends": self.pull_ends, "party_deaths": self.party_deaths,
            "combat_seconds": self.combat_seconds,
            "alive_tank_combat_seconds": self.alive_tank_combat_seconds,
            "tank_hot_seconds": self.tank_hot_seconds, "tank_hot_uptime": uptime,
            "any_hot_seconds": self.any_hot_seconds,
            "proc_usage": crate::auras::observations::snapshot(world),
        })
    }
}
