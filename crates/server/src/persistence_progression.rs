use redb::{Database, ReadableDatabase, TableDefinition};

/// Class progression table: character_id -> serialized `ProgressionData`.
pub(crate) const CHARACTER_PROGRESSION: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_progression");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionRefData {
    Spell(u32),
    Item(u32),
    Macro(u32),
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Eq)]
pub struct ProgressionData {
    /// Active `ChrSpecialization` ID.
    pub spec_id: u32,
    /// Occupied action bar slots, ascending.
    pub action_bar: Vec<(u8, ActionRefData)>,
}

pub fn load_progression(db: &Database, character_id: u64) -> Option<ProgressionData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(CHARACTER_PROGRESSION).ok()?;
    let guard = table
        .get(character_id)
        .expect("Failed to read character progression")?;
    Some(bitcode::decode(guard.value()).expect("Failed to decode character progression"))
}

pub fn save_progression(db: &Database, character_id: u64, data: &ProgressionData) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_PROGRESSION)
            .expect("Failed to open character_progression table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert character progression");
    }
    txn.commit()
        .expect("Failed to commit character progression save");
}
