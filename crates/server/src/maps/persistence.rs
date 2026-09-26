//! The map each character is saved on (`characters.map`). Characters saved before
//! maps existed have no row and are on map 0, the only map the server hosted then.

use redb::{Database, ReadableDatabase, TableDefinition};

/// character_id (u64) -> map id (u32).
pub(crate) const CHARACTER_MAPS: TableDefinition<u64, u32> = TableDefinition::new("character_maps");

pub(crate) fn load_character_map(db: &Database, character_id: u64) -> u32 {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTER_MAPS)
        .expect("Failed to open character maps table");
    table
        .get(character_id)
        .expect("Failed to read character map")
        .map_or(0, |guard| guard.value())
}

pub(crate) fn save_character_map(db: &Database, character_id: u64, map_id: u32) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_MAPS)
            .expect("Failed to open character maps table");
        table
            .insert(character_id, map_id)
            .expect("Failed to save character map");
    }
    txn.commit().expect("Failed to commit character map");
}
