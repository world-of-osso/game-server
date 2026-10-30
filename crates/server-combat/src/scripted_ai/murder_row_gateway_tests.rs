use super::*;
use crate::maps::{DungeonDifficulty, InstanceManager};
use shared::components::Health;

#[test]
fn murder_row_gateway_requires_zaen_and_moves_owner_followers_and_pets() {
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
    h.app.insert_resource(maps);
    h.app.insert_resource(instances);
    let owner = h.caster;
    h.app.world_mut().entity_mut(owner).insert((
        copy,
        DungeonDifficulty(205),
        crate::persistence::ClientId(1),
        MovementControl::default(),
        Position {
            x: SOURCE.x,
            y: SOURCE.y,
            z: SOURCE.z,
        },
    ));
    let distant = crate::spell_cast::test_harness::spawn_player(&mut h.app, 8, 0.0);
    let distant_position = Position {
        x: SOURCE.x + RADIUS + 1.0,
        y: SOURCE.y,
        z: SOURCE.z,
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
            Position {
                x: SOURCE.x,
                y: SOURCE.y,
                z: SOURCE.z,
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
            Position {
                x: SOURCE.x,
                y: SOURCE.y,
                z: SOURCE.z,
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
    update(h.app.world_mut());
    assert_eq!(
        *h.app.world().get::<Position>(owner).unwrap(),
        Position {
            x: SOURCE.x,
            y: SOURCE.y,
            z: SOURCE.z
        }
    );
    let id =
        instance_script::boss_id_for_encounter(h.app.world_mut(), copy, ZAEN_ENCOUNTER).unwrap();
    assert!(instance_script::set_boss_state(
        h.app.world_mut(),
        copy,
        id,
        EncounterState::Done
    ));
    update(h.app.world_mut());
    assert_eq!(
        *h.app.world().get::<Position>(distant).unwrap(),
        distant_position
    );
    let expected = landing().bevy_position();
    for unit in [owner, follower, pet] {
        assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), expected);
        assert_eq!(*h.app.world().get::<InMap>(unit).unwrap(), copy);
    }
}
