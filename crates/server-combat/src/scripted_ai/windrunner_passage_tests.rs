use super::*;
use crate::maps::{DungeonDifficulty, InstanceManager};
use shared::components::Health;

#[test]
fn windrunner_passages_require_previous_boss_and_fly_owner_followers_and_pets() {
    for passage in PASSAGES {
        check_passage(passage);
    }
}

fn check_passage(passage: &Passage) {
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
            MAP,
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
    let expected = network.scripted_path(passage.path).expect("native path");
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
        assert_eq!(*h.app.world().get::<InMap>(unit).unwrap(), copy);
    }
    assert!(
        h.app
            .world()
            .get::<crate::taxi::TaxiFlight>(distant)
            .is_none()
    );
}
