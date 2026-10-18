//! Ordinary source-spawn projection and native follower stats, without a new live run.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::combat::{AutoAttack, forecast};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use crate::world_data::WorldData;
use bevy::prelude::*;
use shared::components::{Health, Npc, UnitLevel, UnitPowers};

pub(super) fn source_fixture(entry: u32) -> (Harness, Entity, Entity) {
    let (mut h, tank, _) = spawn_profile_fixture_with_plugins(209057, |_| {});
    h.advance(3);
    let map = InMap {
        map_id: 34,
        instance_id: 71,
    };
    h.app.world_mut().entity_mut(tank).insert(map);
    h.app.world_mut().entity_mut(h.caster).insert(map);
    let data = crate::test_data::world_data();
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| spawn.map == 34 && spawn.template_entry == entry && spawn.spawns_at(1))
        .expect("authored Normal Stockade spawn");
    let template = data.template_for(entry, 1).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(34, 791060)]),
    );
    let enemy = spawn_source_actor(h.app.world_mut(), data, template, spawn, map, &mut ground);
    h.app
        .world_mut()
        .resource_mut::<WorldData>()
        .templates
        .insert(entry, template.clone());
    h.app.insert_resource(
        crate::scripted_ai::smart_ai::load_smart_scripts(&crate::test_data::world_db_path())
            .unwrap(),
    );
    (h, tank, enemy)
}

fn spawn_source_actor(
    world: &mut World,
    data: &WorldData,
    template: &crate::world_data::CreatureTemplate,
    spawn: &crate::world_data::CreatureSpawn,
    map: InMap,
    ground: &mut crate::ground::GroundMap,
) -> Entity {
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, world);
    let enemy = crate::zones::spawn_creature(
        &mut commands,
        data,
        template,
        &mut crate::zones::CreatureSpawnContext {
            map,
            difficulty_id: 1,
            ground: Some(ground),
            temporary: false,
        },
        false,
        spawn,
        &mut rand::thread_rng(),
    )
    .expect("ordinary sourced actor");
    queue.apply(world);
    enemy
}

#[test]
fn tank_pull_forecast_reads_generic_source_stats_and_actual_swing_timer_without_mutation() {
    let (mut h, tank, enemy) = source_fixture(46249);
    let mut attack = AutoAttack::new(tank);
    attack.timer = 1.25;
    h.app.world_mut().entity_mut(enemy).insert(attack);
    let health = h.health(enemy);
    let level = h.app.world().get::<UnitLevel>(enemy).unwrap().0;
    let rank = h.app.world().resource::<WorldData>().templates[&46249].rank;
    let estimate = forecast::read_generic_creature(h.app.world(), enemy, tank).unwrap();
    assert!(estimate.expected_hit.is_finite() && estimate.expected_hit > 0.0);
    assert!(estimate.largest_hit >= estimate.expected_hit);
    assert!(estimate.interval_secs.is_finite() && estimate.interval_secs > 0.0);
    assert_eq!(estimate.next_swing_secs, 1.25);
    assert!(estimate.remaining_health.is_finite() && estimate.remaining_health > 0.0);
    assert_eq!(estimate.level, level);
    assert_eq!(estimate.rank, rank);
    assert_eq!(h.health(enemy), health);
    assert_eq!(h.app.world().get::<AutoAttack>(enemy).unwrap().timer, 1.25);
    assert_eq!(h.app.world().get::<AutoAttack>(enemy).unwrap().victim, tank);
}

#[test]
fn tank_pull_forecast_rank_does_not_apply_an_extra_elite_coefficient() {
    let (mut h, tank, enemy) = source_fixture(46249);
    let original = forecast::read_generic_creature(h.app.world(), enemy, tank).unwrap();
    // Classification alone cannot multiply already initialized ExpectedStat damage.
    h.app
        .world_mut()
        .resource_mut::<WorldData>()
        .templates
        .get_mut(&46249)
        .unwrap()
        .rank = 0;
    let ordinary = forecast::read_generic_creature(h.app.world(), enemy, tank).unwrap();
    assert_eq!(ordinary.expected_hit, original.expected_hit);
    assert_eq!(ordinary.largest_hit, original.largest_hit);
    assert_eq!(ordinary.interval_secs, original.interval_secs);
    assert_eq!(ordinary.rank, 0);
}

#[test]
fn tank_pull_forecast_smartai_and_named_boss_have_no_numeric_forecast() {
    for entry in [1720, 46254] {
        let (h, tank, enemy) = source_fixture(entry);
        assert!(
            forecast::read_generic_creature(h.app.world(), enemy, tank).is_err(),
            "scripted source{entry} must wait, not receive guessed timing/burst"
        );
    }
}

fn read_native_healer_profile() -> super::Profile {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    super::catalog::load_catalog(&db)
        .unwrap()
        .profiles
        .into_iter()
        .find(|profile| profile.entry == 209072)
        .unwrap()
}

pub(super) fn add_native_healer(h: &mut Harness) -> Entity {
    let profile = read_native_healer_profile();
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[profile.entry]);
    h.app
        .world_mut()
        .resource_mut::<WorldData>()
        .templates
        .extend(native.templates);
    let owner = h.caster;
    let map = *h.app.world().get::<InMap>(owner).unwrap();
    let healer = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(healer).insert((
        Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        },
        UnitPowers::default(),
        map,
    ));
    super::spawning::initialize(h.app.world_mut(), owner, healer, profile).unwrap();
    healer
}

#[test]
fn tank_pull_forecast_lifetime_stops_at_native_divine_protection_stat_expiry() {
    let (mut h, tank, enemy) = source_fixture(46249);
    crate::spell_cast::creature_cast(h.app.world_mut(), tank, 498, Some(tank), false).unwrap();
    h.advance(3);
    let expiry = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(tank)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 498)
        .unwrap()
        .expires_at
        .unwrap();
    let estimate = forecast::read_generic_creature(h.app.world(), enemy, tank).unwrap();
    assert_eq!(
        estimate.valid_until,
        Some(expiry),
        "do not credit native20% mitigation past its actual lifetime"
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(tank)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 498),
        "projection must not expire the real aura"
    );
}

#[test]
fn tank_pull_forecast_lifetime_does_not_blanket_refuse_for_a_pure_native_hot() {
    let (mut h, tank, enemy) = source_fixture(46249);
    let healer = add_native_healer(&mut h);
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 774, Some(tank), false).unwrap();
    h.advance(3);
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(tank)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 774 && aura.expires_at.is_some())
    );
    let estimate = forecast::read_generic_creature(h.app.world(), enemy, tank).unwrap();
    assert_eq!(
        estimate.valid_until, None,
        "pure healing expiry is not a melee stat expiry"
    );
}

#[test]
fn tank_pull_forecast_dead_or_other_copy_enemy_is_not_a_candidate() {
    let (mut h, tank, enemy) = source_fixture(46249);
    assert!(forecast::read_generic_creature(h.app.world(), enemy, tank).is_ok());
    h.app.world_mut().get_mut::<Health>(enemy).unwrap().current = 0.0;
    assert!(forecast::read_generic_creature(h.app.world(), enemy, tank).is_err());
    h.app.world_mut().get_mut::<Health>(enemy).unwrap().current = 1.0;
    h.app.world_mut().entity_mut(enemy).insert(InMap {
        map_id: 34,
        instance_id: 72,
    });
    assert!(forecast::read_generic_creature(h.app.world(), enemy, tank).is_err());
}
