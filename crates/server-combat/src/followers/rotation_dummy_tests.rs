//! Native follower stats and proc observation fixtures.
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, aura_now};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{Health, ThreatTable};
use std::collections::BTreeMap;

const DUMMY_HEALTH: f32 = 1_000_000_000.0;

pub(super) fn prepare_dummies(h: &mut Harness, count: usize) -> Vec<Entity> {
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
        63 => &[(48108, &[11366, 1254851])],
        253 => &[], // Native1264934 Frenzy is a permanent pet-speed passive, not a proc.
        1467 => &[(359618, &[356995, 357211])],
        _ => unreachable!("only the four assigned follower specializations"),
    }
}

#[derive(Clone)]
struct ProcSnapshot {
    instance: u32,
    stacks: u8,
    charges: u8,
    deadline: f64,
}

impl ProcSnapshot {
    fn uses(&self) -> usize {
        usize::from(self.stacks.max(self.charges).max(1))
    }
}

fn proc_snapshots(world: &World, caster: Entity, spec: u32) -> BTreeMap<u32, ProcSnapshot> {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return BTreeMap::new();
    };
    auras
        .iter()
        .filter_map(|aura| {
            let measured = proc_consumers(spec)
                .iter()
                .any(|(id, _)| *id == aura.spell_id);
            measured.then_some((
                aura.spell_id,
                ProcSnapshot {
                    instance: aura.instance_id,
                    stacks: aura.stacks,
                    charges: aura.charges,
                    deadline: aura.expires_at.unwrap_or(f64::INFINITY),
                },
            ))
        })
        .collect()
}

struct ProcObserver {
    spec: u32,
    caster: Entity,
    consumed: BTreeMap<u32, usize>,
    expired: BTreeMap<u32, usize>,
}

impl ProcObserver {
    fn new(spec: u32, caster: Entity) -> Self {
        Self {
            spec,
            caster,
            consumed: BTreeMap::new(),
            expired: BTreeMap::new(),
        }
    }

    fn observe(
        &mut self,
        previous: &BTreeMap<u32, ProcSnapshot>,
        current: &BTreeMap<u32, ProcSnapshot>,
        now: f64,
        receipts: &[shared::protocol::SpellGo],
    ) {
        for (&aura, before) in previous {
            let remaining = current
                .get(&aura)
                .filter(|after| after.instance == before.instance)
                .map_or(0, ProcSnapshot::uses);
            let lost = before.uses().saturating_sub(remaining);
            if lost == 0 {
                continue;
            }
            let consumers = proc_consumers(self.spec)
                .iter()
                .find(|(id, _)| *id == aura)
                .unwrap()
                .1;
            let spent = receipts.iter().any(|event| {
                event.caster == self.caster.to_bits() && consumers.contains(&event.spell_id)
            });
            if spent && now < before.deadline {
                *self.consumed.entry(aura).or_default() += lost;
            } else if now >= before.deadline {
                *self.expired.entry(aura).or_default() += lost;
            }
        }
    }
}

pub(super) fn prepare_follower(entry: u32, spec: u32, level: u8) -> (Harness, Entity) {
    if spec != 253 {
        return at_level(entry, level, |app| {
            app.add_plugins(crate::stats::StatsPlugin);
        });
    }
    let data = crate::spell_cast::test_harness::prepare_class_world().world_data();
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let abilities = crate::pet_control::load_pet_levelup_spells(&source).unwrap();
    at_level(entry, level, |app| {
        app.insert_resource(data)
            .insert_resource(abilities)
            .add_plugins((
                crate::stats::StatsPlugin,
                crate::hunter_pet::HunterPetPlugin,
                crate::pet_control::PetControlPlugin,
                crate::creature_ai::CreatureAiPlugin,
            ));
    })
}

#[test]
fn rotation_dummy_native_stat_buff_recomputes_all_four_follower_sheets() {
    for (entry, spec) in [(209059, 63), (209053, 262), (209065, 253), (215240, 1467)] {
        let (mut h, caster) = prepare_follower(entry, spec, 90);
        let before = h
            .app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .primary
            .intellect;
        assert!(before > 0.0);
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, 1459, Some(caster), true)
            .unwrap();
        h.advance(3);
        assert!(crate::auras::has_aura(h.app.world(), caster, 1459));
        let after = h
            .app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .primary
            .intellect;
        assert!(
            after > before,
            "spec{spec}: native Arcane Intellect must reach the actual production-derived sheet ({before}→{after})"
        );
    }
}

pub(super) fn snapshot_stats(h: &Harness, caster: Entity) -> serde_json::Value {
    let stats = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(caster)
        .unwrap();
    serde_json::json!({
        "primary": stats.primary,
        "spell_power": stats.spell_power,
        "attack_power": stats.attack_power,
        "ranged_attack_power": stats.ranged_attack_power,
        "crit_pct": stats.crit_pct,
        "haste_pct": stats.haste_pct,
        "mastery_pct": stats.mastery_pct,
        "mastery_points": stats.mastery_points,
        "versatility_pct": stats.versatility_pct,
        "max_health": stats.max_health,
        "max_mana": stats.max_mana,
    })
}

#[test]
fn rotation_metrics_1467_observes_one_use_when_two_real_stacks_become_one() {
    use super::kit_proc_fidelity_tests::{complete_cast, prepare};
    let (mut h, caster) = prepare(215240);
    let target = h.npc;
    complete_cast(&mut h, caster, 375087, caster);
    complete_cast(&mut h, caster, 362969, target);
    complete_cast(&mut h, caster, 362969, target);
    let mut observer = ProcObserver::new(1467, caster);
    let before = proc_snapshots(h.app.world(), caster, 1467);
    let start = h.inbox.go.len();
    complete_cast(&mut h, caster, 356995, target);
    let after = proc_snapshots(h.app.world(), caster, 1467);
    observer.observe(
        &before,
        &after,
        aura_now(h.app.world()),
        &h.inbox.go[start..],
    );
    assert_eq!(
        observer.consumed.get(&359618).copied().unwrap_or(0),
        1,
        "a real2→1 stack spend must appear in the consumption numerator"
    );
    assert!(observer.expired.is_empty());
}
