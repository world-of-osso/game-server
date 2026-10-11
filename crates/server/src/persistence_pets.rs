//! Versioned per-character combat-pet stable. Legacy rows migrate on first read.
use redb::{Database, ReadableDatabase, TableDefinition};

use crate::tame::MAX_ACTIVE_PETS;

/// Pre-lifecycle rows remain read-only migration input.
pub(crate) const PET_STABLES: TableDefinition<u64, &[u8]> = TableDefinition::new("pet_stables");
const PET_STABLES_V2: TableDefinition<u64, &[u8]> = TableDefinition::new("pet_stables_v2");
/// Retail PetConstantsDocumentation.lua: MAX_STABLE_SLOTS (including secondary slot).
pub const MAX_STABLED_PETS: usize = shared::protocol::PET_STABLE_SLOTS;

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct StoredPet {
    pub creature_entry: u32,
    pub display_id: u32,
    pub name: String,
    pub level: u8,
    pub family: u32,
    pub react_state: u32,
    /// None only for a newly created or migrated pet, initialized from actual stats.
    pub health: Option<f32>,
    /// Hunter ChrSpecialization.ID; demons have no hunter specialization.
    pub specialization: Option<u32>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct StoredPetStable {
    pub current: Option<u8>,
    pub active: [Option<StoredPet>; MAX_ACTIVE_PETS],
    pub stabled: Vec<Option<StoredPet>>,
}

impl Default for StoredPetStable {
    fn default() -> Self {
        Self {
            current: None,
            active: Default::default(),
            stabled: vec![None; MAX_STABLED_PETS],
        }
    }
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub(crate) struct LegacyStoredPet {
    pub creature_entry: u32,
    pub display_id: u32,
    pub name: String,
    pub level: u8,
    pub family: u32,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub(crate) struct LegacyStoredPetStable {
    pub current: Option<u8>,
    pub active: [Option<LegacyStoredPet>; MAX_ACTIVE_PETS],
}

#[derive(bitcode::Encode, bitcode::Decode)]
pub(crate) struct LegacyStoredPetWithStance {
    pub creature_entry: u32,
    pub display_id: u32,
    pub name: String,
    pub level: u8,
    pub family: u32,
    pub react_state: u32,
}

#[derive(bitcode::Encode, bitcode::Decode)]
pub(crate) struct LegacyStoredPetStableWithStance {
    pub current: Option<u8>,
    pub active: [Option<LegacyStoredPetWithStance>; MAX_ACTIVE_PETS],
}

fn migrate_legacy(bytes: &[u8]) -> StoredPetStable {
    let legacy = match bitcode::decode::<LegacyStoredPetStableWithStance>(bytes) {
        Ok(stable) => stable,
        Err(stance_error) => {
            let old: LegacyStoredPetStable = bitcode::decode(bytes).unwrap_or_else(|error| {
                panic!("Failed to decode legacy pet stable: {stance_error}; pre-stance: {error}")
            });
            LegacyStoredPetStableWithStance {
                current: old.current,
                active: old.active.map(|pet| {
                    pet.map(|pet| LegacyStoredPetWithStance {
                        creature_entry: pet.creature_entry,
                        display_id: pet.display_id,
                        name: pet.name,
                        level: pet.level,
                        family: pet.family,
                        react_state: shared::protocol::REACT_PASSIVE,
                    })
                }),
            }
        }
    };
    StoredPetStable {
        current: legacy.current,
        active: legacy.active.map(|pet| {
            pet.map(|pet| StoredPet {
                creature_entry: pet.creature_entry,
                display_id: pet.display_id,
                name: pet.name,
                level: pet.level,
                family: pet.family,
                react_state: pet.react_state,
                health: None,
                specialization: None,
            })
        }),
        ..Default::default()
    }
}

pub fn load_pet_stable(db: &Database, character_id: u64) -> Option<StoredPetStable> {
    let legacy = {
        let txn = db.begin_read().expect("Failed to begin pet read txn");
        if let Ok(table) = txn.open_table(PET_STABLES_V2)
            && let Some(row) = table.get(character_id).expect("Failed to read v2 stable")
        {
            return Some(bitcode::decode(row.value()).expect("Failed to decode v2 pet stable"));
        }
        let Ok(table) = txn.open_table(PET_STABLES) else {
            return None;
        };
        let row = table
            .get(character_id)
            .expect("Failed to read legacy stable")?;
        migrate_legacy(row.value())
    };
    // End read transaction before the migration's write transaction.
    save_pet_stable(db, character_id, &legacy);
    Some(legacy)
}

pub fn save_pet_stable(db: &Database, character_id: u64, stable: &StoredPetStable) {
    let bytes = bitcode::encode(stable);
    let txn = db.begin_write().expect("Failed to begin pet write txn");
    {
        let mut table = txn
            .open_table(PET_STABLES_V2)
            .expect("Failed to open pet_stables_v2");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to save pet stable");
    }
    txn.commit().expect("Failed to commit pet stable");
}
