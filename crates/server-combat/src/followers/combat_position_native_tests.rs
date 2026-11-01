//! Native Windrunner healer follow geometry from server27b.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::{Level, Position, Rotation, UnitLevel};
use std::collections::HashMap;

#[test]
#[ignore = "requires Windrunner native ground, navigation and source world data"]
fn pending_follow_stays_held_and_cannot_step_into_windrunner_watcher() {
    let (mut h, follower, _) =
        super::profile_behavior_tests::spawn_profile_fixture_with_plugins(209057, |app| {
            app.insert_resource(crate::test_data::world_data().clone());
            app.add_plugins(crate::creature_ai::CreatureAiPlugin);
        });
    let owner = h.caster;
    let map = crate::maps::InMap {
        map_id: 2805,
        instance_id: 72,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    data.spawns = source
        .spawns
        .iter()
        .filter(|spawn| spawn.guid == 63805179)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    assert_eq!(data.spawns.len(), 1);
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data.clone());
    let from = Position {
        x: 5228.9336,
        y: 107.99762,
        z: 3168.8545,
    };
    let unsafe_step = Position {
        x: 5229.1475,
        y: 107.80156,
        z: 3169.1316,
    };
    h.app.world_mut().entity_mut(follower).insert((
        map,
        UnitLevel(90),
        from,
        shared::components::CharacterVisual {
            race: 1,
            class: 10,
            appearance: Default::default(),
        },
    ));
    let owner_position = Position {
        x: 5234.0,
        y: 104.8,
        z: 3169.5,
    };
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((map, Level(90), UnitLevel(90), owner_position));
    let root = std::path::PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
    );
    let mut ground = crate::ground::GroundMap::new(&root, HashMap::from([(2805, 6_099_383)]));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    assert_eq!(
        crate::zones::spawn_map_creatures(
            &mut commands,
            &data,
            crate::zones::MapContent {
                map,
                difficulty_id: 205,
                dead_bosses: vec![]
            },
            false,
            Some(&mut ground)
        ),
        1
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    crate::navigation::install_fixture(
        &mut h.app,
        crate::navigation::Navigation::new(&std::path::PathBuf::from(
            std::env::var_os("FOLLOWER_NAV_CANONICAL").expect("navigation assets"),
        )),
    );
    let watcher = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin)>()
        .iter(h.app.world())
        .find(|(_, origin)| origin.guid == 63805179)
        .unwrap()
        .0;
    h.app.world_mut().entity_mut(watcher).insert(UnitLevel(90));
    h.advance(1);
    *h.app.world_mut().get_mut::<Position>(follower).unwrap() = from;
    let mut geometry = SystemState::<crate::creature_aggro::AggroDistance>::new(h.app.world_mut());
    let radius =
        super::pull_route::read_native_radius(h.app.world(), &mut geometry, follower, watcher)
            .unwrap();
    assert!(
        (radius - 7.4).abs() < 0.1,
        "native watcher radius: {radius}"
    );
    assert!(super::positioning::hold_unsafe_approach(
        h.app.world_mut(),
        follower,
        owner
    ));
    h.app
        .world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs(6));
    assert!(
        super::positioning::hold_unsafe_approach(h.app.world_mut(), follower, owner),
        "unresolved owner follow must remain held"
    );
    let data = h
        .app
        .world()
        .get::<super::Follower>(follower)
        .unwrap()
        .clone();
    super::positioning::update_combat_exclusions(h.app.world_mut(), follower, &data);
    crate::creature_ai::move_follower(h.app.world_mut(), follower, unsafe_step, 7.0);
    assert_eq!(
        *h.app.world().get::<Position>(follower).unwrap(),
        from,
        "follow step must not enter watcher radius plus margin"
    );
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert(crate::creature_ai::FollowOwner::new(owner));
    for _ in 0..50 {
        h.advance(1);
        let point = *h.app.world().get::<Position>(follower).unwrap();
        let center = *h.app.world().get::<Position>(watcher).unwrap();
        assert!(
            Vec3::new(point.x, point.y, point.z).distance(Vec3::new(center.x, center.y, center.z))
                >= radius + 0.5,
            "owner follow entered watcher margin"
        );
    }
    assert!(
        crate::combat_refs::active_opponents(h.app.world(), watcher)
            .next()
            .is_none()
    );
    *h.app.world_mut().get_mut::<Position>(follower).unwrap() = from;
    let safe_step = Position {
        x: from.x - 0.5,
        ..from
    };
    crate::creature_ai::move_follower(h.app.world_mut(), follower, safe_step, 7.0);
    assert_eq!(
        *h.app.world().get::<Position>(follower).unwrap(),
        safe_step,
        "safe movement must remain available after timeout"
    );
}

