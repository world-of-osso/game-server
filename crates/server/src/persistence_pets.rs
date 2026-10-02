//! Per-character hunter pet stable (TrinityCore `character_pet` rows and `PetStable`).
use redb::{Database, ReadableDatabase, TableDefinition};

use crate::tame::MAX_ACTIVE_PETS;

/// character_id (u64) -> bitcode `StoredPetStable`. No row: the hunter never had a pet
/// (`Player::GetPetStable` is null).
pub(crate) const PET_STABLES: TableDefinition<u64, &[u8]> = TableDefinition::new("pet_stables");

/// One stored hunter pet (`PetStable::PetInfo`).
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct StoredPet {
    /// `CreatureId`: the tamed creature's template entry.
    pub creature_entry: u32,
    /// `DisplayId`: the tamed creature's model.
    pub display_id: u32,
    pub name: String,
    pub level: u8,
    /// `CreatureFamily` ID of the template.
    pub family: u32,
}

/// `PetStable`: the active (Call Pet) slots and which of them is out (`CurrentPetIndex`).
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Default)]
pub struct StoredPetStable {
    pub current: Option<u8>,
    pub active: [Option<StoredPet>; MAX_ACTIVE_PETS],
}

pub fn load_pet_stable(db: &Database, character_id: u64) -> Option<StoredPetStable> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(PET_STABLES) else {
        return None;
    };
    table
        .get(character_id)
        .expect("Failed to read pet stable")
        .map(|guard| bitcode::decode(guard.value()).expect("Failed to decode pet stable"))
}

pub fn save_pet_stable(db: &Database, character_id: u64, stable: &StoredPetStable) {
    let bytes = bitcode::encode(stable);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(PET_STABLES)
            .expect("Failed to open pet_stables table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to save pet stable");
    }
    txn.commit().expect("Failed to commit pet stable");
}
