use redb::{
    Database, ReadableDatabase, ReadableTable, TableDefinition, TableError, WriteTransaction,
};

use super::model::EconomyState;
use crate::persistence::inventory_store::{InventoryRecord, write_inventory_record};
use crate::persistence::mail_store::{MAILBOXES, MailData};
use crate::persistence::{AUCTIONS, AuctionData, INVENTORIES, InventoryData};

const STATE: TableDefinition<u64, &[u8]> = TableDefinition::new("economy_state_v1");
const STATE_KEY: u64 = 0;
type StoreResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Persist these rows together; callers publish in-memory changes only after success.
#[derive(Clone, Default)]
pub struct MarketChanges {
    /// Cycle input checked under the write lock before replacing merchant state.
    pub expected_economy: Option<EconomyState>,
    pub auctions: Vec<AuctionData>,
    pub deleted_auctions: Vec<u64>,
    /// Existing rows that must still match, checked while holding the write transaction.
    pub expected_auctions: Vec<(u64, AuctionData)>,
    /// Inventory snapshots checked under the same write lock, before any changes.
    pub expected_inventories: Vec<(u64, InventoryData)>,
    /// Mailbox snapshots must still match before replacing collected mail.
    pub expected_mailboxes: Vec<(u64, Vec<MailData>)>,
    pub inventories: Vec<(u64, InventoryData)>,
    pub records: Vec<(u64, InventoryRecord)>,
    /// Identity for new GUIDs minted from owned market items.
    pub definition_sources: Vec<(u64, shared::item_data::ItemDefinitionSource)>,
    /// Full replacements, applied before appending `mails`.
    pub mailboxes: Vec<(u64, Vec<MailData>)>,
    pub mails: Vec<(u64, MailData)>,
    /// Runtime-only receipt metadata; callers publish it after commit, never serialize it.
    pub(crate) criteria: Vec<(u64, crate::achievement::CriteriaEvent)>,
}

pub fn init_state_table(db: &Database) -> Result<(), String> {
    let initialize = || -> StoreResult<()> {
        let txn = db.begin_write()?;
        txn.open_table(STATE)?;
        txn.commit()?;
        Ok(())
    };
    initialize().map_err(|error| format!("initialize economy state table: {error}"))
}

