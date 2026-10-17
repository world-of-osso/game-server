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
    pub granted: Grant,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Grant {
    Appearances(Vec<u32>),
    Mount(u32),
    Pet(u32),
    Toy(u32),
    /// Exact newly-created instance, not every pet of that species.
    PetInstance(u64),
}

pub(crate) struct Offer<'a> {
    pub vendor: u32,
    pub price: u32,
    pub grant: &'a Grant,
    pub refund_duration: u32,
}

pub(crate) fn purchase(
    db: &Database,
    account: u64,
    offer: Offer<'_>,
    now: i64,
) -> Result<State, String> {
    checkout(db, account, &[offer], now)
}

fn apply_offer(
    txn: &WriteTransaction,
    state: &mut State,
    account: u64,
    offer: &Offer<'_>,
    now: i64,
) -> Result<(), String> {
    if state.purchases.contains_key(&offer.vendor) {
        return Err("This Trading Post item was already purchased.".into());
    }
    state.tender = state
        .tender
        .checked_sub(offer.price)
        .ok_or("Not enough Trader's Tender.")?;
    let granted = grant_collection(txn, account, offer.grant)?;
    let refund_until = if offer.refund_duration == 0 {
        now
    } else {
        now.checked_add(i64::from(offer.refund_duration))
            .ok_or("Refund expiry overflow.")?
    };
    state.purchases.insert(
        offer.vendor,
        Purchase {
            price: offer.price,
            refund_until,
            granted,
        },
    );
    Ok(())
}

