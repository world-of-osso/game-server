//! Native Windrunner healer follow geometry from server27b.
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use shared::components::{Level, Position, Rotation, UnitLevel};
use std::collections::HashMap;

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
                difficulty_id: 1,
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
        "the authored follow offset must enter the Dragonhawk radius"
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
