//! Atomic account Trading Post balance, threshold receipts and collection grants.
use redb::{Database, ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct State {
    pub month_start: i64,
    pub tender: u32,
    pub claimed: BTreeSet<u32>,
    pub frozen: Option<u32>,
    pub purchases: BTreeMap<u32, Purchase>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Purchase {
    pub price: u32,
    pub refund_until: i64,
}
#[derive(Clone, Debug)]
pub(crate) struct Threshold {
    pub id: u32,
    pub points: u32,
    pub tender: u32,
}

const ACCOUNTS: TableDefinition<u64, &[u8]> = TableDefinition::new("trading_post_accounts");

fn read_state(txn: &WriteTransaction, account: u64) -> State {
    let table = txn
        .open_table(ACCOUNTS)
        .expect("open Trading Post accounts");
    table
        .get(account)
        .expect("read Trading Post account")
        .map(|row| serde_json::from_slice(row.value()).expect("decode Trading Post account"))
        .unwrap_or_default()
}

fn write_state(txn: &WriteTransaction, account: u64, state: &State) {
    let bytes = serde_json::to_vec(state).expect("encode Trading Post account");
    txn.open_table(ACCOUNTS)
        .expect("open Trading Post accounts")
        .insert(account, bytes.as_slice())
        .expect("write Trading Post account");
}

pub(crate) fn settle(
    db: &Database,
    account: u64,
    month: i64,
    points: u32,
    thresholds: &[Threshold],
) -> State {
    let txn = db
        .begin_write()
        .expect("begin Trading Post threshold awards");
    let mut state = read_state(&txn, account);
    if state.month_start != month {
        state.month_start = month;
        state.claimed.clear();
    }
    for threshold in thresholds {
        if points >= threshold.points && state.claimed.insert(threshold.id) {
            state.tender = state
                .tender
                .checked_add(threshold.tender)
                .expect("Tender balance overflow");
        }
    }
    write_state(&txn, account, &state);
    txn.commit().expect("commit Trading Post threshold awards");
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    fn thresholds() -> Vec<Threshold> {
        // Pinned PerksActivityThreshold group1: real IDs1-5, 100 Tender each.
        (1..=5)
            .map(|id| Threshold {
                id,
                points: id * 200,
                tender: 100,
            })
            .collect()
    }
    #[test]
    fn trading_post_thresholds_once_per_account_month_survive_database_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trading.redb");
        let october = 1790812800;
        let november = 1793491200;
        {
            let db = Database::create(&path).unwrap();
            crate::persistence::create_tables(&db);
            assert_eq!(settle(&db, 7, october, 199, &thresholds()).tender, 0);
            assert_eq!(settle(&db, 7, october, 200, &thresholds()).tender, 100);
            assert_eq!(settle(&db, 7, october, 200, &thresholds()).tender, 100);
            assert_eq!(settle(&db, 7, october, 9000, &thresholds()).tender, 500);
            assert_eq!(settle(&db, 8, october, 0, &thresholds()).tender, 0);
        }
        let db = Database::open(path).unwrap();
        assert_eq!(settle(&db, 7, october, 9000, &thresholds()).tender, 500);
        let rolled = settle(&db, 7, november, 0, &thresholds());
        assert_eq!(rolled.tender, 500);
        assert!(rolled.claimed.is_empty());
        assert_eq!(settle(&db, 7, november, 200, &thresholds()).tender, 600);
    }
}
