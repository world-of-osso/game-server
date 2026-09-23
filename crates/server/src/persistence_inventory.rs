//! Bag placements and per-instance item attributes, stored beside `InventoryData`.
//!
//! `InventoryData.items` stays the owned-item list every system edits; placements
//! record which bag slot each item guid occupies. Items without a placement (new
//! loot, trade, mail) are placed by the inventory model on next load.
use std::collections::HashMap;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use super::equipment::{EQUIPMENT, EquippedItem};
use super::{
    CHARACTERS, CharacterData, INVENTORIES, InventoryData, load_character, load_inventory,
};

/// Bag layout: character_id (u64) -> bitcode `Vec<ItemPlacementData>`.
pub(crate) const INVENTORY_LAYOUTS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("inventory_layouts");

/// Item instance attributes: item_guid (u64) -> bitcode `ItemInstanceData`.
/// Keyed by guid so attributes follow the item through trade, mail and auctions.
pub(crate) const ITEM_INSTANCES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("item_instances");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPlacementData {
    pub item_guid: u64,
    pub bag: u8,
    pub slot: u8,
}

/// Instance attributes the server has no item template for.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemInstanceData {
    /// Retail `Enum.InventoryType`; 0 = not equippable / unknown.
    pub inventory_type: u8,
    pub soulbound: bool,
}

/// Everything an inventory operation reads and writes for one character.
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryRecord {
    pub character: CharacterData,
    pub inventory: InventoryData,
    pub placements: Vec<ItemPlacementData>,
    pub equipped: Vec<EquippedItem>,
    pub instances: HashMap<u64, ItemInstanceData>,
}

pub fn load_inventory_record(db: &Database, character_id: u64) -> Option<InventoryRecord> {
    let character = load_character(db, character_id)?;
    let inventory = load_inventory(db, character_id);
    let txn = db.begin_read().expect("Failed to begin inventory read");
    let placements = read_row(
        &txn.open_table(INVENTORY_LAYOUTS).expect("open layouts"),
        character_id,
    );
    let equipped: Vec<EquippedItem> = read_row(
        &txn.open_table(EQUIPMENT).expect("open equipment"),
        character_id,
    );
    let table = txn.open_table(ITEM_INSTANCES).expect("open item instances");
    let guids = inventory
        .items
        .iter()
        .chain(equipped.iter().map(|entry| &entry.item))
        .map(|item| item.item_guid);
    let instances = guids
        .filter_map(|guid| {
            let row = table.get(guid).expect("read item instance")?;
            let data = bitcode::decode(row.value()).expect("decode item instance");
            Some((guid, data))
        })
        .collect();
    Some(InventoryRecord {
        character,
        inventory,
        placements,
        equipped,
        instances,
    })
}

fn read_row<T: bitcode::DecodeOwned + Default>(
    table: &impl ReadableTable<u64, &'static [u8]>,
    key: u64,
) -> T {
    table
        .get(key)
        .expect("read inventory row")
        .map(|row| bitcode::decode(row.value()).expect("decode inventory row"))
        .unwrap_or_default()
}

/// Persist all parts of the record in one transaction.
pub fn save_inventory_record(db: &Database, character_id: u64, record: &InventoryRecord) {
    let txn = db.begin_write().expect("Failed to begin inventory write");
    {
        let rows: [(TableDefinition<u64, &[u8]>, Vec<u8>); 4] = [
            (INVENTORIES, bitcode::encode(&record.inventory)),
            (INVENTORY_LAYOUTS, bitcode::encode(&record.placements)),
            (EQUIPMENT, bitcode::encode(&record.equipped)),
            (CHARACTERS, bitcode::encode(&record.character)),
        ];
        for (definition, bytes) in rows {
            txn.open_table(definition)
                .expect("open inventory table")
                .insert(character_id, bytes.as_slice())
                .expect("write inventory row");
        }
        let mut instances = txn.open_table(ITEM_INSTANCES).expect("open item instances");
        for (guid, data) in &record.instances {
            instances
                .insert(*guid, bitcode::encode(data).as_slice())
                .expect("write item instance");
        }
    }
    txn.commit().expect("Failed to commit inventory write");
}
