//! Native Windrunner assistance membership at pull admission.
use bevy::prelude::*;
use shared::components::{Health, Level, Position, UnitLevel};
use std::collections::HashMap;

#[test]
#[ignore = "requires Windrunner native world, ground and LOS data"]
fn windrunner_assistance_snapshot_is_planned_and_tank_can_attack_it() {
    let (mut h, tank, profile) =
        super::profile_behavior_tests::spawn_profile_fixture_with_plugins(209057, |app| {
            app.insert_resource(crate::test_data::world_data().clone());
            app.add_plugins((
                crate::creature_ai::CreatureAiPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
            ));
        });
    let owner = h.caster;
    let map = crate::maps::InMap {
        map_id: 2805,
        instance_id: 73,
    };
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    data.spawns = source
        .spawns
        .iter()
        .filter(|spawn| matches!(spawn.guid, 63805115 | 63805164 | 63805165))
        .cloned()
        .collect::<Vec<_>>()
        .into();
    assert_eq!(data.spawns.len(), 3);
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    h.app.insert_resource(data.clone());
    let root = std::path::PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("ground assets"),
    );
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&root.join("los")).expect("native LOS"),
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
        3
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    h.advance(2);
    let units: HashMap<_, _> = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin)>()
        .iter(h.app.world())
        .map(|(entity, origin)| (origin.guid, entity))
        .collect();
    let caller = units[&63805115];
    let helpers = [units[&63805164], units[&63805165]];
    let here = Position {
        x: 5312.1387,
        y: 126.344864,
        z: 3142.1423,
    };
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((map, Level(90), UnitLevel(90), here));
    h.app.world_mut().entity_mut(tank).insert((map, here));
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    let mut health = h.app.world_mut().get_mut::<Health>(tank).unwrap();
    health.current = health.max;
    drop(health);
    let planned = super::pull_set::members(h.app.world(), caller, tank);
    assert_eq!(planned.len(), 3);
    assert!(planned.contains(&caller));
    assert!(helpers.iter().all(|helper| planned.contains(helper)));
    assert!(
        super::spell_safety::safe(h.app.world(), tank, caller, 31935),
        "the tank may use its native chain against planned assistants"
    );
    crate::creature_aggro::attack_start(h.app.world_mut(), caller, tank);
    for _ in 0..60 {
        h.advance(1);
        if helpers
            .iter()
            .all(|&helper| crate::combat_refs::has_active_reference_to(h.app.world(), helper, tank))
        {
            break;
        }
    }
    assert!(
        helpers
            .iter()
            .all(|&helper| crate::combat_refs::has_active_reference_to(
                h.app.world(),
                helper,
                tank
            )),
        "planned assistants must actually join the tank's combat"
    );
    let after = super::pull_set::members(h.app.world(), caller, tank);
    assert!(
        helpers.iter().all(|helper| after.contains(helper)),
        "the admission snapshot must survive delayed arrival"
    );
}
