use super::*;
use crate::persistence;

#[test]
fn summon_mount_marks_only_selected_mount_active() {
    let mut collection = PlayerCollectionState {
        mounts: vec![
            CollectionMount {
                mount_id: 101,
                name: "Swift Brown Steed".into(),
                known: true,
                active: false,
            },
            CollectionMount {
                mount_id: 102,
                name: "Black War Wolf".into(),
                known: true,
                active: true,
            },
        ],
        pets: vec![],
    };

    let message = summon_mount(&mut collection, 101).expect("summon mount");

    assert_eq!(message, "summoned Swift Brown Steed");
    assert!(collection.mounts[0].active);
    assert!(!collection.mounts[1].active);
}

#[test]
fn dismiss_pet_clears_active_pet() {
    let mut collection = PlayerCollectionState {
        mounts: vec![],
        pets: vec![CollectionPet {
            pet_id: 202,
            name: "Brown Rabbit".into(),
            known: true,
            active: true,
        }],
    };

    let message = dismiss_pet(&mut collection).expect("dismiss pet");

    assert_eq!(message, "dismissed Brown Rabbit");
    assert!(!collection.pets[0].active);
}

#[test]
fn collection_state_round_trips_through_persistence() {
    let db = persistence::temp_database();
    let collection = PlayerCollectionState {
        mounts: vec![CollectionMount {
            mount_id: 101,
            name: "Swift Brown Steed".into(),
            known: true,
            active: true,
        }],
        pets: vec![CollectionPet {
            pet_id: 202,
            name: "Brown Rabbit".into(),
            known: true,
            active: false,
        }],
    };

    persistence::save_collections(&db, 42, &collection);
    let loaded = persistence::load_collections(&db, 42);

    assert_eq!(loaded, collection);
}

#[test]
fn summon_pet_marks_only_selected_pet_active() {
    let mut collection = PlayerCollectionState {
        mounts: vec![],
        pets: vec![
            CollectionPet {
                pet_id: 202,
                name: "Brown Rabbit".into(),
                known: true,
                active: false,
            },
            CollectionPet {
                pet_id: 203,
                name: "Blue Moth".into(),
                known: true,
                active: true,
            },
        ],
    };

    let message = summon_pet(&mut collection, 202).expect("summon pet");

    assert_eq!(message, "summoned Brown Rabbit");
    assert!(collection.pets[0].active);
    assert!(!collection.pets[1].active);
}

#[test]
fn dismiss_mount_clears_active_mount() {
    let mut collection = PlayerCollectionState {
        mounts: vec![CollectionMount {
            mount_id: 101,
            name: "Swift Brown Steed".into(),
            known: true,
            active: true,
        }],
        pets: vec![],
    };

    let message = dismiss_mount(&mut collection).expect("dismiss mount");

    assert_eq!(message, "dismissed Swift Brown Steed");
    assert!(!collection.mounts[0].active);
}