/// Missing table or row is uninitialized; malformed data is an error.
pub fn load_state(db: &Database) -> Result<Option<EconomyState>, String> {
    let load = || -> StoreResult<Option<EconomyState>> {
        let txn = db.begin_read()?;
        let table = match txn.open_table(STATE) {
            Ok(table) => table,
            Err(TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        table
            .get(STATE_KEY)?
            .map(|row| bitcode::decode(row.value()))
            .transpose()
            .map_err(Into::into)
    };
    load().map_err(|error| format!("load economy state: {error}"))
}

/// Failures drop the uncommitted transaction. No caller-owned values are mutated.
pub fn commit_market(
    db: &Database,
    state: Option<&EconomyState>,
    changes: &MarketChanges,
) -> Result<(), String> {
    let commit = || -> StoreResult<()> {
        let txn = db.begin_write()?;
        check_expected_auctions(&txn, &changes.expected_auctions)?;
        check_expected_inventories(&txn, &changes.expected_inventories)?;
        check_expected_economy(&txn, changes.expected_economy.as_ref())?;
        check_expected_mailboxes(&txn, &changes.expected_mailboxes)?;
        if let Some(state) = state {
            txn.open_table(STATE)?
                .insert(STATE_KEY, bitcode::encode(state).as_slice())?;
        }
        for &(guid, source) in &changes.definition_sources {
            crate::persistence::inventory_store::write_item_definition_source(&txn, guid, source);
        }
        write_auctions(&txn, changes)?;
        write_inventories(&txn, changes)?;
        write_mail_changes(&txn, changes)?;
        txn.commit()?;
        Ok(())
    };
    commit().map_err(|error| format!("commit market: {error}"))
}

fn check_expected_mailboxes(
    txn: &WriteTransaction,
    expected: &[(u64, Vec<MailData>)],
) -> StoreResult<()> {
    if expected.is_empty() {
        return Ok(());
    }
    let table = txn.open_table(MAILBOXES)?;
    for (id, snapshot) in expected {
        let persisted: Vec<MailData> = table
            .get(*id)?
            .map(|row| bitcode::decode(row.value()))
            .transpose()?
            .unwrap_or_default();
        if persisted != *snapshot {
            return Err(format!("mailbox {id} changed since it was read").into());
        }
    }
    Ok(())
}

fn check_expected_economy(
    txn: &WriteTransaction,
    expected: Option<&EconomyState>,
) -> StoreResult<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let table = txn.open_table(STATE)?;
    let row = table
        .get(STATE_KEY)?
        .ok_or("economy state no longer exists")?;
    let persisted: EconomyState = bitcode::decode(row.value())?;
    if persisted != *expected {
        return Err("economy state changed since it was read".into());
    }
    Ok(())
}

fn check_expected_auctions(
    txn: &WriteTransaction,
    expected: &[(u64, AuctionData)],
) -> StoreResult<()> {
    if expected.is_empty() {
        return Ok(());
    }
    let table = txn.open_table(AUCTIONS)?;
    for (id, expected_row) in expected {
        let row = table
            .get(*id)?
            .ok_or_else(|| format!("auction {id} no longer exists"))?;
        let persisted: AuctionData = bitcode::decode(row.value())
            .map_err(|error| format!("decode auction {id}: {error}"))?;
        if persisted != *expected_row {
            return Err(format!("auction {id} changed since it was read").into());
        }
    }
    Ok(())
}

fn check_expected_inventories(
    txn: &WriteTransaction,
    expected: &[(u64, InventoryData)],
) -> StoreResult<()> {
    if expected.is_empty() {
        return Ok(());
    }
    let table = txn.open_table(INVENTORIES)?;
    for (id, expected_row) in expected {
        let row = table
            .get(*id)?
            .ok_or_else(|| format!("inventory {id} no longer exists"))?;
        let persisted: InventoryData = bitcode::decode(row.value())
            .map_err(|error| format!("decode inventory {id}: {error}"))?;
        if persisted != *expected_row {
            return Err(format!("inventory {id} changed since it was read").into());
        }
    }
    Ok(())
}

fn write_auctions(txn: &WriteTransaction, changes: &MarketChanges) -> StoreResult<()> {
    if changes.auctions.is_empty() && changes.deleted_auctions.is_empty() {
        return Ok(());
    }
    let mut table = txn.open_table(AUCTIONS)?;
    for auction in &changes.auctions {
        table.insert(auction.auction_id, bitcode::encode(auction).as_slice())?;
    }
    for id in &changes.deleted_auctions {
        table.remove(*id)?;
    }
    Ok(())
}

fn write_inventories(txn: &WriteTransaction, changes: &MarketChanges) -> StoreResult<()> {
    if !changes.inventories.is_empty() {
        let mut table = txn.open_table(INVENTORIES)?;
        for (id, inventory) in &changes.inventories {
            table.insert(*id, bitcode::encode(inventory).as_slice())?;
        }
    }
    for (id, record) in &changes.records {
        write_inventory_record(txn, *id, record);
    }
    Ok(())
}

fn write_mail_changes(txn: &WriteTransaction, changes: &MarketChanges) -> StoreResult<()> {
    if changes.mailboxes.is_empty() && changes.mails.is_empty() {
        return Ok(());
    }
    let mut table = txn.open_table(MAILBOXES)?;
    for (id, mailbox) in &changes.mailboxes {
        table.insert(*id, bitcode::encode(mailbox).as_slice())?;
    }
    for (id, mail) in &changes.mails {
        let mut mailbox: Vec<MailData> = table
            .get(*id)?
            .map(|row| bitcode::decode(row.value()))
            .transpose()
            .map_err(|error| format!("decode mailbox {id}: {error}"))?
            .unwrap_or_default();
        mailbox.push(mail.clone());
        table.insert(*id, bitcode::encode(&mailbox).as_slice())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
