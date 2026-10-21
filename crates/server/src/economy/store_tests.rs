use super::*;
use crate::persistence::inventory_store::{ItemInstanceData, ItemPlacementData};
use crate::persistence::mail_store::{MAILBOXES, MailSender};
use crate::persistence::{CharacterData, InventoryItemData};
use redb::{ReadableDatabase, ReadableTableMetadata};
use std::collections::HashMap;
use std::sync::{Arc, Barrier};

fn item() -> InventoryItemData {
    InventoryItemData {
        item_guid: 501,
        item_id: 2770,
        name: "Copper Ore".into(),
        quality: 1,
        required_level: 1,
        stack_count: 4,
        vendor_sell_price: 5,
        unique_auction: false,
    }
}

fn auction(id: u64) -> AuctionData {
    AuctionData {
        auction_id: id,
        seller_character_id: 7,
        seller_name: "Miner".into(),
        item: item(),
        stack_count: 4,
        min_bid: 200,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(400),
        expires_at_unix_secs: 1800000000,
        faction: 1,
        deposit: 20,
    }
}

fn mail(id: u64) -> MailData {
    MailData {
        mail_id: id,
        sender: MailSender::AuctionHouse,
        subject: "Auction sold".into(),
        body: "Copper Ore".into(),
        items: vec![],
        money: 380,
        cod: 0,
        delivers_at: 1700000000,
        expires_at: 1800000000,
        read: false,
        returned: false,
        cod_payment: false,
    }
}

fn state(cycle: u64) -> EconomyState {
    EconomyState {
        last_cycle: cycle,
        next_batch_id: 11,
        customer_spent: 400,
        ..Default::default()
    }
}

fn read_row<T: bitcode::DecodeOwned>(
    db: &Database,
    definition: TableDefinition<u64, &[u8]>,
    key: u64,
) -> Option<T> {
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(definition).unwrap();
    table
        .get(key)
        .unwrap()
        .map(|row| bitcode::decode(row.value()).unwrap())
}

#[test]
fn missing_state_is_none_and_initialization_is_idempotent() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    assert_eq!(load_state(&db).unwrap(), None);
    init_state_table(&db).unwrap();
    init_state_table(&db).unwrap();
    assert_eq!(load_state(&db).unwrap(), None);
}

#[test]
fn committed_market_reopens_with_state_auctions_inventory_and_appended_mail() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    let old = auction(1);
    commit_market(
        &db,
        Some(&state(1)),
        &MarketChanges {
            auctions: vec![old.clone()],
            mails: vec![(7, mail(1))],
            ..Default::default()
        },
    )
    .unwrap();
    let inventory = InventoryData {
        gold: 600,
        items: vec![item()],
    };
    commit_market(
        &db,
        Some(&state(2)),
        &MarketChanges {
            expected_auctions: vec![(1, old)],
            deleted_auctions: vec![1],
            auctions: vec![auction(2)],
            inventories: vec![(7, inventory.clone())],
            mails: vec![(7, mail(2)), (7, mail(3))],
            ..Default::default()
        },
    )
    .unwrap();
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(load_state(&db).unwrap(), Some(state(2)));
    assert_eq!(read_row::<AuctionData>(&db, AUCTIONS, 1), None);
    assert_eq!(read_row::<AuctionData>(&db, AUCTIONS, 2), Some(auction(2)));
    assert_eq!(
        read_row::<InventoryData>(&db, INVENTORIES, 7),
        Some(inventory)
    );
    assert_eq!(
        read_row::<Vec<MailData>>(&db, MAILBOXES, 7),
        Some(vec![mail(1), mail(2), mail(3)])
    );
}

#[test]
fn mailbox_replacement_precedes_appending_new_mail() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    commit_market(
        &db,
        None,
        &MarketChanges {
            mails: vec![(7, mail(1)), (7, mail(2))],
            ..Default::default()
        },
    )
    .unwrap();
    commit_market(
        &db,
        Some(&state(2)),
        &MarketChanges {
            mailboxes: vec![(7, vec![mail(2)])],
            mails: vec![(7, mail(3))],
            ..Default::default()
        },
    )
    .unwrap();
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(load_state(&db).unwrap(), Some(state(2)));
    assert_eq!(
        read_row::<Vec<MailData>>(&db, MAILBOXES, 7),
        Some(vec![mail(2), mail(3)])
    );
}

#[test]
fn conditional_update_refuses_stale_row_and_preserves_inputs() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    let old = auction(1);
    commit_market(
        &db,
        Some(&state(1)),
        &MarketChanges {
            auctions: vec![old.clone()],
            ..Default::default()
        },
    )
    .unwrap();
    let mut updated = old.clone();
    updated.current_bid = Some(250);
    let changes = MarketChanges {
        expected_auctions: vec![(1, old.clone())],
        auctions: vec![updated.clone()],
        ..Default::default()
    };
    commit_market(&db, None, &changes).unwrap();
    let candidate = state(2);
    let error = commit_market(&db, Some(&candidate), &changes).unwrap_err();
    assert!(error.contains("auction 1"), "{error}");
    assert_eq!(candidate, state(2));
    assert_eq!(changes.expected_auctions[0].1, old);
    assert_eq!(changes.auctions[0], updated);
    assert_eq!(load_state(&db).unwrap(), Some(state(1)));
    assert_eq!(read_row::<AuctionData>(&db, AUCTIONS, 1), Some(updated));
}

