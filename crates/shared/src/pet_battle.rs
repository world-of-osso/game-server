use serde::{Deserialize, Serialize};

/// Pet quality tier (affects stat scaling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PetQuality {
    Poor = 0,
    Common = 1,
    Uncommon = 2,
    Rare = 3,
    Epic = 4,
    Legendary = 5,
}

impl PetQuality {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Poor),
            1 => Some(Self::Common),
            2 => Some(Self::Uncommon),
            3 => Some(Self::Rare),
            4 => Some(Self::Epic),
            5 => Some(Self::Legendary),
            _ => None,
        }
    }
}

/// Maximum pet level.
pub const MAX_PET_LEVEL: u8 = 25;
/// Maximum pets per account journal.
pub const MAX_JOURNAL_SIZE: usize = 1000;
/// Maximum duplicates of the same species.
pub const MAX_PER_SPECIES: usize = 3;

/// A single pet instance owned by an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedPet {
    /// Unique pet instance ID.
    pub id: u64,
    /// Species template ID (which kind of pet).
    pub species_id: u32,
    /// Pet's custom name (or species default if None).
    pub custom_name: Option<String>,
    /// Current level (1–25).
    pub level: u8,
    /// Quality tier.
    pub quality: PetQuality,
    /// Current XP toward next level.
    pub xp: u32,
}

/// Why a pet journal operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetJournalError {
    /// Journal is full (1000 pets max).
    JournalFull,
    /// Already have 3 of this species.
    SpeciesLimitReached,
    /// Pet not found in journal.
    PetNotFound,
    /// Name is empty.
    EmptyName,
}

/// Account-wide pet collection.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PetJournal {
    pub pets: Vec<OwnedPet>,
    next_id: u64,
}

impl PetJournal {
    /// Add a new pet to the journal. Returns the assigned pet ID.
    pub fn add(
        &mut self,
        species_id: u32,
        level: u8,
        quality: PetQuality,
    ) -> Result<u64, PetJournalError> {
        if self.pets.len() >= MAX_JOURNAL_SIZE {
            return Err(PetJournalError::JournalFull);
        }
        let species_count = self.count_species(species_id);
        if species_count >= MAX_PER_SPECIES {
            return Err(PetJournalError::SpeciesLimitReached);
        }
        self.next_id += 1;
        let id = self.next_id;
        self.pets.push(OwnedPet {
            id,
            species_id,
            custom_name: None,
            level: level.clamp(1, MAX_PET_LEVEL),
            quality,
            xp: 0,
        });
        Ok(id)
    }

    /// Remove a pet from the journal (release).
    pub fn remove(&mut self, pet_id: u64) -> Result<(), PetJournalError> {
        let idx = self
            .pets
            .iter()
            .position(|p| p.id == pet_id)
            .ok_or(PetJournalError::PetNotFound)?;
        self.pets.remove(idx);
        Ok(())
    }

    /// Rename a pet.
    pub fn rename(&mut self, pet_id: u64, name: String) -> Result<(), PetJournalError> {
        if name.is_empty() {
            return Err(PetJournalError::EmptyName);
        }
        let pet = self
            .pets
            .iter_mut()
            .find(|p| p.id == pet_id)
            .ok_or(PetJournalError::PetNotFound)?;
        pet.custom_name = Some(name);
        Ok(())
    }

    /// Get a pet by ID.
    pub fn get(&self, pet_id: u64) -> Option<&OwnedPet> {
        self.pets.iter().find(|p| p.id == pet_id)
    }

    /// Number of pets in the journal.
    pub fn count(&self) -> usize {
        self.pets.len()
    }

    /// Number of unique species collected.
    pub fn unique_species(&self) -> usize {
        let mut species: Vec<u32> = self.pets.iter().map(|p| p.species_id).collect();
        species.sort();
        species.dedup();
        species.len()
    }

    /// Count how many of a specific species are owned.
    pub fn count_species(&self, species_id: u32) -> usize {
        self.pets.iter().filter(|p| p.species_id == species_id).count()
    }

    /// All pets of a given species.
    pub fn by_species(&self, species_id: u32) -> Vec<&OwnedPet> {
        self.pets.iter().filter(|p| p.species_id == species_id).collect()
    }

