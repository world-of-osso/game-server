//! Physical equipped items are independent of the CharacterData encoding.
use super::{CHARACTER_NAMES, CHARACTERS, CharacterData, InventoryItemData, NEXT_IDS};
use crate::equipment_admin_types::{EquipmentGrantItem, EquipmentOwnership};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};
use std::collections::HashSet;

pub(crate) const EQUIPMENT: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_equipment");
type StoreResult<T> = Result<T, Box<dyn std::error::Error>>;
type GrantedEquipment = (u64, CharacterData, Vec<EquipmentOwnership>);

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub(crate) struct EquippedItem {
    pub(crate) slot: EquipmentVisualSlot,
    pub(crate) item: InventoryItemData,
}

pub(crate) fn grant_equipment(
    db: &Database,
    name: &str,
    items: &[EquipmentGrantItem],
) -> Result<GrantedEquipment, String> {
    grant_transaction(db, name, items)
        .map_err(|error| format!("grant equipment to '{name}': {error}"))
}

fn grant_transaction(
    db: &Database,
    name: &str,
    items: &[EquipmentGrantItem],
) -> StoreResult<GrantedEquipment> {
    let txn = db.begin_write()?;
    let (id, mut character) = load_named_character(&txn, name)?;
    let mut equipped = {
        let table = txn.open_table(EQUIPMENT)?;
        table
            .get(id)?
            .map(|bytes| bitcode::decode::<Vec<EquippedItem>>(bytes.value()))
            .transpose()?
            .unwrap_or_default()
    };
    let sources = txn.open_table(super::inventory_store::ITEM_DEFINITION_SOURCES)?;
    crate::persistence_character::hydrate_equipment_sources(&mut character, &equipped, &sources);
    drop(sources);
    let slots = validate_batch(&character, &equipped, items)?;
    append_granted_items(&txn, &mut character, &mut equipped, items, slots)?;
    let encoded_items = bitcode::encode(&equipped);
    let encoded_character = character.encode_stored();
    txn.open_table(EQUIPMENT)?
        .insert(id, encoded_items.as_slice())?;
    txn.open_table(CHARACTERS)?
        .insert(id, encoded_character.as_slice())?;
    let result = (
        id,
        character,
        equipped.iter().map(describe_owned_item).collect(),
    );
    txn.commit()?;
    Ok(result)
}

fn load_named_character(txn: &WriteTransaction, name: &str) -> StoreResult<(u64, CharacterData)> {
    let names = txn.open_table(CHARACTER_NAMES)?;
    let id = names
        .get(name)?
        .ok_or_else(|| format!("character '{name}' not found"))?
        .value();
    let characters = txn.open_table(CHARACTERS)?;
    let bytes = characters
        .get(id)?
        .ok_or_else(|| format!("character data {id} missing"))?;
    Ok((id, CharacterData::decode_compatible(bytes.value())?))
}

fn validate_batch(
    character: &CharacterData,
    equipped: &[EquippedItem],
    items: &[EquipmentGrantItem],
) -> StoreResult<Vec<EquipmentVisualSlot>> {
    if items.is_empty() {
        return Err("equipment manifest is empty".into());
    }
    let mut slots = HashSet::new();
    let mut ids = HashSet::new();
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        validate_item(character, item)?;
        let slot = equipment_slot(item.inventory_type)?;
        if !slots.insert(slot) || !ids.insert(item.item_id) {
            return Err(format!("duplicate equipment slot or item {}", item.item_id).into());
        }
        ensure_slot_empty(character, equipped, slot, item)?;
        result.push(slot);
    }
    validate_hand_conflicts(character, items)?;
    Ok(result)
}

fn validate_item(character: &CharacterData, item: &EquipmentGrantItem) -> StoreResult<()> {
    if item.item_id == 0 || item.name.trim().is_empty() || item.quality > 8 {
        return Err(format!("invalid metadata for item {}", item.item_id).into());
    }
    if item.required_level > character.level {
        return Err(format!(
            "item {} requires level {}",
            item.item_id, item.required_level
        )
        .into());
    }
    if !mask_allows(item.allowable_class, character.class)
        || !mask_allows(item.allowable_race, character.race)
    {
        return Err(format!("item {} is not allowed for this class/race", item.item_id).into());
    }
    Ok(())
}

fn mask_allows(mask: i64, id: u8) -> bool {
    if mask == -1 {
        return true;
    }
    id.checked_sub(1)
        .and_then(|shift| 1u64.checked_shl(u32::from(shift)))
        .is_some_and(|bit| mask >= 0 && mask as u64 & bit != 0)
}

