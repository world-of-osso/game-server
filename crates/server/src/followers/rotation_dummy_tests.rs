//! Headless 120s max-level decision/execution measurements, not a damage oracle.
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, aura_now};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use serde::Serialize;
use shared::components::{Health, ThreatTable};
use shared::protocol::CombatLogKind;
use std::collections::BTreeMap;

const DURATION_SECONDS: f64 = 120.0;
const DUMMY_HEALTH: f32 = 1_000_000_000.0;

#[derive(Default, Serialize)]
struct ProcMeasurement {
    gained: usize,
    consumed: usize,
    expired: usize,
    consumed_per_gained: Option<f64>,
}

#[derive(Serialize)]
struct DummyMeasurement {
    spec: u32,
    level: u8,
    targets: usize,
    elapsed_seconds: f64,
    damage: i64,
    actions: BTreeMap<u32, usize>,
    dot_target_seconds: BTreeMap<u32, f64>,
    dot_uptime_fraction: BTreeMap<u32, f64>,
    procs: BTreeMap<u32, ProcMeasurement>,
    pet_present: bool,
}

fn prepare_dummies(h: &mut Harness, count: usize) -> Vec<Entity> {
    let mut targets = vec![h.npc];
    targets.extend((1..count).map(|index| spawn_npc(&mut h.app, index as f32)));
    for &target in &targets {
        h.app.world_mut().entity_mut(target).insert(Health {
            current: DUMMY_HEALTH,
            max: DUMMY_HEALTH,
        });
        let mut threat = ThreatTable::default();
        threat.add_threat(h.caster.to_bits(), 1.0);
        h.app.world_mut().entity_mut(target).insert(threat);
    }
    targets
}

fn proc_consumers(spec: u32) -> &'static [(u32, &'static [u32])] {
    match spec {
        262 => &[(77762, &[51505])],
        63 => &[(48108, &[11366, 1254851]), (48107, &[108853, 319836])],
        253 => &[(272790, &[])], // Frenzy is maintained, never spent as a charge.
        1467 => &[(359618, &[356995, 357211])],
        _ => unreachable!("only the four assigned follower specializations"),
    }
}

fn proc_deadlines(world: &World, caster: Entity, spec: u32) -> BTreeMap<u32, f64> {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return BTreeMap::new();
    };
    auras
        .iter()
        .filter_map(|aura| {
            let measured = proc_consumers(spec)
                .iter()
                .any(|(id, _)| *id == aura.spell_id);
            measured.then_some((aura.spell_id, aura.expires_at.unwrap_or(f64::INFINITY)))
        })
        .collect()
}

fn sample_dots(
    h: &Harness,
    caster: Entity,
    targets: &[Entity],
    seconds: f64,
    totals: &mut BTreeMap<u32, f64>,
) {
    for &target in targets {
        if let Some(auras) = h.app.world().get::<ActiveAuras>(target) {
            for aura in auras.iter().filter(|aura| aura.caster == caster) {
                let periodic_damage = aura
                    .effects
                    .iter()
                    .any(|effect| effect.aura_type == crate::auras::PERIODIC_DAMAGE_AURA);
                if periodic_damage {
                    *totals.entry(aura.spell_id).or_default() += seconds;
                }
            }
        }
    }
}

