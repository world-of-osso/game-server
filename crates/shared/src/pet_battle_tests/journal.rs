use super::*;

#[test]
fn quality_from_id() {
    assert_eq!(PetQuality::from_id(0), Some(PetQuality::Poor));
    assert_eq!(PetQuality::from_id(1), Some(PetQuality::Common));
    assert_eq!(PetQuality::from_id(2), Some(PetQuality::Uncommon));
    assert_eq!(PetQuality::from_id(3), Some(PetQuality::Rare));
    assert_eq!(PetQuality::from_id(4), Some(PetQuality::Epic));
    assert_eq!(PetQuality::from_id(5), Some(PetQuality::Legendary));
    assert_eq!(PetQuality::from_id(6), None);
}

#[test]
fn add_pet() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 5, PetQuality::Common).unwrap();
    assert_eq!(journal.count(), 1);
    let pet = journal.get(id).unwrap();
    assert_eq!(pet.species_id, 1);
    assert_eq!(pet.level, 5);
    assert_eq!(pet.quality, PetQuality::Common);
}

#[test]
fn add_pet_clamps_level() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 99, PetQuality::Common).unwrap();
    let pet = journal.get(id).unwrap();
    assert_eq!(pet.level, MAX_PET_LEVEL);
}

#[test]
fn add_pet_unique_ids() {
    let mut journal = PetJournal::default();
    let id1 = journal.add(1, 1, PetQuality::Common).unwrap();
    let id2 = journal.add(2, 1, PetQuality::Common).unwrap();
    assert_ne!(id1, id2);
}

#[test]
fn species_limit() {
    let mut journal = PetJournal::default();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    let result = journal.add(42, 1, PetQuality::Common);
    assert_eq!(result, Err(PetJournalError::SpeciesLimitReached));
}

#[test]
fn remove_pet() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 1, PetQuality::Common).unwrap();
    journal.remove(id).unwrap();
    assert_eq!(journal.count(), 0);
    assert!(journal.get(id).is_none());
}

#[test]
fn remove_nonexistent() {
    let mut journal = PetJournal::default();
    let result = journal.remove(999);
    assert_eq!(result, Err(PetJournalError::PetNotFound));
}

#[test]
fn rename_pet() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 1, PetQuality::Common).unwrap();
    journal.rename(id, "Fluffy".into()).unwrap();
    let pet = journal.get(id).unwrap();
    assert_eq!(pet.custom_name, Some("Fluffy".into()));
}

#[test]
fn rename_empty_fails() {
    let mut journal = PetJournal::default();
    let id = journal.add(1, 1, PetQuality::Common).unwrap();
    let result = journal.rename(id, "".into());
    assert_eq!(result, Err(PetJournalError::EmptyName));
}

#[test]
fn rename_nonexistent() {
    let mut journal = PetJournal::default();
    let result = journal.rename(999, "Name".into());
    assert_eq!(result, Err(PetJournalError::PetNotFound));
}

#[test]
fn unique_species_count() {
    let mut journal = PetJournal::default();
    journal.add(1, 1, PetQuality::Common).unwrap();
    journal.add(1, 1, PetQuality::Common).unwrap();
    journal.add(2, 1, PetQuality::Common).unwrap();
    assert_eq!(journal.unique_species(), 2);
}

#[test]
fn by_species() {
    let mut journal = PetJournal::default();
    journal.add(1, 1, PetQuality::Common).unwrap();
    journal.add(1, 2, PetQuality::Rare).unwrap();
    journal.add(2, 1, PetQuality::Common).unwrap();
    let pets = journal.by_species(1);
    assert_eq!(pets.len(), 2);
    assert!(pets.iter().all(|p| p.species_id == 1));
}

#[test]
fn highest_level_empty() {
    let journal = PetJournal::default();
    assert_eq!(journal.highest_level(), 0);
}

#[test]
fn highest_level() {
    let mut journal = PetJournal::default();
    journal.add(1, 5, PetQuality::Common).unwrap();
    journal.add(2, 20, PetQuality::Common).unwrap();
    journal.add(3, 12, PetQuality::Common).unwrap();
    assert_eq!(journal.highest_level(), 20);
}

#[test]
fn serialization_round_trip() {
    let mut journal = PetJournal::default();
    let id = journal.add(7, 15, PetQuality::Rare).unwrap();
    journal.rename(id, "Sparky".into()).unwrap();
    let json = serde_json::to_string(&journal).unwrap();
    let restored: PetJournal = serde_json::from_str(&json).unwrap();
    assert_eq!(journal, restored);
}

#[test]
fn remove_frees_species_slot() {
    let mut journal = PetJournal::default();
    let id1 = journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    journal.remove(id1).unwrap();
    journal.add(42, 1, PetQuality::Common).unwrap();
    assert_eq!(journal.count_species(42), 3);
}
