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
#[ignore = "requires FOLLOWER_RUBY_DATA and FOLLOWER_RUBY_GROUND authored private bakes"]
fn follower_ruby_regroup_walks_from_entrance_across_metalplate_to_boss_floor() {
    let data = PathBuf::from(std::env::var_os("FOLLOWER_RUBY_DATA").unwrap());
    let ground = PathBuf::from(std::env::var_os("FOLLOWER_RUBY_GROUND").unwrap());
    let (mut h, unit, _) = spawn_profile_fixture(209057);
    h.app.world_mut().despawn(h.npc);
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    // Actual entrance and the collected supported boss column, not a spawn mutation.
    let from = wow(1386.2, -169.719, 131.0);
    let owner = wow(1551.33, -257.688, 126.01431);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((owner, InMap::continent(2521)));
    h.app.world_mut().entity_mut(unit).insert((
        from,
        InMap::continent(2521),
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
    let wdts = HashMap::from([(2521, maps.map(2521).unwrap().wdt_file_data_id)]);
    h.app.insert_resource(GroundMap::new(&data, wdts.clone()));
    h.app
        .insert_resource(crate::navigation::Navigation::new(&ground.join("navmesh")));
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    let mut support = GroundMap::new(&data, wdts);
    let mut los = los::LosMap::new(&ground.join("los/2521"));
    let mut previous = from;
    for tick in 0..1500 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            los.clear(point(previous) + Vec3::Y * 2.0, point(at) + Vec3::Y * 2.0)
                .unwrap(),
            "Ruby follower tick{tick} crosses collision: {previous:?}->{at:?}"
        );
        let GroundProbe::Supported(floor) = support.creature_ground_at(2521, point(at)) else {
            let samples = [0.0, 1.0, 2.0, 5.0, 10.0].map(|offset| {
                (
                    offset,
                    support.creature_ground_at(2521, point(at) + Vec3::Y * offset),
                )
            });
            panic!(
                "unsupported Ruby follower tick{tick} feet {at:?}; previous{previous:?}; raised support samples{samples:?}"
            )
        };
        assert!(
            (at.y - floor).abs() < 0.05,
            "Ruby follower tick{tick} must follow exact WMO/M2 support: {at:?} floor{floor}"
        );
        previous = at;
    }
    let gap = (owner.x - previous.x).hypot(owner.z - previous.z);
    assert!(
        (gap - 4.0).abs() < 0.1,
        "Ruby follower must complete regroup across the metalplate: gap{gap} at{previous:?}"
    );
    eprintln!("Ruby actual grounded follower regroup: {from:?}->{previous:?} gap{gap}");
}

