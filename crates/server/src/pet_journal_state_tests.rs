use crate::pet_journal_state::{OwnedPetBattleState, load_states, save_states};
use server::pet_battle::Catalog;
use shared::pet_battle::{PetJournal, PetQuality};
use std::collections::BTreeMap;

fn catalog() -> Catalog {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    Catalog::load_world_db(&db).unwrap()
}
fn journal(level: u8) -> PetJournal {
    let mut journal = PetJournal::default();
    journal
        .add_with_guid(39, level, PetQuality::Rare, 3, (1_u64 << 42) + 17)
        .unwrap();
    journal
}

#[test]
fn pet_journal_state_preserves_real_injury_and_chosen_ability_after_database_reopen() {
    let catalog = catalog();
    let journal = journal(25);
    let pet = &journal.pets[0];
    let mut state = OwnedPetBattleState::initial(&catalog, pet).unwrap();
    assert_eq!(state.health, 1563);
    assert_eq!(state.selected_abilities, [384, 389, 459]);
    state.health = 73;
    state.select_ability(&catalog, pet, 1, 202).unwrap();
    let states = BTreeMap::from([(pet.id, state.clone())]);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("owned-state.redb");
    {
        let db = redb::Database::create(&path).unwrap();
        save_states(&db, 42, &states).unwrap();
    }
    let db = redb::Database::open(&path).unwrap();
    let restored = load_states(&db, 42).unwrap();
    assert_eq!(restored[&pet.id].health, 73);
    assert_eq!(restored[&pet.id].selected_abilities, [202, 389, 459]);
    assert!(load_states(&db, 43).unwrap().is_empty());
}

#[test]
fn pet_journal_state_rejects_locked_foreign_and_invalid_ability_slots_without_mutation() {
    let catalog = catalog();
    let journal = journal(1);
    let pet = &journal.pets[0];
    let mut state = OwnedPetBattleState::initial(&catalog, pet).unwrap();
    let before = state.clone();
    for (slot, ability) in [(0, 384), (4, 384), (1, 110), (1, 202), (2, 389)] {
        assert!(state.select_ability(&catalog, pet, slot, ability).is_err());
        assert_eq!(state, before);
    }
}
