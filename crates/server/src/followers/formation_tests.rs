//! Recorded LFG2436 Ruby entrance: native AI + asynchronous navigation + movement.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::ground::GroundMap;
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::Position;
use std::{collections::HashMap, path::PathBuf};

#[test]
#[ignore = "RED: recorded Ruby entrance party must follow without overlapping; requires FOLLOWER_RUBY_DATA/GROUND"]
fn ruby_entrance_party_follows_owner_and_separates() {
    let data = PathBuf::from(std::env::var_os("FOLLOWER_RUBY_DATA").unwrap());
    let ground = PathBuf::from(std::env::var_os("FOLLOWER_RUBY_GROUND").unwrap());
    let (mut h, tank, _) = spawn_profile_fixture_with_plugins(209057, |_| {});
    h.app.world_mut().despawn(h.npc);
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let profiles = super::catalog::load_catalog(&db).unwrap().profiles;
    // Native summons require the real class-level base stats as well as templates.
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    h.app.world_mut().despawn(tank);
    let profiles: Vec<_> = [209057, 209053, 209059, 209072]
        .iter()
        .map(|entry| {
            profiles
                .iter()
                .find(|profile| profile.entry == *entry)
                .unwrap()
                .clone()
        })
        .collect();
    // Actual production coordinates, already in Bevy x/up-y/z, not WoW x/y/up-z.
    let entrance = Position {
        x: 1386.2,
        y: 131.0,
        z: 169.719,
    };
    let owner = Position {
        x: 1382.726,
        y: 128.6585,
        z: 180.8571,
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((entrance, InMap::continent(2521)));
    let members = super::spawning::spawn_members(h.app.world_mut(), h.caster, &profiles).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(owner);
    for &unit in &members {
        h.app.world_mut().entity_mut(unit).insert((
            entrance,
            InMap::continent(2521),
            crate::creature_ai::SpawnPoint {
                x: entrance.x,
                y: entrance.y,
                z: entrance.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
        ));
    }
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    h.app.insert_resource(GroundMap::new(
        &data,
        HashMap::from([(2521, maps.map(2521).unwrap().wdt_file_data_id)]),
    ));
    h.app
        .insert_resource(crate::navigation::Navigation::new(&ground.join("navmesh")));
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    let initial_gap = Vec3::new(owner.x - entrance.x, 0.0, owner.z - entrance.z).length();
    //23 seconds, matching the capture; Harness drains the real asynchronous worker.
    h.advance(460);
    let positions: Vec<_> = members
        .iter()
        .map(|&unit| *h.app.world().get::<Position>(unit).unwrap())
        .collect();
    eprintln!("recorded Ruby entrance {entrance:?}, owner {owner:?}, party {positions:?}");
    for (index, at) in positions.iter().enumerate() {
        let gap = (owner.x - at.x).hypot(owner.z - at.z);
        assert!(
            gap < initial_gap - 1.0,
            "member{index} must advance toward owner within23s: {at:?}, gap{gap}"
        );
    }
    for (index, a) in positions.iter().enumerate() {
        for b in positions.iter().skip(index + 1) {
            assert!(
                (a.x - b.x).hypot(a.z - b.z) > 0.5,
                "idle party members must not occupy one point: {a:?} / {b:?}"
            );
        }
    }
}