pub(crate) fn checkout(
    db: &Database,
    account: u64,
    offers: &[Offer<'_>],
    now: i64,
) -> Result<State, String> {
    if offers.is_empty() {
        return Err("The Trading Post cart is empty.".into());
    }
    let txn = db.begin_write().expect("begin Trading Post checkout");
    let mut state = read_state(&txn, account);
    for offer in offers {
        apply_offer(&txn, &mut state, account, offer, now)?;
    }
    write_state(&txn, account, &state);
    txn.commit().expect("commit atomic Trading Post checkout");
    Ok(state)
}

pub(crate) fn refund(db: &Database, account: u64, vendor: u32, now: i64) -> Result<State, String> {
    let txn = db.begin_write().expect("begin Trading Post refund");
    let mut state = read_state(&txn, account);
    let receipt = state
        .purchases
        .get(&vendor)
        .ok_or("This item has no Trading Post purchase receipt.")?;
    if now >= receipt.refund_until {
        return Err("This Trading Post purchase is not refundable.".into());
    }
    revoke_collection(&txn, account, &receipt.granted)?;
    state.tender = state
        .tender
        .checked_add(receipt.price)
        .ok_or("Tender refund overflow.")?;
    state.purchases.remove(&vendor);
    write_state(&txn, account, &state);
    txn.commit().expect("commit atomic Trading Post refund");
    Ok(state)
}

/// Admin-socket test setup only; players earn Tender through `settle`.
pub(crate) fn grant_tender(db: &Database, account: u64, amount: u32) -> Result<State, String> {
    let txn = db.begin_write().expect("begin Trading Post Tender grant");
    let mut state = read_state(&txn, account);
    state.tender = state
        .tender
        .checked_add(amount)
        .ok_or("Tender balance overflow.")?;
    write_state(&txn, account, &state);
    txn.commit().expect("commit Trading Post Tender grant");
    Ok(state)
}

pub(crate) fn freeze(db: &Database, account: u64, vendor: Option<u32>) -> State {
    let txn = db.begin_write().expect("begin Trading Post freeze");
    let mut state = read_state(&txn, account);
    state.frozen = vendor;
    write_state(&txn, account, &state);
    txn.commit().expect("commit Trading Post freeze");
    state
}

fn read_json<T: serde::de::DeserializeOwned + Default>(
    txn: &WriteTransaction,
    table: TableDefinition<u64, &[u8]>,
    account: u64,
) -> T {
    let table = txn.open_table(table).expect("open account collection");
    table
        .get(account)
        .expect("read account collection")
        .map(|row| serde_json::from_slice(row.value()).expect("decode account collection"))
        .unwrap_or_default()
}

fn write_json<T: Serialize>(
    txn: &WriteTransaction,
    table: TableDefinition<u64, &[u8]>,
    account: u64,
    value: &T,
) {
    let bytes = serde_json::to_vec(value).expect("encode account collection");
    txn.open_table(table)
        .expect("open account collection")
        .insert(account, bytes.as_slice())
        .expect("write account collection");
}

fn change_favourite_collection(
    txn: &WriteTransaction,
    table: TableDefinition<u64, &[u8]>,
    account: u64,
    id: u32,
    learn: bool,
) -> Result<(), String> {
    let mut table = txn
        .open_table(table)
        .expect("open account mount/toy collection");
    let mut owned: BTreeMap<u32, bool> = table
        .get(account)
        .expect("read account mount/toy collection")
        .map(|row| bitcode::decode(row.value()).expect("decode account mount/toy collection"))
        .unwrap_or_default();
    if learn {
        if owned.contains_key(&id) {
            return Err("This collection item is already known.".into());
        }
        owned.insert(id, false);
    } else {
        owned.remove(&id);
    }
    table
        .insert(account, bitcode::encode(&owned).as_slice())
        .expect("write account mount/toy collection");
    Ok(())
}

fn grant_collection(txn: &WriteTransaction, account: u64, grant: &Grant) -> Result<Grant, String> {
    use crate::persistence::{
        appearance_store::ACCOUNT_APPEARANCES, mount_store::ACCOUNT_MOUNTS, toy_store::ACCOUNT_TOYS,
    };
    match grant {
        Grant::Appearances(ids) => {
            let mut owned: shared::transmog::AppearanceCollection =
                read_json(txn, ACCOUNT_APPEARANCES, account);
            let added: Vec<_> = ids.iter().copied().filter(|id| owned.learn(*id)).collect();
            if added.is_empty() {
                return Err("All these appearances are already collected.".into());
            }
            write_json(txn, ACCOUNT_APPEARANCES, account, &owned);
            Ok(Grant::Appearances(added))
        }
        Grant::Mount(id) => {
            change_favourite_collection(txn, ACCOUNT_MOUNTS, account, *id, true)?;
            Ok(grant.clone())
        }
        Grant::Toy(id) => {
            change_favourite_collection(txn, ACCOUNT_TOYS, account, *id, true)?;
            Ok(grant.clone())
        }
        Grant::Pet(species) => grant_pet(txn, account, *species),
        Grant::PetInstance(_) => {
            Err("A pet instance is a receipt, not a purchasable grant.".into())
        }
    }
}

fn grant_pet(txn: &WriteTransaction, account: u64, species: u32) -> Result<Grant, String> {
    use crate::battle_pets::ACCOUNT_PET_JOURNALS;
    let mut journal: shared::pet_battle::PetJournal = read_json(txn, ACCOUNT_PET_JOURNALS, account);
    let mut ids = txn
        .open_table(crate::persistence::NEXT_IDS)
        .expect("open pet GUID allocator");
    let guid = ids
        .get("battle_pet_instance")
        .expect("read pet GUID")
        .map_or(1, |row| row.value() + 1);
    // Existing battle_pets::learn_companion_from_item semantics: level1, Common, balanced breed3.
    journal
        .add_with_guid(species, 1, shared::pet_battle::PetQuality::Common, 3, guid)
        .map_err(|reason| format!("Cannot collect Trading Post pet: {reason:?}"))?;
    ids.insert("battle_pet_instance", guid)
        .expect("persist pet GUID");
    write_json(txn, ACCOUNT_PET_JOURNALS, account, &journal);
    Ok(Grant::PetInstance(guid))
}

fn revoke_collection(txn: &WriteTransaction, account: u64, granted: &Grant) -> Result<(), String> {
    use crate::persistence::{
        appearance_store::ACCOUNT_APPEARANCES, mount_store::ACCOUNT_MOUNTS, toy_store::ACCOUNT_TOYS,
    };
    match granted {
        Grant::Appearances(ids) => {
            let mut owned: shared::transmog::AppearanceCollection =
                read_json(txn, ACCOUNT_APPEARANCES, account);
            for id in ids {
                owned.remove(*id);
            }
            write_json(txn, ACCOUNT_APPEARANCES, account, &owned);
        }
        Grant::Mount(id) => change_favourite_collection(txn, ACCOUNT_MOUNTS, account, *id, false)?,
        Grant::Toy(id) => change_favourite_collection(txn, ACCOUNT_TOYS, account, *id, false)?,
        Grant::PetInstance(id) => {
            let mut journal: shared::pet_battle::PetJournal =
                read_json(txn, crate::battle_pets::ACCOUNT_PET_JOURNALS, account);
            journal
                .remove(*id)
                .map_err(|reason| format!("Cannot refund pet instance: {reason:?}"))?;
            write_json(
                txn,
                crate::battle_pets::ACCOUNT_PET_JOURNALS,
                account,
                &journal,
            );
        }
        Grant::Pet(_) => return Err("Refund receipt is missing its granted pet instance.".into()),
    }
    Ok(())
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

pub(crate) fn load(db: &Database, account: u64) -> State {
    let txn = db.begin_read().expect("begin Trading Post account read");
    let table = match txn.open_table(ACCOUNTS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return State::default(),
        Err(error) => panic!("open Trading Post account: {error}"),
    };
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
    fn buy(
        db: &Database,
        account: u64,
        vendor: u32,
        price: u32,
        grant: &Grant,
        now: i64,
        refund_duration: u32,
    ) -> Result<State, String> {
        purchase(
            db,
            account,
            Offer {
                vendor,
                price,
                grant,
                refund_duration,
            },
            now,
        )
    }
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
    fn trading_post_purchase_atomic_collection_grants_and_refund_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("purchase.redb");
        let october = 1790812800;
        let november = 1793491200;
        let now = november + 100;
        {
            let db = Database::create(&path).unwrap();
            crate::persistence::create_tables(&db);
            settle(&db, 7, october, 1000, &thresholds());
            settle(&db, 7, november, 1000, &thresholds());
            let paid = buy(&db, 7, 4, 100, &Grant::Appearances(vec![37866]), now, 0)
                .expect("authentic Warcloak grant");
            assert_eq!(paid.tender, 900);
            assert!(
                crate::persistence::appearance_store::load_account_appearances(&db, 7).has(37866)
            );
            assert!(buy(&db, 7, 4, 100, &Grant::Appearances(vec![37866]), now, 0).is_err());
            assert!(
                refund(&db, 7, 4, now).is_err(),
                "unsourced refund duration stays disabled"
            );
            let state = buy(&db, 7, 73, 700, &Grant::Mount(1574), now, 60).unwrap();
            assert_eq!(state.tender, 200);
            assert!(crate::persistence::mount_store::load(&db, 7).contains_key(&1574));
            assert!(buy(&db, 7, 473, 500, &Grant::Toy(206268), now, 0).is_err());
            assert!(
                crate::persistence::toy_store::load(&db, 7).is_empty(),
                "failed charge cannot grant"
            );
            let refunded = refund(&db, 7, 73, now + 59).unwrap();
            assert_eq!(refunded.tender, 900);
            assert!(!crate::persistence::mount_store::load(&db, 7).contains_key(&1574));
            assert!(refund(&db, 7, 73, now + 59).is_err());
            buy(&db, 7, 73, 700, &Grant::Mount(1574), now, 60).unwrap();
            assert!(
                refund(&db, 7, 73, now + 60).is_err(),
                "exclusive expiry boundary"
            );
            freeze(&db, 7, Some(80));
            settle(&db, 7, november + 30 * 86400, 1000, &thresholds());
            buy(&db, 7, 80, 600, &Grant::Pet(3250), now, 60).unwrap();
            assert_eq!(
                crate::battle_pets::load_journal(&db, 7).count_species(3250),
                1
            );
            refund(&db, 7, 80, now + 1).unwrap();
            assert_eq!(
                crate::battle_pets::load_journal(&db, 7).count_species(3250),
                0
            );
            buy(&db, 7, 473, 500, &Grant::Toy(206268), now, 60).unwrap();
            assert!(crate::persistence::toy_store::load(&db, 7).contains_key(&206268));
        }
        let db = Database::open(&path).unwrap();
        let state = settle(&db, 7, november + 30 * 86400, 0, &thresholds());
        assert_eq!(state.frozen, Some(80));
        assert_eq!(state.tender, 200);
        assert!(crate::persistence::appearance_store::load_account_appearances(&db, 7).has(37866));
        assert!(crate::persistence::mount_store::load(&db, 7).contains_key(&1574));
        assert!(crate::persistence::toy_store::load(&db, 7).contains_key(&206268));
        assert!(crate::persistence::mount_store::load(&db, 8).is_empty());
        refund(&db, 7, 473, now + 1).unwrap();
        assert!(!crate::persistence::toy_store::load(&db, 7).contains_key(&206268));
    }

    #[test]
    fn trading_post_checkout_commits_all_receipts_and_refunds_each_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("checkout.redb");
        let now = 1790812900;
        {
            let db = Database::create(&path).unwrap();
            crate::persistence::create_tables(&db);
            grant_tender(&db, 7, 1000).unwrap();
            let offers = [
                Offer {
                    vendor: 4,
                    price: 100,
                    grant: &Grant::Appearances(vec![37866]),
                    refund_duration: 60,
                },
                Offer {
                    vendor: 473,
                    price: 500,
                    grant: &Grant::Toy(206268),
                    refund_duration: 60,
                },
            ];
            let paid = checkout(&db, 7, &offers, now).unwrap();
            assert_eq!(paid.tender, 400);
            assert_eq!(paid.purchases.len(), 2);
            assert!(
                crate::persistence::appearance_store::load_account_appearances(&db, 7).has(37866)
            );
            assert!(crate::persistence::toy_store::load(&db, 7).contains_key(&206268));
        }
        let db = Database::open(&path).unwrap();
        assert_eq!(load(&db, 7).tender, 400);
        assert_eq!(refund(&db, 7, 4, now + 1).unwrap().tender, 500);
        assert!(!crate::persistence::appearance_store::load_account_appearances(&db, 7).has(37866));
        assert!(crate::persistence::toy_store::load(&db, 7).contains_key(&206268));
        assert_eq!(refund(&db, 7, 473, now + 1).unwrap().tender, 1000);
        assert!(!crate::persistence::toy_store::load(&db, 7).contains_key(&206268));
    }

    #[test]
    fn trading_post_checkout_failure_after_first_offer_cannot_commit_charge_or_collection() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        let now = 1790812900;
        grant_tender(&db, 7, 2000).unwrap();
        buy(&db, 7, 73, 700, &Grant::Mount(1574), now, 60).unwrap();
        let before = load(&db, 7);
        let cloak = Offer {
            vendor: 4,
            price: 100,
            grant: &Grant::Appearances(vec![37866]),
            refund_duration: 60,
        };
        let owned = Offer {
            vendor: 73,
            price: 700,
            grant: &Grant::Mount(1574),
            refund_duration: 60,
        };
        assert!(checkout(&db, 7, &[cloak, owned], now).is_err());
        assert_eq!(load(&db, 7), before);
        assert!(!crate::persistence::appearance_store::load_account_appearances(&db, 7).has(37866));
        assert!(crate::persistence::mount_store::load(&db, 7).contains_key(&1574));
        let duplicate = [
            Offer {
                vendor: 4,
                price: 100,
                grant: &Grant::Appearances(vec![37866]),
                refund_duration: 60,
            },
            Offer {
                vendor: 4,
                price: 100,
                grant: &Grant::Appearances(vec![37866]),
                refund_duration: 60,
            },
        ];
        assert!(checkout(&db, 7, &duplicate, now).is_err());
        assert_eq!(load(&db, 7), before);
        assert!(!crate::persistence::appearance_store::load_account_appearances(&db, 7).has(37866));
        assert!(checkout(&db, 7, &[], now).is_err());
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
