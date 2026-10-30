use super::*;
use crate::maps::{DungeonDifficulty, InstanceManager};
use shared::components::Health;

#[test]
fn dungeon_passages_require_previous_boss_and_fly_owner_followers_and_pets() {
    for passage in PASSAGES {
        if let PassageKind::Taxi(path) = passage.kind {
            check_passage(passage, path);
        }
    }
}

fn check_passage(passage: &Passage, path: u32) {
    let source = passage.source;
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 8).unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            passage.map,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 205,
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
    let expected = network.scripted_path(path).expect("native path");
    assert!(
        expected[0].distance(source) < 0.01,
        "source must match native node 0"
    );
    h.app.insert_resource(network);
    h.app.insert_resource(maps);
    h.app.insert_resource(instances);
    let owner = h.caster;
    h.app.world_mut().entity_mut(owner).insert((
        copy,
        DungeonDifficulty(205),
        crate::persistence::ClientId(1),
        MovementControl::default(),
        shared::components::Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Position {
            x: source.x,
            y: source.y,
            z: source.z,
        },
    ));
    let distant = crate::spell_cast::test_harness::spawn_player(&mut h.app, 8, 0.0);
    let distant_position = Position {
        x: source.x + RADIUS + 1.0,
        y: source.y,
        z: source.z,
    };
    h.app
        .world_mut()
        .entity_mut(distant)
        .insert((copy, distant_position));
    let follower = h
        .app
        .world_mut()
        .spawn((
            copy,
            shared::components::Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Position {
                x: source.x,
                y: source.y,
                z: source.z,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            crate::followers::Follower {
                owner,
                profile: crate::followers::Profile {
                    entry: 209057,
                    name: "Captain Garrick".into(),
                    class_id: 2,
                    spec_id: 66,
                    role: shared::group::GroupRole::Tank,
                    attack_spell: 0,
                    cooldown_spell: 0,
                    heal_spell: 0,
                    taunt_spell: 0,
                    form_spell: 0,
                    melee: true,
                },
            },
        ))
        .id();
    let pet = h
        .app
        .world_mut()
        .spawn((
            copy,
            shared::components::Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Position {
                x: source.x,
                y: source.y,
                z: source.z,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            crate::hunter_pet::HunterPet {
                owner: follower,
                slot: 0,
            },
        ))
        .id();
    let edge = crate::test_support::graph::TransportEdge {
        from: 1,
        to: 2,
        source_guid: 0,
        radius: RADIUS,
        departure: None,
        landing: expected.last().unwrap().to_array(),
        action: crate::test_support::transport::Action::TaxiProximity,
    };
    assert!(
        crate::test_support::transport::activate(h.app.world_mut(), owner, None, &edge).is_err()
    );
    assert_eq!(
        *h.app.world().get::<Position>(owner).unwrap(),
        Position {
            x: source.x,
            y: source.y,
            z: source.z
        }
    );
    let id =
        instance_script::boss_id_for_encounter(h.app.world_mut(), copy, passage.encounter).unwrap();
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        copy,
        id,
        EncounterState::Done
    ));
    crate::test_support::transport::activate(h.app.world_mut(), owner, None, &edge).unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(distant).unwrap(),
        distant_position
    );
    for unit in [owner, follower, pet] {
        let flight = h
            .app
            .world()
            .get::<crate::taxi::TaxiFlight>(unit)
            .expect("party member boards");
        assert_eq!(flight.points, expected);
        assert!(
            h.app
                .world()
                .get::<shared::components::Mounted>(unit)
                .is_none()
        );
        assert_eq!(*h.app.world().get::<InMap>(unit).unwrap(), copy);
    }
    assert!(
        h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(distant)
            .is_none()
    );
}

