//! Per-character known flight points (AzerothCore `characters.taximask`).
use std::collections::BTreeSet;

use redb::{Database, ReadableDatabase, TableDefinition};

/// character_id (u64) -> bitcode ascending `TaxiNodes.ID` list.
pub(crate) const KNOWN_TAXI_NODES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("known_taxi_nodes");

pub fn load_known_taxi_nodes(db: &Database, character_id: u64) -> BTreeSet<u32> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(KNOWN_TAXI_NODES) else {
        return BTreeSet::new();
    };
    table
        .get(character_id)
        .expect("Failed to read known taxi nodes")
        .map_or_else(BTreeSet::new, |guard| {
            let nodes: Vec<u32> =
                bitcode::decode(guard.value()).expect("Failed to decode known taxi nodes");
            nodes.into_iter().collect()
        })
}

pub fn save_known_taxi_nodes(db: &Database, character_id: u64, nodes: &BTreeSet<u32>) {
    let bytes = bitcode::encode(&nodes.iter().copied().collect::<Vec<u32>>());
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(KNOWN_TAXI_NODES)
            .expect("Failed to open known_taxi_nodes table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to save known taxi nodes");
    }
    txn.commit().expect("Failed to commit known taxi nodes");
}
