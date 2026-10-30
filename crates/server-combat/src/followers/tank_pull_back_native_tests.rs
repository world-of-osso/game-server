//! Real Windrunner ground, source placements and native tank/creature movement.
use super::{Follower, ai, owner_trail, pull_history, tank_pull_back};
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
    use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
    let (mut h, tank, profile) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.insert_resource(crate::test_data::world_data().clone());
        app.add_plugins((
            crate::creature_ai::CreatureAiPlugin,
            crate::creature_aggro::CreatureAggroPlugin,
        ));
    });
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
    // The source spawn is about 21 yards away: an attack request alone cannot
    // land a melee swing. Start combat with the mage's native ranged spell.
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
    crate::spell_cast::creature_cast(h.app.world_mut(), tank, 62124, Some(target), false)
        .expect("native ranged taunt must acquire the intended Groundskeeper");
    let follower = h.app.world().get::<Follower>(tank).unwrap().clone();
    let party = vec![owner, tank];
    let mut farthest = 0.0_f32;
    for _ in 0..1200 {
        owner_trail::record(h.app.world_mut(), owner);
        pull_history::sample_all(h.app.world_mut());
        tank_pull_back::update(h.app.world_mut(), tank, &follower, &party, 40.0);
        h.advance(1);
        let point = crate::spell_cast::targets::position(h.app.world(), tank).unwrap();
        farthest = farthest.max(start.distance(point));
        assert!(
            !crate::combat_refs::has_active_reference_to(h.app.world(), hatchling, tank),
            "unrelated hatchling acquired the tank"
        );
        if farthest > 20.0 && !tank_pull_back::party_should_wait(h.app.world(), owner) {
            break;
        }
    }
    let end = crate::spell_cast::targets::position(h.app.world(), tank).unwrap();
    assert!(
        farthest > 20.0,
        "tank must retreat on real ground: start={start:?}, end={end:?}"
    );
    assert!(
        !tank_pull_back::party_should_wait(h.app.world(), owner),
        "tank must wait for the acquired pack to arrive before releasing the party"
    );
    assert!(h.app.world().get::<Health>(hatchling).unwrap().current > 0.0);
}
