//! Recorded LFG2436 Ruby entrance: native AI + asynchronous navigation + movement.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::ground::GroundMap;
use crate::maps::InMap;
use crate::spell_cast::test_harness::spawn_npc;
use bevy::prelude::*;
use shared::components::{Npc, Position, UnitPowers};
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
    let native =
        crate::spell_cast::test_harness::load_native_creature_world(&[209053, 209059, 209072]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    let mut members = vec![tank];
    for entry in [209053, 209059, 209072] {
        let profile = profiles
            .iter()
            .find(|profile| profile.entry == entry)
            .unwrap()
            .clone();
        let unit = spawn_npc(&mut h.app, 0.0);
        h.app.world_mut().entity_mut(unit).insert((
            Npc {
                template_id: entry,
                name: profile.name.clone(),
            },
            UnitPowers::default(),
        ));
        super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
        members.push(unit);
    }
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
        .insert((owner, InMap::continent(2521)));
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
    //23 seconds, matching the capture; yield for the real asynchronous path worker.
    for _ in 0..460 {
        h.advance(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
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