    /// Highest-level pet in the journal.
    pub fn highest_level(&self) -> u8 {
        self.pets.iter().map(|p| p.level).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_from_id() {
        assert_eq!(PetQuality::from_id(0), Some(PetQuality::Poor));
        assert_eq!(PetQuality::from_id(3), Some(PetQuality::Rare));
        assert_eq!(PetQuality::from_id(5), Some(PetQuality::Legendary));
        assert_eq!(PetQuality::from_id(6), None);
    }

    #[test]
    fn add_pet() {
        let mut journal = PetJournal::default();
        let id = journal.add(100, 1, PetQuality::Common).unwrap();
        assert_eq!(journal.count(), 1);
        let pet = journal.get(id).unwrap();
        assert_eq!(pet.species_id, 100);
        assert_eq!(pet.level, 1);
        assert_eq!(pet.quality, PetQuality::Common);
        assert!(pet.custom_name.is_none());
    }

    #[test]
    fn add_pet_clamps_level() {
        let mut journal = PetJournal::default();
        let id = journal.add(1, 50, PetQuality::Rare).unwrap();
        assert_eq!(journal.get(id).unwrap().level, 25); // clamped to max

        let id2 = journal.add(2, 0, PetQuality::Rare).unwrap();
        assert_eq!(journal.get(id2).unwrap().level, 1); // clamped to min
    }

    #[test]
    fn add_pet_unique_ids() {
        let mut journal = PetJournal::default();
        let id1 = journal.add(100, 1, PetQuality::Common).unwrap();
        let id2 = journal.add(100, 1, PetQuality::Common).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn species_limit() {
        let mut journal = PetJournal::default();
        journal.add(100, 1, PetQuality::Common).unwrap();
        journal.add(100, 1, PetQuality::Uncommon).unwrap();
        journal.add(100, 1, PetQuality::Rare).unwrap();
        assert_eq!(
            journal.add(100, 1, PetQuality::Epic),
            Err(PetJournalError::SpeciesLimitReached)
        );
        // Different species still works
        journal.add(200, 1, PetQuality::Common).unwrap();
    }

    #[test]
    fn remove_pet() {
        let mut journal = PetJournal::default();
        let id = journal.add(100, 1, PetQuality::Common).unwrap();
        journal.remove(id).unwrap();
        assert_eq!(journal.count(), 0);
        assert!(journal.get(id).is_none());
    }

    #[test]
    fn remove_nonexistent() {
        let mut journal = PetJournal::default();
        assert_eq!(journal.remove(999), Err(PetJournalError::PetNotFound));
    }

    #[test]
    fn rename_pet() {
        let mut journal = PetJournal::default();
        let id = journal.add(100, 1, PetQuality::Common).unwrap();
        journal.rename(id, "Fluffy".into()).unwrap();
        assert_eq!(
            journal.get(id).unwrap().custom_name,
            Some("Fluffy".into())
        );
    }

    #[test]
    fn rename_empty_fails() {
        let mut journal = PetJournal::default();
        let id = journal.add(100, 1, PetQuality::Common).unwrap();
        assert_eq!(journal.rename(id, "".into()), Err(PetJournalError::EmptyName));
    }

    #[test]
    fn rename_nonexistent() {
        let mut journal = PetJournal::default();
        assert_eq!(
            journal.rename(999, "Test".into()),
            Err(PetJournalError::PetNotFound)
        );
    }

    #[test]
    fn unique_species_count() {
        let mut journal = PetJournal::default();
        journal.add(100, 1, PetQuality::Common).unwrap();
        journal.add(100, 1, PetQuality::Rare).unwrap();
        journal.add(200, 1, PetQuality::Common).unwrap();
        assert_eq!(journal.unique_species(), 2);
    }

    #[test]
    fn by_species() {
        let mut journal = PetJournal::default();
        journal.add(100, 1, PetQuality::Common).unwrap();
        journal.add(200, 5, PetQuality::Rare).unwrap();
        journal.add(100, 10, PetQuality::Uncommon).unwrap();
        let species_100 = journal.by_species(100);
        assert_eq!(species_100.len(), 2);
        assert!(species_100.iter().all(|p| p.species_id == 100));
    }

    #[test]
    fn highest_level_empty() {
        let journal = PetJournal::default();
        assert_eq!(journal.highest_level(), 0);
    }

    #[test]
    fn highest_level() {
        let mut journal = PetJournal::default();
        journal.add(100, 5, PetQuality::Common).unwrap();
        journal.add(200, 25, PetQuality::Rare).unwrap();
        journal.add(300, 10, PetQuality::Common).unwrap();
        assert_eq!(journal.highest_level(), 25);
    }

    #[test]
    fn serialization_round_trip() {
        let mut journal = PetJournal::default();
        journal.add(100, 5, PetQuality::Rare).unwrap();
        journal.add(200, 10, PetQuality::Epic).unwrap();

        let json = serde_json::to_string(&journal).unwrap();
        let restored: PetJournal = serde_json::from_str(&json).unwrap();
        assert_eq!(journal, restored);
    }

    #[test]
    fn remove_frees_species_slot() {
        let mut journal = PetJournal::default();
        let id1 = journal.add(100, 1, PetQuality::Common).unwrap();
        journal.add(100, 1, PetQuality::Common).unwrap();
        journal.add(100, 1, PetQuality::Common).unwrap();
        // Full on species 100
        assert!(journal.add(100, 1, PetQuality::Common).is_err());
        // Release one
        journal.remove(id1).unwrap();
        // Can add again
        journal.add(100, 1, PetQuality::Common).unwrap();
    }
}
