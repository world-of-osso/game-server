//! Account-wide appearance collection: account_id -> serde_json
//! `shared::transmog::AppearanceCollection` (docs/specs/appearance-collection.md).
use redb::{Database, ReadableDatabase, TableDefinition, WriteTransaction};
use shared::transmog::AppearanceCollection;

pub(crate) const ACCOUNT_APPEARANCES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("account_appearances");

pub(crate) fn create_appearance_tables(txn: &WriteTransaction) {
    let _ = txn.open_table(ACCOUNT_APPEARANCES);
}

/// The account's collection; empty before it learns anything.
pub(crate) fn load_account_appearances(db: &Database, account_id: u64) -> AppearanceCollection {
    let txn = db.begin_read().expect("Failed to begin appearance read");
    let table = txn
        .open_table(ACCOUNT_APPEARANCES)
        .expect("open account_appearances");
    table
        .get(account_id)
        .expect("read account_appearances row")
        .map(|row| serde_json::from_slice(row.value()).expect("decode account appearances"))
        .unwrap_or_default()
}

pub(crate) fn save_account_appearances(
    db: &Database,
    account_id: u64,
    collection: &AppearanceCollection,
) {
    let bytes = serde_json::to_vec(collection).expect("encode account appearances");
    let txn = db.begin_write().expect("Failed to begin appearance write");
    txn.open_table(ACCOUNT_APPEARANCES)
        .expect("open account_appearances")
        .insert(account_id, bytes.as_slice())
        .expect("write account_appearances row");
    txn.commit().expect("Failed to commit appearance write");
}