fn equipment_slot(inventory_type: u8) -> StoreResult<EquipmentVisualSlot> {
    use EquipmentVisualSlot::*;
    let slot = match inventory_type {
        1 => Head,
        3 => Shoulder,
        4 => Shirt,
        5 | 20 => Chest,
        6 => Waist,
        7 => Legs,
        8 => Feet,
        9 => Wrist,
        10 => Hands,
        16 => Back,
        19 => Tabard,
        13 | 17 | 21 => MainHand,
        14 | 22 | 23 => OffHand,
        _ => return Err(format!("unsupported inventory type {inventory_type}").into()),
    };
    Ok(slot)
}

fn ensure_slot_empty(
    character: &CharacterData,
    equipped: &[EquippedItem],
    slot: EquipmentVisualSlot,
    item: &EquipmentGrantItem,
) -> StoreResult<()> {
    let occupied = equipped
        .iter()
        .any(|entry| entry.slot == slot || entry.item.item_id == item.item_id)
        || character
            .equipment_appearance
            .entries
            .iter()
            .any(|entry| entry.slot == slot);
    if occupied {
        return Err(
            format!("equipment slot {slot:?} is occupied (item or appearance override)").into(),
        );
    }
    Ok(())
}

fn validate_hand_conflicts(
    character: &CharacterData,
    items: &[EquipmentGrantItem],
) -> StoreResult<()> {
    let two_handed = items.iter().any(|item| item.inventory_type == 17)
        || character
            .equipment_appearance
            .entries
            .iter()
            .any(|entry| entry.inventory_type == 17);
    let offhand = items
        .iter()
        .any(|item| matches!(item.inventory_type, 14 | 22 | 23))
        || character
            .equipment_appearance
            .entries
            .iter()
            .any(|entry| entry.slot == EquipmentVisualSlot::OffHand);
    if two_handed && offhand {
        return Err("two-handed weapon conflicts with off-hand equipment".into());
    }
    Ok(())
}

fn append_granted_items(
    txn: &WriteTransaction,
    character: &mut CharacterData,
    equipped: &mut Vec<EquippedItem>,
    items: &[EquipmentGrantItem],
    slots: Vec<EquipmentVisualSlot>,
) -> StoreResult<()> {
    let mut counters = txn.open_table(NEXT_IDS)?;
    let mut guid = counters
        .get("item_instance")?
        .map(|value| value.value())
        .unwrap_or(0);
    for (item, slot) in items.iter().zip(slots) {
        guid = guid
            .checked_add(1)
            .ok_or("item instance GUID counter exhausted")?;
        equipped.push(EquippedItem {
            slot,
            item: owned_item(item, guid),
        });
        character
            .equipment_appearance
            .entries
            .push(EquippedAppearanceEntry {
                slot,
                item_id: Some(item.item_id),
                definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
                display_info_id: None,
                inventory_type: item.inventory_type,
                hidden: false,
            });
        super::inventory_store::write_item_definition_source(
            txn,
            guid,
            shared::item_data::ItemDefinitionSource::Retail,
        );
    }
    counters.insert("item_instance", guid)?;
    Ok(())
}

fn owned_item(item: &EquipmentGrantItem, item_guid: u64) -> InventoryItemData {
    InventoryItemData {
        item_guid,
        item_id: item.item_id,
        name: item.name.clone(),
        quality: item.quality,
        required_level: item.required_level,
        stack_count: 1,
        vendor_sell_price: item.vendor_sell_price,
        unique_auction: false,
    }
}

fn describe_owned_item(equipped: &EquippedItem) -> EquipmentOwnership {
    let item = &equipped.item;
    EquipmentOwnership {
        slot: equipped.slot,
        item_guid: item.item_guid,
        item_id: item.item_id,
        name: item.name.clone(),
        quality: item.quality,
        required_level: item.required_level,
        stack_count: item.stack_count,
        vendor_sell_price: item.vendor_sell_price,
        unique_auction: item.unique_auction,
    }
}

pub(crate) fn list_equipment(
    db: &Database,
    name: &str,
) -> Result<(u64, Vec<EquipmentOwnership>), String> {
    list_transaction(db, name).map_err(|error| format!("list equipment for '{name}': {error}"))
}

fn list_transaction(db: &Database, name: &str) -> StoreResult<(u64, Vec<EquipmentOwnership>)> {
    let txn = db.begin_read()?;
    let names = txn.open_table(CHARACTER_NAMES)?;
    let id = names
        .get(name)?
        .ok_or_else(|| format!("character '{name}' not found"))?
        .value();
    if txn.open_table(CHARACTERS)?.get(id)?.is_none() {
        return Err(format!("character data {id} missing").into());
    }
    let table = txn.open_table(EQUIPMENT)?;
    let equipped = table
        .get(id)?
        .map(|bytes| bitcode::decode::<Vec<EquippedItem>>(bytes.value()))
        .transpose()?
        .unwrap_or_default();
    Ok((id, equipped.iter().map(describe_owned_item).collect()))
}

#[cfg(test)]
#[path = "persistence_equipment_tests.rs"]
mod tests;
