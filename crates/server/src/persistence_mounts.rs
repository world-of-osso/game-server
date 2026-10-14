//! Account-owned Mount.db2 IDs, independent of character serialization.
use super::inventory_store::{InventoryRecord, write_inventory_record};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use std::collections::BTreeMap;

pub(crate) const ACCOUNT_MOUNTS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("account_mounts");
pub(crate) type AccountMounts = BTreeMap<u32, bool>;

pub(crate) fn create_tables(txn: &WriteTransaction) {
    txn.open_table(ACCOUNT_MOUNTS)
        .expect("create account mounts");
}

pub(crate) fn load(db: &Database, account: u64) -> AccountMounts {
    let txn = db.begin_read().expect("begin mount read");
    let table = txn.open_table(ACCOUNT_MOUNTS).expect("open account mounts");
    table
        .get(account)
        .expect("read mounts")
        .map(|row| bitcode::decode(row.value()).expect("decode mounts"))
        .unwrap_or_default()
}

fn read_for_write(txn: &WriteTransaction, account: u64) -> AccountMounts {
    let table = txn.open_table(ACCOUNT_MOUNTS).expect("open account mounts");
    table
        .get(account)
        .expect("read mounts")
        .map(|row| bitcode::decode(row.value()).expect("decode mounts"))
        .unwrap_or_default()
}

fn write(txn: &WriteTransaction, account: u64, mounts: &AccountMounts) {
    txn.open_table(ACCOUNT_MOUNTS)
        .expect("open account mounts")
        .insert(account, bitcode::encode(mounts).as_slice())
        .expect("write mounts");
}

/// Normal spell learning (quest/trainer/spell effect) adds ownership without resetting favourites.
pub(crate) fn learn_spells(db: &Database, account: u64, ids: &[u32]) {
    let txn = db.begin_write().expect("begin mount spell learning");
    let mut mounts = read_for_write(&txn, account);
    for id in ids {
        mounts.entry(*id).or_insert(false);
    }
    write(&txn, account, &mounts);
    crate::persistence_commit::commit(txn).expect("commit mount spell learning");
}

/// Learning and consuming the physical item share one durable transaction.
pub(crate) fn learn_item(
    db: &Database,
    account: u64,
    character: u64,
    mount: u32,
    consumed: &InventoryRecord,
) -> Result<(), String> {
    let txn = db.begin_write().expect("begin mount item learning");
    let mut mounts = read_for_write(&txn, account);
    if mounts.contains_key(&mount) {
        return Err("You already know this mount.".into());
    }
    mounts.insert(mount, false);
    write(&txn, account, &mounts);
    write_inventory_record(&txn, character, consumed);
    crate::persistence_commit::commit(txn).expect("commit mount item learning");
    Ok(())
}

pub(crate) fn favourite(
    db: &Database,
    account: u64,
    mount: u32,
    favourite: bool,
) -> Result<(), String> {
    let txn = db.begin_write().expect("begin mount favourite");
    let mut mounts = read_for_write(&txn, account);
    *mounts
        .get_mut(&mount)
        .ok_or("You have not collected this mount.")? = favourite;
    write(&txn, account, &mounts);
    crate::persistence_commit::commit(txn).expect("commit mount favourite");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mount_journal_account_learning_and_favourites_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mounts.redb");
        {
            let db = Database::create(&path).unwrap();
            crate::persistence::create_tables(&db);
            assert!(favourite(&db, 1, 18, true).is_err());
            learn_spells(&db, 1, &[18, 6]);
            favourite(&db, 1, 18, true).unwrap();
            learn_spells(&db, 1, &[18]);
            assert!(load(&db, 2).is_empty());
        }
        let db = Database::open(path).unwrap();
        assert_eq!(load(&db, 1), [(6, false), (18, true)].into());
    }
}
