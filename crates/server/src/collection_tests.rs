use super::*;
use crate::persistence;

#[test]
fn mount_journal_new_character_has_no_fabricated_mounts() {
    let db = persistence::temp_database();
    let collection = persistence::load_collections(&db, 42);
    assert!(collection.mounts.is_empty());
}

#[test]
fn collection_state_round_trips_through_persistence() {
    let db = persistence::temp_database();
    let collection = PlayerCollectionState {
        mounts: vec![],
        pets: vec![CollectionPet {
            pet_id: 72,
            name: "Brown Rabbit".into(),
            known: true,
            active: false,
        }],
    };
    persistence::save_collections(&db, 42, &collection);
    assert_eq!(persistence::load_collections(&db, 42), collection);
    let snapshot = build_collection_snapshot(&collection);
    assert_eq!(snapshot.pets[0].name, "Brown Rabbit");
    assert!(snapshot.mounts.is_empty());
}