#[test]
fn concurrent_duplicate_purchase_commits_only_one_mail_and_inventory_debit() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Arc::new(Database::create(file.path()).unwrap());
    commit_market(
        &db,
        Some(&state(1)),
        &MarketChanges {
            auctions: vec![auction(1)],
            ..Default::default()
        },
    )
    .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let db = Arc::clone(&db);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let changes = MarketChanges {
                    expected_auctions: vec![(1, auction(1))],
                    deleted_auctions: vec![1],
                    inventories: vec![(
                        7,
                        InventoryData {
                            gold: 600,
                            items: vec![],
                        },
                    )],
                    mails: vec![(7, mail(1))],
                    ..Default::default()
                };
                barrier.wait();
                commit_market(&db, Some(&state(2)), &changes)
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let db = Arc::try_unwrap(db).unwrap();
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(load_state(&db).unwrap(), Some(state(2)));
    assert_eq!(read_row::<AuctionData>(&db, AUCTIONS, 1), None);
    assert_eq!(
        read_row::<InventoryData>(&db, INVENTORIES, 7).unwrap().gold,
        600
    );
    assert_eq!(
        read_row::<Vec<MailData>>(&db, MAILBOXES, 7),
        Some(vec![mail(1)])
    );
}

#[test]
fn corrupt_mailbox_aborts_prior_writes_in_same_transaction() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    let initial_inventory = InventoryData {
        gold: 1000,
        items: vec![],
    };
    commit_market(
        &db,
        Some(&state(1)),
        &MarketChanges {
            auctions: vec![auction(1)],
            inventories: vec![(7, initial_inventory.clone())],
            ..Default::default()
        },
    )
    .unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(MAILBOXES)
        .unwrap()
        .insert(8, b"corrupt".as_slice())
        .unwrap();
    txn.commit().unwrap();
    let changes = MarketChanges {
        expected_auctions: vec![(1, auction(1))],
        deleted_auctions: vec![1],
        inventories: vec![(
            7,
            InventoryData {
                gold: 600,
                items: vec![],
            },
        )],
        mails: vec![(7, mail(1)), (8, mail(2))],
        ..Default::default()
    };
    let error = commit_market(&db, Some(&state(2)), &changes).unwrap_err();
    assert!(error.contains("mailbox 8"), "{error}");
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(load_state(&db).unwrap(), Some(state(1)));
    assert_eq!(read_row::<AuctionData>(&db, AUCTIONS, 1), Some(auction(1)));
    assert_eq!(
        read_row::<InventoryData>(&db, INVENTORIES, 7),
        Some(initial_inventory)
    );
    assert_eq!(read_row::<Vec<MailData>>(&db, MAILBOXES, 7), None);
}

#[test]
fn dropping_uncommitted_transaction_preserves_reopened_state() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    commit_market(&db, Some(&state(1)), &MarketChanges::default()).unwrap();
    {
        let txn = db.begin_write().unwrap();
        txn.open_table(STATE)
            .unwrap()
            .insert(0, bitcode::encode(&state(2)).as_slice())
            .unwrap();
    }
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(load_state(&db).unwrap(), Some(state(1)));
}

#[test]
fn corrupt_state_is_error_not_missing_state() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    let txn = db.begin_write().unwrap();
    txn.open_table(STATE)
        .unwrap()
        .insert(0, b"corrupt".as_slice())
        .unwrap();
    txn.commit().unwrap();
    assert!(load_state(&db).unwrap_err().contains("economy state"));
}

#[test]
fn record_helper_persists_layout_instances_character_and_inventory() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();
    let record = InventoryRecord {
        character: CharacterData {
            name: "Miner".into(),
            x: 1.0,
            y: 2.0,
            z: 3.0,
            health: 100.0,
            max_health: 100.0,
            race: 1,
            class: 1,
            level: 10,
            appearance: Default::default(),
            equipment_appearance: Default::default(),
            durability: vec![],
        },
        inventory: InventoryData {
            gold: 600,
            items: vec![item()],
        },
        placements: vec![ItemPlacementData {
            item_guid: 501,
            bag: 0,
            slot: 1,
        }],
        equipped: vec![],
        bag_slots: vec![],
        nonvisual_equipped: Vec::new(),
        instances: HashMap::from([(501, ItemInstanceData { soulbound: true })]),
        crafting_origins: HashMap::new(),
        item_modifiers: HashMap::new(),
        item_durability: HashMap::from([(501, (44, 55))]),
        definition_sources: HashMap::from([(501, shared::item_data::ItemDefinitionSource::Retail)]),
        appearance_account: None,
        appearance_grants: Vec::new(),
        transmogs: Default::default(),
    };
    commit_market(
        &db,
        Some(&state(1)),
        &MarketChanges {
            records: vec![(7, record.clone())],
            ..Default::default()
        },
    )
    .unwrap();
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(
        crate::persistence::inventory_store::load_inventory_record(&db, 7),
        Some(record)
    );
    assert_eq!(load_state(&db).unwrap(), Some(state(1)));
}

