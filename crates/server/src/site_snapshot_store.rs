//! Read-only projections of existing frozen bitcode tables. Field order is disk ABI.
//! Sources: persistence_types, persistence_equipment, persistence_progression,
//! transmog_store and maps/locks. No account/auth tables are opened.
use super::Result;
use redb::{ReadTransaction, TableDefinition};
use serde::Serialize;
use shared::components::EquipmentVisualSlot;
use std::collections::HashMap;

pub fn bytes(txn: &ReadTransaction, table: &str, id: u64) -> Result<Option<Vec<u8>>> {
    let table = match txn.open_table(TableDefinition::<u64, &[u8]>::new(table)) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    Ok(table.get(id)?.map(|row| row.value().to_vec()))
}
pub fn decode<T: bitcode::DecodeOwned>(
    txn: &ReadTransaction,
    table: &str,
    id: u64,
) -> Result<Option<T>> {
    bytes(txn, table, id)?
        .map(|bytes| bitcode::decode(&bytes).map_err(Into::into))
        .transpose()
}
pub fn json(txn: &ReadTransaction, table: &str, id: u64) -> Result<Option<serde_json::Value>> {
    bytes(txn, table, id)?
        .map(|bytes| serde_json::from_slice(&bytes).map_err(Into::into))
        .transpose()
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct Guild {
    pub guild_id: u32,
    pub guild_name: String,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct Item {
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub stack_count: u32,
    pub vendor_sell_price: u32,
    pub unique_auction: bool,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct Equipped {
    pub slot: EquipmentVisualSlot,
    pub item: Item,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct EquippedBag {
    pub bag: u8,
    pub item: Item,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct Transmog {
    pub source_id: u32,
    pub display_id: u32,
    pub inventory_type: u8,
    pub hidden: bool,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub enum Action {
    Spell(u32),
    Item(u32),
    Macro(u32),
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct Progression {
    pub spec_id: u32,
    pub action_bar: Vec<(u8, Action)>,
}
#[derive(bitcode::Encode, bitcode::Decode, Serialize)]
pub struct Purchase {
    pub node_id: u32,
    pub entry_id: u32,
    pub ranks: u32,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct NamedConfig {
    pub id: u32,
    pub name: String,
    pub purchases: Vec<Purchase>,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct SpecConfig {
    pub spec_id: u32,
    pub selected_id: u32,
    pub default: Vec<Purchase>,
    pub configs: Vec<NamedConfig>,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct Traits {
    pub specs: Vec<SpecConfig>,
    pub next_id: u32,
}
#[derive(bitcode::Encode, bitcode::Decode, Default)]
pub struct LockData {
    pub done_bosses: u32,
    pub completed_encounters: u32,
}
#[derive(bitcode::Encode, bitcode::Decode)]
pub struct Lock {
    pub map_id: u32,
    pub difficulty_id: u32,
    pub instance_id: u32,
    pub expiry: i64,
    pub extended: bool,
    pub data: LockData,
    pub is_new: bool,
}
#[derive(bitcode::Encode, bitcode::Decode, Default)]
pub struct Locks {
    pub locks: HashMap<u64, HashMap<(u32, u32), Lock>>,
    pub shared: HashMap<u32, LockData>,
}

pub fn locks(txn: &ReadTransaction) -> Result<Locks> {
    let table = match txn.open_table(TableDefinition::<u32, &[u8]>::new("instance_locks")) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Locks::default()),
        Err(error) => return Err(error.into()),
    };
    table
        .get(0)?
        .map(|row| bitcode::decode(row.value()).map_err(Into::into))
        .unwrap_or_else(|| Ok(Locks::default()))
}