fn measure_dummy(entry: u32, spec: u32, count: usize, level: u8, seconds: f64) {
    let (mut h, caster) = at_level(entry, level, |_| {});
    let targets = prepare_dummies(&mut h, count);
    let begin = aura_now(h.app.world());
    let first_log = h.inbox.log.len();
    let mut dots = BTreeMap::new();
    let mut consumed = BTreeMap::<u32, usize>::new();
    let mut expired = BTreeMap::<u32, usize>::new();
    let mut updates = 0;
    while aura_now(h.app.world()) - begin < seconds {
        let previous_time = aura_now(h.app.world());
        let previous_procs = proc_deadlines(h.app.world(), caster, spec);
        let first_go = h.inbox.go.len();
        super::update_ai(h.app.world_mut());
        h.advance(1);
        let now = aura_now(h.app.world());
        let current = proc_deadlines(h.app.world(), caster, spec);
        for (aura, deadline) in previous_procs {
            if current.contains_key(&aura) {
                continue;
            }
            let consumers = proc_consumers(spec)
                .iter()
                .find(|(id, _)| *id == aura)
                .unwrap()
                .1;
            let spent = h.inbox.go[first_go..].iter().any(|event| {
                event.caster == caster.to_bits() && consumers.contains(&event.spell_id)
            });
            if spent && now < deadline {
                *consumed.entry(aura).or_default() += 1;
            } else if now >= deadline {
                *expired.entry(aura).or_default() += 1;
            }
        }
        sample_dots(&h, caster, &targets, now - previous_time, &mut dots);
        updates += 1;
        assert!(updates <= 3000, "120s deterministic clock did not advance");
        assert!(targets.iter().all(|target| h.health(*target) > 0.0));
    }
    let elapsed = aura_now(h.app.world()) - begin;
    let pet = crate::hunter_pet::summoned_pet(h.app.world(), caster);
    let sources: Vec<_> = [Some(caster.to_bits()), pet.map(Entity::to_bits)]
        .into_iter()
        .flatten()
        .collect();
    let events = &h.inbox.log[first_log..];
    let damage = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source.is_some_and(|source| sources.contains(&source))
        })
        .map(|event| i64::from(event.amount))
        .sum();
    let mut actions = BTreeMap::new();
    for event in events.iter().filter(|event| {
        event.kind == CombatLogKind::CastSuccess && event.source == Some(caster.to_bits())
    }) {
        if let Some(spell) = event.spell_id {
            *actions.entry(spell).or_default() += 1;
        }
    }
    let procs = proc_consumers(spec)
        .iter()
        .map(|(aura, _)| {
            let gained = events
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::AuraApplied
                        && event.spell_id == Some(*aura)
                        && event.source.is_some_and(|source| sources.contains(&source))
                })
                .count();
            let consumed = consumed.get(aura).copied().unwrap_or(0);
            (
                *aura,
                ProcMeasurement {
                    gained,
                    consumed,
                    expired: expired.get(aura).copied().unwrap_or(0),
                    consumed_per_gained: (gained > 0).then_some(consumed as f64 / gained as f64),
                },
            )
        })
        .collect();
    let fractions = dots
        .iter()
        .map(|(&spell, &seconds)| (spell, seconds / elapsed / count as f64))
        .collect();
    let report = DummyMeasurement {
        spec,
        level,
        targets: count,
        elapsed_seconds: elapsed,
        damage,
        actions,
        dot_target_seconds: dots,
        dot_uptime_fraction: fractions,
        procs,
        pet_present: pet.is_some(),
    };
    let json = serde_json::to_string_pretty(&report).unwrap();
    println!("ROTATION_DUMMY {json}");
    if let Some(directory) = std::env::var_os("ROTATION_FIDELITY_OUTPUT") {
        let path = std::path::PathBuf::from(directory)
            .join(format!("{spec}-level{level}-{count}target.json"));
        std::fs::write(&path, json + "\n")
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    }
    assert!(damage > 0, "native follower produced no damage");
}

#[test]
fn rotation_dummy_262_single_target_120s() {
    measure_dummy(209053, 262, 1, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_262_three_targets_120s() {
    measure_dummy(209053, 262, 3, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_262_level32_smoke() {
    measure_dummy(209053, 262, 1, 32, 10.0);
}
#[test]
fn rotation_dummy_63_single_target_120s() {
    measure_dummy(209059, 63, 1, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_63_three_targets_120s() {
    measure_dummy(209059, 63, 3, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_63_level32_smoke() {
    measure_dummy(209059, 63, 1, 32, 10.0);
}
#[test]
fn rotation_dummy_253_single_target_120s() {
    measure_dummy(209065, 253, 1, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_253_three_targets_120s() {
    measure_dummy(209065, 253, 3, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_253_level32_smoke() {
    measure_dummy(209065, 253, 1, 32, 10.0);
}
#[test]
fn rotation_dummy_1467_single_target_120s() {
    measure_dummy(215240, 1467, 1, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_1467_three_targets_120s() {
    measure_dummy(215240, 1467, 3, 90, DURATION_SECONDS);
}
#[test]
fn rotation_dummy_1467_level32_smoke() {
    measure_dummy(215240, 1467, 1, 32, 10.0);
}