#[test]
fn inventory_snapshot_guard_rejects_missing_changed_and_corrupt_rows_before_settlement() {
    for scenario in ["missing", "changed", "corrupt"] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        let expected = InventoryData {
            gold: 1_000,
            items: vec![item()],
        };
        commit_market(
            &db,
            Some(&state(1)),
            &MarketChanges {
                inventories: vec![(7, expected.clone())],
                auctions: vec![auction(1)],
                mails: vec![(7, mail(1))],
                ..Default::default()
            },
        )
        .unwrap();
        let tx = db.begin_write().unwrap();
        {
            let mut table = tx.open_table(INVENTORIES).unwrap();
            match scenario {
                "missing" => {
                    table.remove(7).unwrap();
                }
                "changed" => {
                    table
                        .insert(
                            7,
                            bitcode::encode(&InventoryData {
                                gold: 999,
                                ..expected.clone()
                            })
                            .as_slice(),
                        )
                        .unwrap();
                }
                _ => {
                    table.insert(7, b"corrupt".as_slice()).unwrap();
                }
            }
        }
        tx.commit().unwrap();
        let result = commit_market(
            &db,
            Some(&state(2)),
            &MarketChanges {
                expected_inventories: vec![(7, expected)],
                inventories: vec![(
                    7,
                    InventoryData {
                        gold: 600,
                        items: vec![],
                    },
                )],
                auctions: vec![auction(2)],
                mails: vec![(7, mail(2))],
                ..Default::default()
            },
        );
        let error = result.unwrap_err();
        assert!(error.contains("inventory 7"), "{error}");
        drop(db);
        let db = Database::open(file.path()).unwrap();
        assert_eq!(load_state(&db).unwrap(), Some(state(1)));
        let tx = db.begin_read().unwrap();
        let inventories = tx.open_table(INVENTORIES).unwrap();
        let row = inventories.get(7).unwrap();
        match scenario {
            "missing" => assert!(row.is_none()),
            "changed" => assert_eq!(
                bitcode::decode::<InventoryData>(row.unwrap().value())
                    .unwrap()
                    .gold,
                999
            ),
            _ => assert_eq!(row.unwrap().value(), b"corrupt"),
        }
        let auctions = tx.open_table(AUCTIONS).unwrap();
        assert!(auctions.get(2).unwrap().is_none());
        assert_eq!(read_row::<AuctionData>(&db, AUCTIONS, 1), Some(auction(1)));
        assert_eq!(
            read_row::<Vec<MailData>>(&db, MAILBOXES, 7),
            Some(vec![mail(1)])
        );
    }
}

#[test]
fn concurrent_inventory_guard_lists_same_source_exactly_once_after_reopen() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Arc::new(Database::create(file.path()).unwrap());
    let inventory = InventoryData {
        gold: 1_000,
        items: vec![item()],
    };
    commit_market(
        &db,
        Some(&state(1)),
        &MarketChanges {
            inventories: vec![(7, inventory.clone())],
            ..Default::default()
        },
    )
    .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (1..=2)
        .map(|id| {
            let db = db.clone();
            let barrier = barrier.clone();
            let expected = inventory.clone();
            std::thread::spawn(move || {
                let changes = MarketChanges {
                    expected_inventories: vec![(7, expected)],
                    inventories: vec![(
                        7,
                        InventoryData {
                            gold: 980,
                            items: vec![],
                        },
                    )],
                    auctions: vec![auction(id)],
                    mails: vec![(7, mail(id))],
                    ..Default::default()
                };
                barrier.wait();
                commit_market(&db, Some(&state(2)), &changes)
            })
        })
        .collect();
    let successes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .filter(|result| result.is_ok())
        .count();
    assert_eq!(successes, 1);
    drop(db);
    let db = Database::open(file.path()).unwrap();
    assert_eq!(load_state(&db).unwrap(), Some(state(2)));
    let tx = db.begin_read().unwrap();
    assert_eq!(tx.open_table(AUCTIONS).unwrap().len().unwrap(), 1);
    assert_eq!(
        read_row::<InventoryData>(&db, INVENTORIES, 7),
        Some(InventoryData {
            gold: 980,
            items: vec![]
        })
    );
    assert_eq!(
        read_row::<Vec<MailData>>(&db, MAILBOXES, 7).unwrap().len(),
        1
    );
}
