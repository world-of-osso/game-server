//! The map each character is saved on (`characters.map`) and the instance locks. Characters saved before
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
    crate::persistence_commit::commit(txn).expect("Failed to commit character map");
}

/// Key of the one `instance_locks` row.
const INSTANCE_LOCKS_KEY: u32 = 0;
/// `character_instance_lock` + `instance` (TrinityCore characters DB): every permanent
/// instance lock and the shared state of instance-id-bound copies, bitcode `SavedLocks`.
pub(crate) const INSTANCE_LOCKS: TableDefinition<u32, &[u8]> =
    TableDefinition::new("instance_locks");

pub(crate) fn load_instance_locks(db: &Database) -> super::locks::SavedLocks {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(INSTANCE_LOCKS) else {
        return super::locks::SavedLocks::default();
    };
    table
        .get(INSTANCE_LOCKS_KEY)
        .expect("Failed to read instance locks")
        .map_or_else(super::locks::SavedLocks::default, |guard| {
            bitcode::decode(guard.value()).expect("Failed to decode instance locks")
        })
}

pub(crate) const NORMAL_INSTANCE_LOCKS: TableDefinition<u32, &[u8]> =
    TableDefinition::new("normal_instance_locks");

pub(crate) fn load_normal_instance_locks(db: &Database) -> Vec<super::locks::NormalInstanceLock> {
    let txn = db
        .begin_read()
        .expect("Failed to begin normal instance read txn");
    let table = txn
        .open_table(NORMAL_INSTANCE_LOCKS)
        .expect("Failed to open normal instance locks");
    table
        .get(INSTANCE_LOCKS_KEY)
        .expect("Failed to read normal instance locks")
        .map_or_else(Vec::new, |guard| {
            bitcode::decode(guard.value()).expect("Failed to decode normal instance locks")
        })
}

pub(crate) fn save_instance_locks(
    db: &Database,
    locks: &super::locks::SavedLocks,
    normal: &[super::locks::NormalInstanceLock],
) {
    let bytes = bitcode::encode(locks);
    let normal_bytes = bitcode::encode(&normal.to_vec());
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(INSTANCE_LOCKS)
            .expect("Failed to open instance locks table");
        table
            .insert(INSTANCE_LOCKS_KEY, bytes.as_slice())
            .expect("Failed to save instance locks");
        let mut normal_table = txn
            .open_table(NORMAL_INSTANCE_LOCKS)
            .expect("Failed to open normal instance locks");
        normal_table
            .insert(INSTANCE_LOCKS_KEY, normal_bytes.as_slice())
            .expect("Failed to save normal instance locks");
    }
    crate::persistence_commit::commit(txn).expect("Failed to commit instance locks");
}
