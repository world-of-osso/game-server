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
use shared::protocol::CraftingOrigin;

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

/// Non-visual equipment keyed by Retail inventory slot id (2, 11-14, 18).
/// Additive storage preserves the legacy visual equipment bitcode ABI.
pub(crate) const NONVISUAL_EQUIPMENT: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_nonvisual_equipment");

/// Item instance attributes: item_guid (u64) -> bitcode `ItemInstanceData`.
/// Keyed by guid so attributes follow the item through trade, mail and auctions.
pub(crate) const ITEM_INSTANCES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("item_instances");

/// Additive GUID-keyed provenance; the legacy ItemInstanceData encoding is unchanged.
pub(crate) const ITEM_CRAFTING_ORIGINS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("item_crafting_origins");

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

/// Durability by item guid: bitcode `(current, max)`, separate from existing instance rows.
pub(crate) const ITEM_DURABILITIES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("item_durabilities");

/// ITEM_FIELD_FLAG_UNLOCKED: GUID state follows the item across owner transfers.
/// Separate table preserves the existing ItemInstanceData encoding.
pub(crate) const ITEM_UNLOCKED: TableDefinition<u64, u8> = TableDefinition::new("item_unlocked");

pub(crate) fn is_item_unlocked(db: &Database, guid: u64) -> bool {
    let txn = db.begin_read().expect("begin item unlock read");
    let table = txn
        .open_table(ITEM_UNLOCKED)
        .expect("open item unlock state");
    table.get(guid).expect("read item unlock state").is_some()
}

