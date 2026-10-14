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
fn pet_journal_request_checks_authenticated_account_ownership_and_keeps_injuries() {
    use bevy::prelude::*;
    use shared::protocol::SetBattlePetAbility;
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("ownership.redb")).unwrap();
    let owned = journal(25);
    let owned_id = owned.pets[0].id;
    let foreign_id = owned_id + 1;
    let mut other = PetJournal::default();
    other
        .add_with_guid(39, 25, PetQuality::Rare, 3, foreign_id)
        .unwrap();
    crate::battle_pets::save_journal(&db, 42, &owned);
    crate::battle_pets::save_journal(&db, 43, &other);
    let mut world = World::new();
    world.insert_resource(crate::persistence::GameDatabase::new(db));
    world.insert_resource(crate::battle_pets::load_test_catalog());
    let mut owners = Vec::new();
    for account in [42, 43] {
        let owner = world
            .spawn(crate::collection::default_collection_state())
            .id();
        world.spawn((
            crate::auth::Authenticated {
                account_id: account,
            },
            crate::auth::InWorld {
                character_id: account,
            },
            crate::networking::PlayerLink(owner),
        ));
        owners.push(owner);
    }
    crate::battle_pets::load_account_journals(&mut world);
    let mut injured =
        load_states(&world.resource::<crate::persistence::GameDatabase>().db, 42).unwrap();
    injured.get_mut(&owned_id).unwrap().health = 73;
    save_states(
        &world.resource::<crate::persistence::GameDatabase>().db,
        42,
        &injured,
    )
    .unwrap();
    let before = load_states(&world.resource::<crate::persistence::GameDatabase>().db, 43).unwrap();
    assert!(
        crate::battle_pets::choose_ability(
            &mut world,
            owners[0],
            SetBattlePetAbility {
                pet_id: foreign_id,
                slot: 1,
                ability_id: 202
            }
        )
        .is_err()
    );
    assert_eq!(
        load_states(&world.resource::<crate::persistence::GameDatabase>().db, 43).unwrap(),
        before
    );
    crate::battle_pets::choose_ability(
        &mut world,
        owners[0],
        SetBattlePetAbility {
            pet_id: owned_id,
            slot: 1,
            ability_id: 202,
        },
    )
    .unwrap();
    let saved = load_states(&world.resource::<crate::persistence::GameDatabase>().db, 42).unwrap();
    assert_eq!(saved[&owned_id].health, 73);
    assert_eq!(saved[&owned_id].selected_abilities[0], 202);
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