#[test]
#[ignore = "requires FOLLOWER_RUBY_DATA authored local WMO assets"]
fn ruby_creature_height_supports_steep_step_without_changing_player_floor_policy() {
    let data = PathBuf::from(std::env::var_os("FOLLOWER_RUBY_DATA").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut ground = GroundMap::new(
        &data,
        HashMap::from([(2521, maps.map(2521).unwrap().wdt_file_data_id)]),
    );
    // Actual WMO4391460 face1016: world-up normal0.564409, collision height128.1038.
    let feet = Vec3::new(1405.7883, 127.84611, 173.86066);
    assert_eq!(ground.ground_at(2521, feet), GroundProbe::Unsupported);
    let GroundProbe::Supported(height) = ground.creature_ground_at(2521, feet) else {
        panic!("native creature map-height query must support the actual WMO step")
    };
    assert!(
        (height - 128.1038).abs() < 0.01,
        "actual WMO collision height: {height}"
    );
    assert_eq!(
        ground.ground_at(2521, feet),
        GroundProbe::Unsupported,
        "creature sampling must not change the player slope filter"
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

#[test]
#[ignore = "requires prepared follower eye/nav bake; set GAME_SERVER_GROUND_DIR"]
fn follower_healer_walks_through_actual_stockade_doorway_before_healing() {
    let engine = crate::ground::ground_data_dir();
    // Stockwatch42 command-00535.log: Crenna stopped here while Garrick was
    // injured inside the cell. A stationary owner patient isolates this same
    // doorway/range boundary without another follower moving the recorded point.
    let from = Position {
        x: 105.6,
        y: -26.2,
        z: 3.2,
    };
    let patient = Position {
        x: 106.1,
        y: -26.5,
        z: 17.6,
    };
    let (mut h, healer, profile) = spawn_profile_fixture(209072);
    h.app.world_mut().despawn(h.npc);
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    h.app
        .world_mut()
        .get_mut::<shared::components::Level>(h.caster)
        .unwrap()
        .0 = 32;
    super::spawning::initialize(h.app.world_mut(), h.caster, healer, profile).unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let display: u32 = db.query_row(
        "SELECT CreatureDisplayID FROM content_creature_template_model WHERE CreatureID=209072 ORDER BY Idx LIMIT 1",
        [], |row| row.get(0),
    ).unwrap();
    h.app.world_mut().entity_mut(healer).insert((
        from,
        InMap::continent(34),
        shared::components::ModelDisplay {
            display_id: display,
        },
        shared::components::Rotation {
            x: 0.0,
            y: crate::creature_ai::facing_along(patient.x - from.x, patient.z - from.z).unwrap(),
            z: 0.0,
        },
        crate::creature_ai::SpawnPoint {
            x: from.x,
            y: from.y,
            z: from.z,
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 1.0,
        },
    ));
    h.app.world_mut().entity_mut(h.caster).insert((
        patient,
        InMap::continent(34),
        shared::components::Health {
            current: 4979.0,
            max: 7840.0,
        },
    ));
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    h.app.insert_resource(GroundMap::new(
        &engine,
        HashMap::from([(34, maps.map(34).unwrap().wdt_file_data_id)]),
    ));
    h.app
        .insert_resource(crate::navigation::Navigation::new(&engine.join("navmesh")));
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&engine.join("los")).unwrap());
    crate::creature_ai::install_follower_follow_fixture(&mut h.app);
    {
        let world = h.app.world_mut();
        let mut bodies = world.query::<crate::line_of_sight::BodyParts>();
        let sight = world.resource::<crate::line_of_sight::LineOfSight>();
        let healer_body = sight
            .body(bodies.get(world, healer).unwrap())
            .expect("sourced Crenna eye row");
        let patient_body = sight
            .body(bodies.get(world, h.caster).unwrap())
            .expect("sourced Human eye row");
        assert!(
            sight
                .witness(InMap::continent(34), &healer_body, &patient_body)
                .is_none(),
            "recorded doorway must obstruct authoritative eyes, chest and feet rays"
        );
    }
    let mut collision = los::LosMap::new(&engine.join("los/34"));
    let mut previous = from;
    for tick in 0..200 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(healer).unwrap();
        assert!(
            collision
                .clear(point(previous) + Vec3::Y, point(at) + Vec3::Y)
                .unwrap(),
            "healer tick{tick} crossed authored collision: {previous:?}->{at:?}"
        );
        previous = at;
        if h.inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(8936)
                    && event.amount > 0
                    && event.kind == shared::protocol::CombatLogKind::Heal
            })
            .any(|event| {
                event.source == Some(healer.to_bits()) && event.target == Some(h.caster.to_bits())
            })
        {
            assert!(
                h.health(h.caster) > 4979.0,
                "ordinary Regrowth must reach the patient"
            );
            let GroundProbe::Supported(floor) = h
                .app
                .world_mut()
                .resource_mut::<GroundMap>()
                .creature_ground_at(34, point(at))
            else {
                panic!("healer feet lost authored support: {at:?}")
            };
            assert!(
                (at.y - floor).abs() < 0.05,
                "healer must stay on the actual floor"
            );
            return;
        }
    }
    panic!(
        "in-range Crenna remained unable to heal through the doorway: {from:?}->{previous:?}; {:?}",
        h.inbox.log
    );
}