pub(crate) fn unlock_item(db: &Database, guid: u64) -> Result<(), Box<dyn std::error::Error>> {
    let txn = db.begin_write()?;
    txn.open_table(ITEM_UNLOCKED)?.insert(guid, 1)?;
    txn.commit()?;
    Ok(())
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
    pub nonvisual_equipped: Vec<(u8, InventoryItemData)>,
    pub bag_slots: Vec<EquippedBagData>,
    pub instances: HashMap<u64, ItemInstanceData>,
    pub crafting_origins: HashMap<u64, CraftingOrigin>,
    pub item_modifiers: HashMap<u64, server::item_instance::ItemInstanceModifiers>,
    pub item_durability: HashMap<u64, (u32, u32)>,
    pub definition_sources: HashMap<u64, ItemDefinitionSource>,
    /// Runtime-only collection receipt; not part of the stored inventory ABI.
    pub appearance_account: Option<u64>,
    pub appearance_grants: Vec<u32>,
    pub(crate) transmogs: HashMap<u64, crate::transmog_store::ItemTransmog>,
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
    let nonvisual_equipped: Vec<(u8, InventoryItemData)> = read_row(
        &txn.open_table(NONVISUAL_EQUIPMENT)
            .expect("open nonvisual equipment"),
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
        .chain(nonvisual_equipped.iter().map(|(_, item)| item))
        .chain(bag_slots.iter().map(|entry| &entry.item))
        .map(|item| item.item_guid)
        .collect::<Vec<_>>();
    let definition_sources = guids
        .iter()
        .map(|&guid| (guid, read_item_definition_source(&sources, guid)))
        .collect();
    let instances = guids
        .iter()
        .copied()
        .filter_map(|guid| {
            let row = table.get(guid).expect("read item instance")?;
            let data = bitcode::decode(row.value()).expect("decode item instance");
            Some((guid, data))
        })
        .collect();
    let modifier_table = txn
        .open_table(server::item_instance::store::ITEM_MODIFIERS)
        .expect("open GUID item modifiers");
    let item_modifiers = guids
        .iter()
        .filter_map(|&guid| {
            let row = modifier_table
                .get(guid)
                .expect("read GUID item modifiers")?;
            Some((
                guid,
                bitcode::decode(row.value()).expect("decode GUID item modifiers"),
            ))
        })
        .collect();
    let origin_table = txn
        .open_table(ITEM_CRAFTING_ORIGINS)
        .expect("open crafting origins");
    let crafting_origins = guids
        .iter()
        .filter_map(|&guid| {
            read_item_crafting_origin(&origin_table, guid).map(|origin| (guid, origin))
        })
        .collect();
    let durability_table = txn
        .open_table(ITEM_DURABILITIES)
        .expect("open item durability");
    let transmogs = crate::transmog_store::load_item_transmogs(db, guids.iter().copied());
    let item_durability = guids
        .into_iter()
        .filter_map(|guid| {
            let row = durability_table.get(guid).expect("read item durability")?;
            Some((
                guid,
                bitcode::decode(row.value()).expect("decode item durability"),
            ))
        })
        .collect();
    Some(InventoryRecord {
        character,
        inventory,
        placements,
        equipped,
        nonvisual_equipped,
        bag_slots,
        instances,
        crafting_origins,
        item_modifiers,
        item_durability,
        definition_sources,
        appearance_account: super::find_account_id_for_character(db, character_id),
        appearance_grants: Vec::new(),
        transmogs,
    })
}

pub(crate) fn load_item_crafting_origin(db: &Database, guid: u64) -> Option<CraftingOrigin> {
    let txn = db.begin_read().expect("begin crafting origin read");
    let table = txn
        .open_table(ITEM_CRAFTING_ORIGINS)
        .expect("open crafting origins");
    read_item_crafting_origin(&table, guid)
}

fn read_item_crafting_origin(
    table: &impl ReadableTable<u64, &'static [u8]>,
    guid: u64,
) -> Option<CraftingOrigin> {
    let row = table.get(guid).expect("read crafting origin")?;
    Some(bitcode::decode(row.value()).expect("decode crafting origin"))
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

pub(crate) fn read_item_definition_source(
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

pub(crate) fn read_row<T: bitcode::DecodeOwned + Default>(
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
    if !record.appearance_grants.is_empty() {
        super::appearance_store::learn_account_appearances_in(
            txn,
            record
                .appearance_account
                .expect("appearance grant requires an account owner"),
            record.appearance_grants.iter().copied(),
        );
    }
    crate::transmog_store::write_item_transmogs(
        txn,
        record.definition_sources.keys().copied(),
        &record.transmogs,
    );
    let rows: [(TableDefinition<u64, &[u8]>, Vec<u8>); 6] = [
        (INVENTORIES, bitcode::encode(&record.inventory)),
        (INVENTORY_LAYOUTS, bitcode::encode(&record.placements)),
        (EQUIPMENT, bitcode::encode(&record.equipped)),
        (
            NONVISUAL_EQUIPMENT,
            bitcode::encode(&record.nonvisual_equipped),
        ),
        (BAG_SLOTS, bitcode::encode(&record.bag_slots)),
        (CHARACTERS, record.character.encode_stored()),
    ];
    for (definition, bytes) in rows {
        txn.open_table(definition)
            .expect("open inventory table")
            .insert(character_id, bytes.as_slice())
            .expect("write inventory row");
    }
    let mut durability = txn
        .open_table(ITEM_DURABILITIES)
        .expect("open item durability");
    for (guid, state) in &record.item_durability {
        durability
            .insert(*guid, bitcode::encode(state).as_slice())
            .expect("write item durability");
    }
    for (&guid, &source) in &record.definition_sources {
        write_item_definition_source(txn, guid, source);
    }
    crate::persistence_character::validate_forever_equipment_sources(
        txn,
        character_id,
        &record.character,
    );
    let mut instances = txn.open_table(ITEM_INSTANCES).expect("open item instances");
    for (guid, data) in &record.instances {
        instances
            .insert(*guid, bitcode::encode(data).as_slice())
            .expect("write item instance");
    }
    drop(instances);
    let mut origins = txn
        .open_table(ITEM_CRAFTING_ORIGINS)
        .expect("open crafting origins");
    for (&guid, origin) in &record.crafting_origins {
        origins
            .insert(guid, bitcode::encode(origin).as_slice())
            .expect("write crafting origin");
    }
    drop(origins);
    for (&guid, state) in &record.item_modifiers {
        server::item_instance::store::write_modifiers(txn, guid, state)
            .expect("write GUID item modifiers");
    }
    super::charters::prune_destroyed_in(txn, character_id, record);
}
