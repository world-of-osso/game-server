//! Actual Stockade collision/nav artifacts, with production fixed-order AI/follow ticks.
use super::profile_behavior_tests::spawn_profile_fixture;
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::Position;
use std::{collections::HashMap, path::PathBuf};

fn wow(x: f32, y: f32, z: f32) -> Position {
    Position { x, y: z, z: -y }
}
fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}
pub(super) fn stockade_fixture(from: Position) -> (Harness, Entity, PathBuf) {
    let (mut h, unit, _) = spawn_profile_fixture(209057);
    h.app.world_mut().despawn(h.npc);
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    let owner = wow(103.0, 76.0, -34.85623);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((owner, InMap::continent(34)));
    h.app.world_mut().entity_mut(unit).insert((
        from,
        InMap::continent(34),
        crate::creature_ai::SpawnPoint {
            x: from.x,
            y: from.y,
            z: from.z,
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 1.0,
        },
    ));
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data");
    h.app.insert_resource(GroundMap::new(
        &engine,
        HashMap::from([(34, maps.map(34).unwrap().wdt_file_data_id)]),
    ));
    let nav = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh");
    h.app
        .insert_resource(crate::navigation::Navigation::new(&nav));
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    (h, unit, engine)
}

#[test]
fn follower_follow_routes_around_actual_stockade_walls_without_crossing_collision() {
    // Actual Slayer home and the stationary-player point of the existing live oracle.
    let from = wow(150.557, 45.8164, -34.85624);
    let (mut h, unit, engine) = stockade_fixture(from);
    let mut los = los::LosMap::new(&engine.join("los/34"));
    let mut ground = h.app.world_mut().remove_resource::<GroundMap>().unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    h.app.insert_resource(GroundMap::new(
        &engine,
        HashMap::from([(34, maps.map(34).unwrap().wdt_file_data_id)]),
    ));
    let mut previous = from;
    for tick in 0..1000 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            los.clear(point(previous) + Vec3::Y, point(at) + Vec3::Y)
                .unwrap(),
            "follower tick{tick} crossed authored collision: {previous:?}->{at:?}"
        );
        let GroundProbe::Supported(floor) = ground.ground_at(34, point(at)) else {
            panic!("unsupported follower feet {at:?}")
        };
        assert!(
            (at.y - floor).abs() < 0.05,
            "follower feet must stay on the real floor"
        );
        previous = at;
    }
    let owner = *h.app.world().get::<Position>(h.caster).unwrap();
    let gap = (owner.x - previous.x).hypot(owner.z - previous.z);
    assert!(
        (gap - 4.0).abs() < 0.1,
        "follower must finish its four-yard follow point, not stop on the tolerance boundary: {gap}"
    );
}

#[test]
fn follower_regroup_finishes_active_follow_instead_of_resetting_movement_each_tick() {
    let (mut h, unit, _) = stockade_fixture(wow(116.0, 76.0, -34.85623));
    for _ in 0..300 {
        h.advance(1);
    }
    let at = *h.app.world().get::<Position>(unit).unwrap();
    let owner = *h.app.world().get::<Position>(h.caster).unwrap();
    let gap = (owner.x - at.x).hypot(owner.z - at.z);
    assert!(
        (gap - 4.0).abs() < 0.1,
        "active regroup must reach four yards after the owner stops: {gap}"
    );
}

#[test]
fn follower_unbaked_follow_and_chase_hold_position_instead_of_using_direct_movement() {
    let from = wow(116.0, 76.0, -34.85623);
    let (mut h, unit, _) = stockade_fixture(from);
    let missing_bake = tempfile::tempdir().unwrap();
    h.app
        .insert_resource(crate::navigation::Navigation::new(missing_bake.path()));
    for _ in 0..40 {
        h.advance(1);
    }
    assert_eq!(
        *h.app.world().get::<Position>(unit).unwrap(),
        from,
        "unbaked follow must not cross authored geometry directly"
    );
    let enemy = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert((wow(103.0, 76.0, -34.85623), InMap::continent(34)));
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = Some(enemy);
    for _ in 0..40 {
        h.advance(1);
    }
    assert_eq!(
        *h.app.world().get::<Position>(unit).unwrap(),
        from,
        "unbaked combat chase must not become direct movement either"
    );
}
