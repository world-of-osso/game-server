//! Matched native training-dummy receipts with the production derived-stats plugin.
use super::{kit_tests::at_level, update_ai};
use crate::auras::ActiveAuras;
use crate::spell_cast::powers::PowerPools;
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{DerivedStats, Health, ThreatTable};
use shared::protocol::CombatLogKind;
use std::collections::{BTreeMap, BTreeSet};

const SECONDS: usize = 60;
const FRAMES: usize = SECONDS * 20;
const SEED: u64 = 1;

#[derive(Default)]
struct Samples {
    capped_frames: BTreeMap<String, usize>,
    actor_aura_frames: BTreeMap<u32, usize>,
    owned_target_aura_frames: Vec<BTreeMap<u32, usize>>,
    owned_actors: BTreeSet<u64>,
}

fn prepare_targets(h: &mut Harness, count: usize) -> Vec<Entity> {
    (0..count)
        .map(|index| {
            let target = if index == 0 {
                h.npc
            } else {
                spawn_npc(&mut h.app, 1.0)
            };
            h.app.world_mut().entity_mut(target).insert(Health {
                current: 100_000_000.0,
                max: 100_000_000.0,
            });
            let mut threat = ThreatTable::default();
            threat.add_threat(h.caster.to_bits(), 1.0);
            h.app
                .world_mut()
                .entity_mut(target)
                .insert(threat)
                .remove::<(crate::creature_ai::CreatureState, crate::combat::AutoAttack)>();
            target
        })
        .collect()
}

fn count_auras(
    world: &World,
    target: Entity,
    owner: Option<Entity>,
    counts: &mut BTreeMap<u32, usize>,
) {
    if let Some(auras) = world.get::<ActiveAuras>(target) {
        for aura in auras
            .iter()
            .filter(|aura| owner.is_none_or(|owner| aura.caster == owner))
        {
            *counts.entry(aura.spell_id).or_default() += 1;
        }
    }
}

fn sample_actor(h: &mut Harness, unit: Entity, samples: &mut Samples) {
    if let Some(pools) = h.app.world().get::<PowerPools>(unit) {
        for pool in pools.pools.iter().filter(|pool| pool.current >= pool.max) {
            *samples
                .capped_frames
                .entry(format!("{:?}", pool.power))
                .or_default() += 1;
        }
    }
    count_auras(h.app.world(), unit, None, &mut samples.actor_aura_frames);
    let world = h.app.world_mut();
    for (entity, summon) in world.query::<(Entity, &SummonedCreature)>().iter(world) {
        if summon.owner == unit {
            samples.owned_actors.insert(entity.to_bits());
        }
    }
    for (entity, pet) in world
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(world)
    {
        if pet.owner == unit {
            samples.owned_actors.insert(entity.to_bits());
        }
    }
}

fn advance_samples(h: &mut Harness, unit: Entity, targets: &[Entity]) -> Samples {
    let mut samples = Samples {
        owned_target_aura_frames: vec![BTreeMap::new(); targets.len()],
        ..Default::default()
    };
    for _ in 0..FRAMES {
        update_ai(h.app.world_mut());
        h.advance(1);
        sample_actor(h, unit, &mut samples);
        for (index, target) in targets.iter().enumerate() {
            count_auras(
                h.app.world(),
                *target,
                Some(unit),
                &mut samples.owned_target_aura_frames[index],
            );
        }
    }
    samples
}

fn collect_report(h: &Harness, unit: Entity, start: usize, samples: Samples) -> serde_json::Value {
    let mut casts = BTreeMap::<u32, usize>::new();
    let mut actor_damage = 0_i64;
    let mut owned_actor_damage = 0_i64;
    let mut energize_attempted = BTreeMap::<u32, i64>::new();
    let mut energize_overflow = BTreeMap::<u32, i64>::new();
    for event in &h.inbox.log[start..] {
        if event.source == Some(unit.to_bits()) {
            if event.kind == CombatLogKind::CastSuccess {
                *casts.entry(event.spell_id.unwrap()).or_default() += 1;
            } else if event.kind == CombatLogKind::Damage {
                actor_damage += i64::from(event.amount);
            }
        } else if event
            .source
            .is_some_and(|source| samples.owned_actors.contains(&source))
            && event.kind == CombatLogKind::Damage
        {
            owned_actor_damage += i64::from(event.amount);
        }
        if event.target == Some(unit.to_bits()) && event.kind == CombatLogKind::Energize {
            let spell = event.spell_id.unwrap();
            *energize_attempted.entry(spell).or_default() += i64::from(event.amount);
            *energize_overflow.entry(spell).or_default() += i64::from(event.overflow);
        }
    }
    serde_json::json!({
        "casts":casts,"actor_damage":actor_damage,"owned_actor_damage":owned_actor_damage,
        "total_damage":actor_damage+owned_actor_damage,"capped_frames":samples.capped_frames,
        "actor_aura_frames":samples.actor_aura_frames,"owned_target_aura_frames":samples.owned_target_aura_frames,
        "logged_energize_attempted_raw_by_spell":energize_attempted,
        "logged_energize_overflow_raw_by_spell":energize_overflow,
        "observed_owned_actors":samples.owned_actors.len()
    })
}

