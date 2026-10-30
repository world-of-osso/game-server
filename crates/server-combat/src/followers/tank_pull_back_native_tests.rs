//! Real Windrunner ground, source placements and native tank/creature movement.
use super::{Follower, owner_trail, pull_history, tank_pull_back};
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::{Health, Level, Position, UnitLevel};
use std::collections::HashMap;

fn at(point: Vec3) -> Position {
    Position {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}

#[test]
#[ignore = "requires Windrunner native ground, navigation and sourced follower data"]
fn windrunner_tank_pulls_back_and_waits_without_pulling_hatchling() {
    run_native_pull_back(false, false);
}

#[test]
#[ignore = "requires Windrunner native ground, navigation and sourced follower data"]
fn windrunner_production_ai_pulls_back_without_pulling_hatchling() {
    run_native_pull_back(true, false);
}

#[test]
#[ignore = "requires Windrunner native ground, navigation and sourced follower data"]
fn windrunner_production_party_follows_and_heals_during_pull_back() {
    run_native_pull_back(true, true);
}

fn run_native_pull_back(production: bool, full_party: bool) {
    use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
    let (mut h, tank, profile) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.insert_resource(crate::test_data::world_data().clone());
        app.add_plugins((
            crate::creature_ai::CreatureAiPlugin,
            crate::creature_aggro::CreatureAggroPlugin,
        ));
    });
    if production {
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(super::catalog::load_catalog(&db).unwrap());
    }
    let owner = h.caster;
    let map = InMap {
        map_id: 2805,
        instance_id: 71,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    // Actual source coordinates and templates; other spawns are outside this two-pack fixture.
    let spawns: Vec<_> = source
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, 63805115 | 63805164))
        .cloned()
        .collect();
    assert_eq!(spawns.len(), 2);
    data.spawns = spawns.into();
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data.clone());
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((map, Level(90), UnitLevel(90)));
    h.app.world_mut().entity_mut(tank).insert(map);
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &std::path::PathBuf::from(
            std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
        ),
        HashMap::from([(2805, 6_099_383)]),
    );
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    let count = crate::zones::spawn_map_creatures(
        &mut commands,
        &data,
        crate::zones::MapContent {
            map,
            difficulty_id: 1,
            dead_bosses: vec![],
        },
        false,
        Some(&mut ground),
    );
    assert_eq!(count, 2);
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    crate::navigation::install_fixture(
        &mut h.app,
        crate::navigation::Navigation::new(&std::path::PathBuf::from(
            std::env::var_os("FOLLOWER_NAV_CANONICAL").expect("navigation assets"),
        )),
    );
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: vec![tank],
            player_role: shared::group::GroupRole::Dps,
        });
    let find_source = |world: &mut World, guid| {
        world
            .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
            .iter(world)
            .find(|(_, origin, copy)| origin.guid == guid && **copy == map)
            .map(|(unit, _, _)| unit)
            .unwrap()
    };
    let target = find_source(h.app.world_mut(), 63805115);
    let hatchling = find_source(h.app.world_mut(), 63805164);
    let start = Vec3::new(5316.6274, 126.463684, 3129.5269);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(shared::components::CombatStatus(false));
    for point in [
        Vec3::new(5314.594, 136.86368, 3080.4695),
        Vec3::new(5314.983, 136.06369, 3089.843),
        Vec3::new(5315.2554, 135.21678, 3096.4187),
        Vec3::new(5315.629, 133.40715, 3105.4285),
        Vec3::new(5316.076, 130.74896, 3116.221),
        start,
    ] {
        h.app.world_mut().entity_mut(owner).insert(at(point));
        owner_trail::record(h.app.world_mut(), owner);
    }
    h.app.world_mut().entity_mut(tank).insert(at(start));
    let mut companions = Vec::new();
    if full_party {
        let profiles: Vec<_> = h
            .app
            .world()
            .resource::<super::FollowerCatalog>()
            .profiles
            .iter()
            .filter(|profile| matches!(profile.entry, 209072 | 209059))
            .cloned()
            .collect();
        assert_eq!(profiles.len(), 2);
        for profile in profiles {
            let unit = super::spawning::spawn(h.app.world_mut(), owner, profile).unwrap();
            companions.push(unit);
        }
        h.app
            .world_mut()
            .entity_mut(owner)
            .insert(super::stockade_diagnostic::DiagnosticParty {
                copy: map,
                members: std::iter::once(tank)
                    .chain(companions.iter().copied())
                    .collect(),
                player_role: shared::group::GroupRole::Dps,
            });
    }
    if production {
        assert!(
            !super::spell_safety::safe(h.app.world(), tank, target, 31935),
            "Avenger's Shield must not bounce into the idle hatchling"
        );
        assert!(
            super::spell_safety::safe(h.app.world(), tank, target, 62124),
            "the single-target native taunt remains available"
        );
    }
    // The source spawn is about 21 yards away: an attack request alone cannot
    // land a melee swing. Start combat with the mage's native ranged spell.
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::class_progression::KnownSpells::fixture(vec![30451]));
    h.cast(30451, Some(target));
    for _ in 0..180 {
        h.advance(1);
        if h.app
            .world()
            .get::<shared::components::CombatStatus>(owner)
            .is_some_and(|status| status.0)
        {
            break;
        }
    }
    assert!(
        h.app
            .world()
            .get::<shared::components::CombatStatus>(owner)
            .is_some_and(|status| status.0),
        "ordinary owner ranged attack must start combat: {:?}",
        h.inbox.failed
    );
    owner_trail::record(h.app.world_mut(), owner);
    pull_history::sample_all(h.app.world_mut());
    let initial_health = if full_party {
        let mut health = h.app.world_mut().get_mut::<Health>(owner).unwrap();
        health.current = health.max * 0.1;
        health.current
    } else {
        0.0
    };
    if !production {
        crate::spell_cast::creature_cast(h.app.world_mut(), tank, 62124, Some(target), false)
            .expect("native ranged taunt must acquire the intended Groundskeeper");
    }
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    let party = vec![owner, tank];
    let minimum_retreat = if production { 1.0 } else { 20.0 };
    let mut farthest = 0.0_f32;
    let mut waited = false;
    let mut acquired = false;
    let mut healed_while_waiting = false;
    for _ in 0..1200 {
        if production {
            super::update_ai(h.app.world_mut());
        } else {
            owner_trail::record(h.app.world_mut(), owner);
            pull_history::sample_all(h.app.world_mut());
            tank_pull_back::update(h.app.world_mut(), tank, &follower, &party, 40.0);
        }
        waited |= tank_pull_back::party_should_wait(h.app.world(), owner);
        acquired |= h
            .app
            .world()
            .get::<crate::networking::CombatTarget>(target)
            .is_some_and(|victim| victim.0 == Some(tank));
        let waiting_now = tank_pull_back::party_should_wait(h.app.world(), owner);
        let before_events = h.inbox.log.len();
        h.advance(1);
        if full_party && waiting_now {
            healed_while_waiting |= h.health(owner) > initial_health;
            assert!(
                h.inbox.log[before_events..].iter().all(|event| {
                    !matches!(event.kind, shared::protocol::CombatLogKind::Damage)
                        || !companions
                            .iter()
                            .any(|unit| event.source == Some(unit.to_bits()))
                }),
                "non-tank follower damaged an enemy while waiting"
            );
            assert!(
                crate::combat_refs::has_active_reference_to(h.app.world(), owner, target),
                "waiting must preserve owner combat reference"
            );
        }
        let point = crate::spell_cast::targets::position(h.app.world(), tank).unwrap();
        farthest = farthest.max(start.distance(point));
        assert!(
            !crate::combat_refs::has_active_reference_to(h.app.world(), hatchling, tank),
            "unrelated hatchling acquired the tank: tank={:?} target={:?} hatchling={:?} waited={waited} acquired={acquired} target_victim={:?} hatchling_victim={:?} recent_events={:?}",
            crate::spell_cast::targets::position(h.app.world(), tank),
            crate::spell_cast::targets::position(h.app.world(), target),
            crate::spell_cast::targets::position(h.app.world(), hatchling),
            h.app
                .world()
                .get::<crate::networking::CombatTarget>(target)
                .and_then(|victim| victim.0),
            h.app
                .world()
                .get::<crate::networking::CombatTarget>(hatchling)
                .and_then(|victim| victim.0),
            h.inbox.log.iter().rev().take(12).collect::<Vec<_>>()
        );
        if waited
            && acquired
            && farthest > minimum_retreat
            && !tank_pull_back::party_should_wait(h.app.world(), owner)
        {
            break;
        }
    }
    let end = crate::spell_cast::targets::position(h.app.world(), tank).unwrap();
    assert!(
        farthest > minimum_retreat,
        "tank must retreat on real ground: start={start:?}, end={end:?}"
    );
    assert!(waited, "party must wait for the pull-back");
    if full_party {
        assert!(healed_while_waiting, "healer must heal while waiting");
    }
    assert!(acquired, "tank must acquire the intended Groundskeeper");
    if production {
        assert!(
            super::spell_safety::safe(h.app.world(), tank, target, 31935),
            "Avenger's Shield must become available after the pack arrives safely"
        );
    }
    assert!(
        !tank_pull_back::party_should_wait(h.app.world(), owner),
        "tank must wait for the acquired pack to arrive before releasing the party"
    );
    assert!(h.app.world().get::<Health>(hatchling).unwrap().current > 0.0);
}
