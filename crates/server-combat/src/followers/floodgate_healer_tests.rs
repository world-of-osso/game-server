//! Followers standing in a floor dip that the navigation detail mesh bridges.
use super::profile_behavior_tests::spawn_profile_fixture;
use crate::ground::GroundMap;
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::Position;
use std::collections::HashMap;

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

#[test]
fn follower_in_bridged_dip_steps_along_its_own_floor() {
    use crate::navigation::{Goal, MoveRequest, Navigation};
    // Live Floodgate dip: WMO floor 338.66 under a route surface at 340.28.
    for (floor, should_move) in [(338.66, true), (330.0, false)] {
        let (mut h, unit, _) = spawn_profile_fixture(209072);
        h.app.world_mut().despawn(h.npc);
        let from = Position {
            x: 1657.5,
            y: 338.66,
            z: 2799.9,
        };
        let owner = Position {
            x: 1657.5,
            y: 340.28,
            z: 2809.9,
        };
        let map = InMap::continent(2773);
        h.app.world_mut().entity_mut(h.caster).insert((
            owner,
            map,
            shared::components::Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ));
        h.app
            .world_mut()
            .entity_mut(unit)
            .remove::<crate::creature_ai::PartyFollowSlot>()
            .insert((from, map, crate::creature_ai::FollowOwner::new(h.caster)));
        h.app.insert_resource(GroundMap::flat_arena(floor));
        let dir = tempfile::tempdir().unwrap();
        let mut nav = Navigation::new(dir.path());
        let target = Position {
            x: owner.x,
            y: owner.y,
            z: owner.z - 4.0,
        };
        let route_start = Position { y: 340.28, ..from };
        nav.adopt_candidate_route(
            unit,
            MoveRequest {
                map_id: 2773,
                goal: Goal::FollowFormation(h.caster.to_bits(), point(owner)),
                from,
                to: target,
                distance: 0.0,
                reach: 0.0,
            },
            vec![point(route_start), point(target)],
        );
        h.app.insert_resource(nav);
        crate::creature_ai::install_follow_fixture(&mut h.app);
        h.advance(1);
        let after = *h.app.world().get::<Position>(unit).unwrap();
        if should_move {
            assert!(after.z > from.z, "dip floor must advance: {after:?}");
            assert_eq!(after.y, floor, "follower stays on its own floor");
        } else {
            assert_eq!(after, from, "a floor far below the follower is not a step");
        }
    }
}

#[test]
#[ignore = "requires Operation: Floodgate ground and navigation bakes"]
fn floodgate_follower_leaves_bridged_dip_toward_owner() {
    let data = crate::ground::ground_data_dir();
    let (mut h, unit, _) = spawn_profile_fixture(209057);
    h.app.world_mut().despawn(h.npc);
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    // Live map 2773 copy 2 roster: Follower2 stalled here for eight minutes.
    let start = Position {
        x: 1657.5,
        y: 338.7,
        z: 2799.9,
    };
    let owner = Position {
        x: 1645.7,
        y: 335.3,
        z: 2785.4,
    };
    let map = InMap::continent(2773);
    h.app.world_mut().entity_mut(h.caster).insert((owner, map));
    h.app.world_mut().entity_mut(unit).insert((start, map));
    h.app
        .insert_resource(GroundMap::new(&data, HashMap::from([(2773, 5_839_271)])));
    h.app
        .insert_resource(crate::navigation::Navigation::new(&data.join("navmesh")));
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    for _ in 0..400 {
        h.advance(1);
    }
    let end = *h.app.world().get::<Position>(unit).unwrap();
    assert!(
        point(end).distance(point(owner)) < 8.0,
        "follower stalled at {end:?}; owner {owner:?}"
    );
}