fn install_pet_runtime(app: &mut App, directory: &std::path::Path) {
    let source = crate::spell_cast::test_harness::prepare_class_world();
    let data = source.world_data();
    let connection = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let pets = crate::pet_control::load_pet_levelup_spells(&connection).unwrap();
    let database = redb::Database::create(directory.join("npc-pets.redb")).unwrap();
    crate::persistence::create_tables(&database);
    app.insert_resource(data)
        .insert_resource(pets)
        .insert_resource(crate::persistence::GameDatabase::new(database))
        .add_plugins((
            crate::hunter_pet::HunterPetPlugin,
            crate::pet_control::PetControlPlugin,
            crate::creature_ai::CreatureAiPlugin,
        ));
}

fn measure(entry: u32, count: usize) -> serde_json::Value {
    let _seed = crate::random::seed_scope(SEED);
    // The directory must outlive the app holding its real temporary redb handle.
    let pet_directory = tempfile::tempdir().unwrap();
    let (mut h, unit) = at_level(entry, 32, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
        if entry == 230861 {
            install_pet_runtime(app, pet_directory.path());
        }
    });
    if entry == 230861 {
        h.app
            .insert_resource(super::rotation_imp_tests::prepared_world());
    }
    let targets = prepare_targets(&mut h, count);
    let stats = *h.app.world().get::<DerivedStats>(unit).unwrap();
    let start = h.inbox.log.len();
    let samples = advance_samples(&mut h, unit, &targets);
    let mut report = collect_report(&h, unit, start, samples);
    let dummy_damage: i64 = h.inbox.log[start..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event
                    .source
                    .is_some_and(|source| targets.iter().any(|target| target.to_bits() == source))
        })
        .map(|event| i64::from(event.amount))
        .sum();
    report["dummy_retaliation_damage"] = dummy_damage.into();
    report["entry"] = entry.into();
    report["level"] = 32.into();
    report["targets"] = count.into();
    report["seconds"] = SECONDS.into();
    report["frames"] = FRAMES.into();
    report["seed"] = SEED.into();
    report["production_stats_plugin"] = true.into();
    report["initial_derived_stats"] = serde_json::json!({"attack_power":stats.attack_power,"spell_power":stats.spell_power,"crit_pct":stats.crit_pct,"haste_pct":stats.haste_pct});
    report["limits"] = "Training dummies do not attack. Cap-time is not waste. Energize overflow covers logged spell events only, not unlogged scripts/passive regeneration. Owned-actor damage covers observed direct-owned pets and guardians. Seeded native draws, no forced hit/proc/resource grants; one seed is not statistical or cross-client parity.".into();
    report
}

fn write_report(report: &serde_json::Value) {
    println!("ROTATION_MEASUREMENT {report}");
    if let Some(root) = std::env::var_os("ROTATION_OUTPUT") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join(format!("{}-{}.json", report["entry"], report["targets"])),
            serde_json::to_vec_pretty(report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn rotation_measurement_demonology_lifecycle_st_and_three_targets() {
    let mut empty = Vec::new();
    let entry = 230861;
    for count in [1, 3] {
        let report = measure(entry, count);
        write_report(&report);
        if report["casts"].as_object().unwrap().is_empty()
            || report["total_damage"].as_i64().unwrap() <= 0
            || report["dummy_retaliation_damage"].as_i64().unwrap() != 0
        {
            empty.push((entry, count));
        }
    }
    assert!(
        empty.is_empty(),
        "native cases lacked casts/damage or had dummy retaliation: {empty:?}"
    );
}