#[test]
#[ignore = "requires FOLLOWER_COMPONENT_ASSETS and FOLLOWER_NAV_CANONICAL Floodgate assets"]
fn floodgate_drop_requires_swampface_and_moves_party_from_edge_to_geezle() {
    use crate::navigation::collision_drop::CollisionDropMovement;
    use crate::scripted_ai::fall_movement::{self, FallMovement};
    use bevy::ecs::system::RunSystemOnce;
    let assets = std::path::PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_ASSETS").unwrap());
    let nav_root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[], 8).unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut instances = InstanceManager::default();
    let copy = instances
        .create_map(
            &maps,
            2773,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 205,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_792_454_400,
            },
        )
        .unwrap()
        .map;
    h.app
        .insert_resource(maps)
        .insert_resource(instances)
        .insert_resource(crate::ground::GroundMap::new(
            &assets,
            std::collections::HashMap::from([(2773, 5_839_271)]),
        ));
    crate::navigation::install_fixture(&mut h.app, crate::navigation::Navigation::new(&nav_root));
    let owner = h.caster;
    let start = Position {
        x: 1531.0,
        y: 334.14,
        z: 2746.0,
    };
    h.app.world_mut().entity_mut(owner).insert((
        copy,
        DungeonDifficulty(205),
        crate::persistence::ClientId(1),
        MovementControl::default(),
        start,
    ));
    let mut party = vec![owner];
    for index in 0..4 {
        let follower = h
            .app
            .world_mut()
            .spawn((
                copy,
                start,
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                crate::followers::Follower {
                    owner,
                    profile: crate::followers::Profile {
                        entry: 209057 + index,
                        name: format!("Follower {index}"),
                        class_id: 2,
                        spec_id: 66,
                        role: shared::group::GroupRole::Tank,
                        attack_spell: 0,
                        cooldown_spell: 0,
                        heal_spell: 0,
                        taunt_spell: 0,
                        form_spell: 0,
                        melee: true,
                    },
                },
            ))
            .id();
        party.push(follower);
    }
    let pet = h
        .app
        .world_mut()
        .spawn((
            copy,
            start,
            Health {
                current: 100.0,
                max: 100.0,
            },
            crate::hunter_pet::HunterPet {
                owner: party[1],
                slot: 0,
            },
        ))
        .id();
    party.push(pet);
    update(h.app.world_mut());
    for &unit in &party {
        assert!(
            h.app.world().get::<CollisionDropMovement>(unit).is_none(),
            "not-Done must not start a drop"
        );
        assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), start);
    }
    let id = instance_script::boss_id_for_encounter(h.app.world_mut(), copy, 3053).unwrap();
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        copy,
        id,
        EncounterState::Done
    ));
    update(h.app.world_mut());
    for &unit in &party {
        assert!(
            h.app.world().get::<CollisionDropMovement>(unit).is_some(),
            "party member did not start"
        );
    }
    let edge = Vec3::new(1529.167, 334.6686, 2746.0);
    let landing = Vec3::new(1524.25, 275.31427, 2746.25);
    let mut fall_started = std::collections::HashSet::new();
    let mut previous: Vec<_> = party
        .iter()
        .map(|_| Vec3::new(start.x, start.y, start.z))
        .collect();
    let mut collision = los::LosMap::new(&assets.join("los/2773"));
    for _ in 0..400 {
        h.app
            .world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        crate::navigation::drain_fixture(&mut h.app);
        crate::navigation::collision_drop::advance_collision_drops(h.app.world_mut());
        for (i, &unit) in party.iter().enumerate() {
            if h.app.world().get::<FallMovement>(unit).is_some() && fall_started.insert(unit) {
                let pos = h.app.world().get::<Position>(unit).unwrap();
                let at = Vec3::new(pos.x, pos.y, pos.z);
                assert!(
                    at.distance(edge) < 0.3,
                    "fall must begin on supported edge: {at:?}"
                );
            }
            let pos = h.app.world().get::<Position>(unit).unwrap();
            let next = Vec3::new(pos.x, pos.y, pos.z);
            assert!(
                next.distance(previous[i]) < 3.1,
                "unexpected movement {previous:?} -> {next:?}"
            );
            if fall_started.contains(&unit) && next.x < edge.x - 0.05 {
                assert!(next.y < edge.y, "must fall while leaving support: {next:?}");
            }
            previous[i] = next;
        }
        let before = previous.clone();
        fall_movement::advance_falls(h.app.world_mut());
        for (i, &unit) in party.iter().enumerate() {
            let pos = h.app.world().get::<Position>(unit).unwrap();
            let next = Vec3::new(pos.x, pos.y, pos.z);
            if fall_started.contains(&unit) && next != before[i] {
                assert!(
                    collision
                        .clear(before[i] + Vec3::Y, next + Vec3::Y)
                        .unwrap(),
                    "fall crossed collision: {:?}->{next:?}",
                    before[i]
                );
                assert!(next.y < before[i].y, "gravity must run during departure");
            }
            previous[i] = next;
        }
        h.app
            .world_mut()
            .run_system_once(fall_movement::release_finished_falls)
            .unwrap();
        if party
            .iter()
            .all(|&unit| h.app.world().get::<CollisionDropMovement>(unit).is_none())
        {
            break;
        }
    }
    assert_eq!(fall_started.len(), party.len());
    let mut nav = ::navigation::NavigationMap::open(&nav_root.join("2773")).unwrap();
    let geezle = Vec3::new(1195.83, 277.058, 2906.88);
    for unit in party {
        let pos = h.app.world().get::<Position>(unit).unwrap();
        let feet = Vec3::new(pos.x, pos.y, pos.z);
        assert!(
            feet.distance(landing) < 0.2,
            "party member landed at {feet:?}"
        );
        assert!(h.app.world().get::<CollisionDropMovement>(unit).is_none());
        assert!(
            nav.complete_path(feet, geezle).is_ok(),
            "landing must route to Geezle"
        );
    }
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(owner)
            .unwrap()
            .controlled
    );
}