#[test]
#[ignore = "requires Windrunner native ground, navigation and source world data"]
fn crenna_follow_position_does_not_pull_windrunner_dragonhawk() {
    let (mut h, healer, profile) =
        super::profile_behavior_tests::spawn_profile_fixture_with_plugins(209072, |app| {
            app.insert_resource(crate::test_data::world_data().clone());
            app.add_plugins((
                crate::creature_ai::CreatureAiPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
            ));
        });
    let owner = h.caster;
    let enemy = h.npc;
    let map = crate::maps::InMap {
        map_id: 2805,
        instance_id: 72,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    data.spawns = source
        .spawns
        .iter()
        .filter(|spawn| spawn.guid == 63805162)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    assert_eq!(data.spawns.len(), 1);
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data.clone());
    h.app.world_mut().entity_mut(owner).insert((
        map,
        Level(90),
        UnitLevel(90),
        Position {
            x: 5312.1387,
            y: 126.344864,
            z: 3142.1423,
        },
        Rotation {
            x: 0.0,
            y: -std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    h.app.world_mut().entity_mut(healer).insert((
        map,
        Position {
            x: 5311.517,
            y: 126.34487,
            z: 3141.0833,
        },
    ));
    h.app.world_mut().entity_mut(enemy).insert((
        map,
        Position {
            x: 5310.0,
            y: 126.344864,
            z: 3138.0,
        },
    ));
    let model = &data.templates[&209072].models[0];
    h.app.world_mut().entity_mut(healer).insert((
        shared::components::ModelDisplay {
            display_id: model.display_id,
        },
        shared::components::UnitScale(model.display_scale),
    ));
    super::spawning::initialize(h.app.world_mut(), owner, healer, profile).unwrap();
    let root = std::path::PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
    );
    let mut ground = crate::ground::GroundMap::new(&root, HashMap::from([(2805, 6_099_383)]));
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    assert_eq!(
        crate::zones::spawn_map_creatures(
            &mut commands,
            &data,
            crate::zones::MapContent {
                map,
                difficulty_id: 205,
                dead_bosses: vec![]
            },
            false,
            Some(&mut ground)
        ),
        1
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    crate::navigation::install_fixture(
        &mut h.app,
        crate::navigation::Navigation::new(&std::path::PathBuf::from(
            std::env::var_os("FOLLOWER_NAV_CANONICAL").expect("navigation assets"),
        )),
    );
    let hawk = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin)>()
        .iter(h.app.world())
        .find(|(_, origin)| origin.guid == 63805162)
        .unwrap()
        .0;
    h.app.world_mut().entity_mut(hawk).insert(UnitLevel(90));
    crate::combat_refs::set_in_combat_with(h.app.world_mut(), owner, enemy);
    h.app.world_mut().entity_mut(healer).insert((
        crate::creature_ai::FollowOwner::new(owner),
        crate::creature_ai::PartyFollowSlot::new(1, 3),
    ));
    let follower = h
        .app
        .world()
        .get::<super::Follower>(healer)
        .unwrap()
        .clone();
    super::positioning::update_combat_exclusions(h.app.world_mut(), healer, &follower);
    println!(
        "NATIVE_HAWK_LEVEL hawk={:?} scaling={:?} healer={:?} model={:?}",
        h.app.world().get::<UnitLevel>(hawk).map(|level| level.0),
        h.app
            .world()
            .get::<shared::level_scaling::LevelScaling>(hawk),
        h.app.world().get::<UnitLevel>(healer).map(|level| level.0),
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(healer)
            .map(|model| model.display_id)
    );
    let mut geometry = SystemState::<crate::creature_aggro::AggroDistance>::new(h.app.world_mut());
    let radius =
        super::pull_route::read_native_radius(h.app.world_mut(), &mut geometry, healer, hawk)
            .unwrap();
    let initial = crate::spell_cast::targets::position(h.app.world(), healer).unwrap();
    let center = crate::spell_cast::targets::position(h.app.world(), hawk).unwrap();
    assert!(initial.distance(center) > radius);
    let unguarded_follow_point = Vec3::new(5316.1387, 126.344864, 3142.1423);
    assert!(
        unguarded_follow_point.distance(center) < radius,
        "the follow offset {unguarded_follow_point:?} must enter the Dragonhawk radius {radius} around {center:?}"
    );
    let mut farthest = 0.0_f32;
    for _ in 0..200 {
        h.advance(1);
        let position = crate::spell_cast::targets::position(h.app.world(), healer).unwrap();
        farthest = farthest.max(initial.distance(position));
        assert!(
            position.distance(center) >= radius,
            "Crenna entered Dragonhawk aggro radius: {position:?}, radius {radius}"
        );
        assert!(!crate::combat_refs::has_active_reference_to(
            h.app.world(),
            hawk,
            healer
        ));
    }
    assert!(
        farthest > 0.5,
        "Crenna must actually move along the native follow path"
    );
}
