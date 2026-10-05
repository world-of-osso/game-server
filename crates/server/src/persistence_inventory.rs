//! Bag placements and per-instance item attributes, stored beside `InventoryData`.
//!
//! `InventoryData.items` stays the owned-item list every system edits; placements
//! record which bag slot each item guid occupies. Items without a placement (new
//! loot, trade, mail) are placed by the inventory model on next load.
#[cfg(test)]
#[path = "persistence_item_source_tests.rs"]
mod item_source_tests;

use std::collections::{HashMap, HashSet};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use shared::item_data::ItemDefinitionSource;

use super::equipment::{EQUIPMENT, EquippedItem};
use super::{
    CHARACTERS, CharacterData, INVENTORIES, InventoryData, InventoryItemData, load_character,
    load_inventory,
};

/// Bag layout: character_id (u64) -> bitcode `Vec<ItemPlacementData>`.
pub(crate) const INVENTORY_LAYOUTS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("inventory_layouts");

/// Equipped bags: character_id (u64) -> bitcode `Vec<EquippedBagData>`. Kept apart
/// from `EQUIPMENT`, whose rows are keyed by the visual slot bags don't have.
pub(crate) const BAG_SLOTS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_bag_slots");

/// Item instance attributes: item_guid (u64) -> bitcode `ItemInstanceData`.
/// Keyed by guid so attributes follow the item through trade, mail and auctions.
pub(crate) const ITEM_INSTANCES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("item_instances");

/// Immutable catalog identity, independent of owner and unchanged serialized rows.
/// Tags: 0 = Retail, 1 = Forever build 70205. Unmarked legacy GUIDs are Retail.
pub(crate) const ITEM_DEFINITION_SOURCES: TableDefinition<u64, u8> =
    TableDefinition::new("item_definition_sources");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPlacementData {
    pub item_guid: u64,
    pub bag: u8,
    pub slot: u8,
}

/// The bag equipped in bag slot `bag` (1-4 bags, 5 the reagent bag).
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct EquippedBagData {
    pub bag: u8,
    pub item: InventoryItemData,
}

/// Per-instance item state (template attributes live in `ItemTemplates`).
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemInstanceData {
    pub soulbound: bool,
}

/// Everything an inventory operation reads and writes for one character.
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryRecord {
    pub character: CharacterData,
    pub inventory: InventoryData,
    pub placements: Vec<ItemPlacementData>,
    pub equipped: Vec<EquippedItem>,
    pub bag_slots: Vec<EquippedBagData>,
    pub instances: HashMap<u64, ItemInstanceData>,
    pub definition_sources: HashMap<u64, ItemDefinitionSource>,
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
    let bag_slots: Vec<EquippedBagData> = read_row(
        &txn.open_table(BAG_SLOTS).expect("open bag slots"),
        character_id,
    );
    let table = txn.open_table(ITEM_INSTANCES).expect("open item instances");
    let sources = txn
        .open_table(ITEM_DEFINITION_SOURCES)
        .expect("open item definition sources");
    let guids: Vec<u64> = inventory
        .items
        .iter()
        .chain(equipped.iter().map(|entry| &entry.item))
        .chain(bag_slots.iter().map(|entry| &entry.item))
        .map(|item| item.item_guid)
        .collect();
    let definition_sources = guids
        .iter()
        .map(|&guid| (guid, read_item_definition_source(&sources, guid)))
        .collect();
    let instances = guids
        .into_iter()
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
        bag_slots,
        instances,
        definition_sources,
    })
}

/// Read global GUID identity; absent legacy markers mean Retail, not a catalog probe.
pub fn load_item_definition_source(db: &Database, guid: u64) -> ItemDefinitionSource {
    let txn = db.begin_read().expect("begin item definition source read");
    let table = txn
        .open_table(ITEM_DEFINITION_SOURCES)
        .expect("open item definition sources");
    read_item_definition_source(&table, guid)
}

fn decode_item_definition_source(guid: u64, tag: u8) -> ItemDefinitionSource {
    match tag {
        0 => ItemDefinitionSource::Retail,
        1 => ItemDefinitionSource::Forever70205,
        _ => panic!("invalid item definition source {tag} for GUID {guid}"),
    }
}

fn read_item_definition_source(
    table: &impl ReadableTable<u64, u8>,
    guid: u64,
) -> ItemDefinitionSource {
    let row = table.get(guid).expect("read item definition source");
    row.map_or(ItemDefinitionSource::Retail, |row| {
        decode_item_definition_source(guid, row.value())
    })
}

/// Mark a newly minted GUID inside its creation transaction. Repeated writes of
/// the same identity are allowed; changing an existing marker is an error.
pub(crate) fn write_item_definition_source(
    txn: &WriteTransaction,
    guid: u64,
    source: ItemDefinitionSource,
) {
    let mut table = txn
        .open_table(ITEM_DEFINITION_SOURCES)
        .expect("open item definition sources");
    let existing = table
        .get(guid)
        .expect("read item definition source")
        .map(|row| decode_item_definition_source(guid, row.value()));
    if let Some(stored) = existing {
        assert_eq!(
            stored, source,
            "cannot change item definition source for GUID {guid}"
        );
        return;
    }
    let tag = match source {
        ItemDefinitionSource::Retail => 0,
        ItemDefinitionSource::Forever70205 => 1,
    };
    table
        .insert(guid, tag)
        .expect("write item definition source");
}

/// Which of `guids` are soulbound (`ItemInstanceData::soulbound`).
pub fn soulbound_guids(db: &Database, guids: impl Iterator<Item = u64>) -> HashSet<u64> {
    let txn = db.begin_read().expect("Failed to begin item instance read");
    let table = txn.open_table(ITEM_INSTANCES).expect("open item instances");
    guids
        .filter(|&guid| {
            table
                .get(guid)
                .expect("read item instance")
                .is_some_and(|row| {
                    bitcode::decode::<ItemInstanceData>(row.value())
                        .expect("decode item instance")
                        .soulbound
                })
        })
        .collect()
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
    write_inventory_record(&txn, character_id, record);
    txn.commit().expect("Failed to commit inventory write");
}

/// Write all parts of the record inside `txn`.
pub(crate) fn write_inventory_record(
    txn: &WriteTransaction,
    character_id: u64,
    record: &InventoryRecord,
) {
    let rows: [(TableDefinition<u64, &[u8]>, Vec<u8>); 5] = [
        (INVENTORIES, bitcode::encode(&record.inventory)),
        (INVENTORY_LAYOUTS, bitcode::encode(&record.placements)),
        (EQUIPMENT, bitcode::encode(&record.equipped)),
        (BAG_SLOTS, bitcode::encode(&record.bag_slots)),
        (CHARACTERS, bitcode::encode(&record.character)),
    ];
    for (definition, bytes) in rows {
        txn.open_table(definition)
            .expect("open inventory table")
            .insert(character_id, bytes.as_slice())
            .expect("write inventory row");
    }
    for (&guid, &source) in &record.definition_sources {
        write_item_definition_source(txn, guid, source);
    }
    let mut instances = txn.open_table(ITEM_INSTANCES).expect("open item instances");
    for (guid, data) in &record.instances {
        instances
            .insert(*guid, bitcode::encode(data).as_slice())
            .expect("write item instance");
    }
}
