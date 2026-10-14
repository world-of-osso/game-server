//! Account-wide Toy Box, outside frozen character serialization.
use super::inventory_store::{InventoryRecord, write_inventory_record};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use std::collections::BTreeMap;

pub(crate) const ACCOUNT_TOYS: TableDefinition<u64, &[u8]> = TableDefinition::new("account_toys");
/// Learned item id -> favourite. Unlearned entries are never stored.
pub(crate) type AccountToys = BTreeMap<u32, bool>;

pub(crate) fn create_tables(txn: &WriteTransaction) {
    txn.open_table(ACCOUNT_TOYS).expect("create account toys");
}

pub(crate) fn load(db: &Database, account_id: u64) -> AccountToys {
    let txn = db.begin_read().expect("begin toys read");
    let table = txn.open_table(ACCOUNT_TOYS).expect("open account toys");
    table
        .get(account_id)
        .expect("read toys")
        .map(|row| bitcode::decode(row.value()).expect("decode toys"))
        .unwrap_or_default()
}

fn write(txn: &WriteTransaction, account_id: u64, toys: &AccountToys) {
    txn.open_table(ACCOUNT_TOYS)
        .expect("open toys")
        .insert(account_id, bitcode::encode(toys).as_slice())
        .expect("write toys");
}

/// One commit both learns the toy and removes the physical item. No crash window.
pub(crate) fn learn(
    db: &Database,
    account_id: u64,
    character_id: u64,
    item_id: u32,
    consumed: &InventoryRecord,
) -> Result<(), String> {
    let txn = db.begin_write().expect("begin toy learning");
    let mut toys: AccountToys = {
        let table = txn.open_table(ACCOUNT_TOYS).expect("open toys");
        table
            .get(account_id)
            .expect("read toys")
            .map(|row| bitcode::decode(row.value()).expect("decode toys"))
            .unwrap_or_default()
    };
    if toys.contains_key(&item_id) {
        return Err("toy already learned".into());
    }
    toys.insert(item_id, false);
    write(&txn, account_id, &toys);
    write_inventory_record(&txn, character_id, consumed);
    txn.commit().expect("commit toy learning");
    Ok(())
}

pub(crate) fn favourite(
    db: &Database,
    account_id: u64,
    item_id: u32,
    favourite: bool,
) -> Result<(), String> {
    let txn = db.begin_write().expect("begin toy favourite");
    let mut toys: AccountToys = {
        let table = txn.open_table(ACCOUNT_TOYS).expect("open toys");
        table
            .get(account_id)
            .expect("read toys")
            .map(|row| bitcode::decode(row.value()).expect("decode toys"))
            .unwrap_or_default()
    };
    *toys.get_mut(&item_id).ok_or("toy not learned")? = favourite;
    write(&txn, account_id, &toys);
    txn.commit().expect("commit toy favourite");
    Ok(())
}
