//! Real navigation and production follower movement across a registered pyre.
use bevy::prelude::*;
use shared::components::Position;

#[test]
fn petalwing_requires_both_bosses_and_follower_starts_its_own_flight() {
    use crate::scripted_ai::instance_script::{self, EncounterState};
    let from = Position {
        x: 1277.99475,
        y: 80.09892,
        z: -1670.79858,
    };
    let (mut h, follower, _) = super::navigation_tests::stockade_fixture(from);
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut instances = crate::maps::InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            2859,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 23,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_792_454_400,
            },
        )
        .unwrap()
        .map;
    let network = crate::taxi::load_taxi_network(&db).unwrap();
    let expected = network.scripted_path(10799).unwrap();
    let destination = *expected.last().unwrap();
    h.app.insert_resource(network);
    h.app.insert_resource(maps);
    h.app.insert_resource(instances);
    let owner = h.caster;
    let rotation = shared::components::Rotation {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((copy, from, rotation));
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert((copy, from, rotation));
    let bird = h
        .app
        .world_mut()
        .spawn((
            copy,
            from,
            shared::components::Npc {
                template_id: 247402,
                name: "Orweyna's Petalwing".into(),
            },
        ))
        .id();
    let node = |id, point: Vec3| crate::test_support::graph::Pack {
        id,
        position: [point.x, point.z],
        height: point.y,
        spawns: Vec::new(),
    };
    h.app.insert_resource(crate::test_support::Space(
        crate::test_support::graph::Graph {
            map_id: 2859,
            entrance: [from.x, from.z],
            entrance_height: from.y,
            entrance_pack: 1,
            packs: vec![
                node(1, Vec3::new(from.x, from.y, from.z)),
                node(2, destination),
            ],
            edges: Vec::new(),
            teleports: Vec::new(),
            transports: Vec::new(),
            flights: Vec::new(),
            unreachable: vec![2],
            route: Vec::new(),
            follower_difficulty: 23,
            follower_boss_entries: Vec::new(),
            follower_boss_sources: Default::default(),
            follower_boss_bits: Default::default(),
            follower_encounter_actor_bits: Default::default(),
        },
    ));
    assert!(crate::npc_interaction::petalwing::traverse(h.app.world_mut(), owner, bird).is_err());
    for (index, encounter) in [3199, 3200].into_iter().enumerate() {
        let id =
            instance_script::boss_id_for_encounter(h.app.world_mut(), copy, encounter).unwrap();
        assert!(instance_script::set_boss_state(
            h.app.world_mut(),
            copy,
            id,
            EncounterState::Done
        ));
        if index == 0 {
            assert!(
                crate::npc_interaction::petalwing::traverse(h.app.world_mut(), owner, bird)
                    .is_err()
            );
        }
    }
    crate::npc_interaction::petalwing::traverse(h.app.world_mut(), owner, bird).unwrap();
    assert!(
        h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(owner)
            .is_some()
    );
    assert!(
        h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(follower)
            .is_none()
    );
    let profile = h
        .app
        .world()
        .get::<super::Follower>(follower)
        .unwrap()
        .clone();
    for _ in 0..3 {
        super::transport_follow::update(h.app.world_mut(), follower, &profile);
    }
    let flight = h
        .app
        .world()
        .get::<crate::taxi::TaxiFlight>(follower)
        .expect("follower activates its own ride");
    assert_eq!(flight.points, expected);
    assert_eq!(*h.app.world().get::<Position>(follower).unwrap(), from);
    h.app.add_systems(FixedUpdate, crate::taxi::fly_taxis);
    for _ in 0..400 {
        h.advance(1);
        if h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(follower)
            .is_none()
        {
            break;
        }
    }
    assert!(
        h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(follower)
            .is_none()
    );
    let after = h.app.world().get::<Position>(follower).unwrap();
    assert!(
        Vec3::new(after.x, after.y, after.z).distance(destination) < 0.1,
        "follower must reach the path's final node: {after:?}"
    );
}

#[test]
fn follower_walks_to_and_uses_pyre_after_owner_uses_it() {
    let from = Position {
        x: 4474.46,
        y: 13.525905,
        z: 4918.17,
    };
    let (mut h, follower, engine) = super::navigation_tests::stockade_fixture(from);
    let owner = h.caster;
    let map = crate::maps::InMap::continent(2825);
    let source = Position {
        x: 4480.69,
        y: 13.37,
        z: 4912.04,
    };
    h.app.world_mut().entity_mut(owner).insert((map, source));
    h.app.world_mut().entity_mut(follower).insert((map, from));
    let maps = crate::maps::store::load_map_store(
        &rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap(),
    )
    .unwrap();
    h.app.insert_resource(crate::ground::GroundMap::new(
        &engine,
        std::collections::HashMap::from([(2825, maps.map(2825).unwrap().wdt_file_data_id)]),
    ));
    let pyre = h
        .app
        .world_mut()
        .spawn((
            map,
            source,
            shared::protocol::GameObjectInfo {
                entry: 616377,
                go_type: 10,
                display_id: 0,
                name: "Ethereal Pyre".into(),
                scale: 1.0,
            },
        ))
        .id();
    crate::npc_interaction::nalorakk_pyre::traverse(h.app.world_mut(), owner, pyre).unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(follower).unwrap(),
        from,
        "another unit must not teleport the follower"
    );
    let mut walked = false;
    for _ in 0..1600 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(follower).unwrap();
        if at.y < 30.0
            && Vec3::new(at.x, at.y, at.z).distance(Vec3::new(from.x, from.y, from.z)) > 0.2
        {
            walked = true;
        }
        if at.y > 90.0 {
            break;
        }
    }
    let at = *h.app.world().get::<Position>(follower).unwrap();
    assert!(walked, "follower must walk to the pyre");
    assert!(
        (at.x - 4339.33).abs() < 0.1
            && (at.y - 104.31152).abs() < 0.1
            && (at.z - 4947.34).abs() < 0.1,
        "follower did not use its own pyre interaction: {at:?}"
    );
}
