use super::*;
use crate::persistence;
use model::{MerchantState, Recipe, StockBatch};

#[test]
fn economy_status_reports_exact_state_without_mutation() {
    let (mut config, mut state) = fixture();
    config.merchants[0].entry = 1295;
    config.merchants[0].capital_limit = 9000;
    state.merchants.clear();
    state.merchants.insert(
        1295,
        MerchantState {
            gold: 4567,
            stock: vec![StockBatch {
                id: 12,
                item_id: 2304,
                count: 3,
                ingredient_cost: 47,
                unit_price: 100,
                expires_at: 86460,
            }],
            materials: vec![model::MaterialLot {
                item_id: 2318,
                count: 7,
                ingredient_cost: 123,
            }],
        },
    );
    state.merchants.insert(3532, MerchantState::default());
    state.gatherers.insert(2318, 42);
    state.last_cycle = 60;
    state.next_batch_id = 13;
    state.customer_spent = 234;
    state.refund_issued = 56;
    state.withdrawn = 78;
    let economy = Economy::from_state(state.clone());
    let response = admin_status(Some(&economy), Some(&config));
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::json!({
            "EconomyStatus": { "status": {
                "last_cycle": 60,
                "merchants": [
                    { "entry": 1295, "gold": 4567, "capital_cap": 9000,
                      "stock": [{ "id": 12, "item_id": 2304, "count": 3,
                        "ingredient_cost": 47, "unit_price": 100, "expires_at": 86460 }],
                      "materials": [{ "item_id": 2318, "count": 7, "ingredient_cost": 123 }] },
                    { "entry": 3532, "gold": 0, "capital_cap": null, "stock": [], "materials": [] }
                ],
                "gatherers": { "2318": 42 }, "gatherer_count": 1,
                "customer_spent": 234, "refund_issued": 56, "withdrawn": 78
            }}
        })
    );
    assert_eq!(economy.state, state);
    assert_eq!(config.merchants[0].capital_limit, 9000);
}

#[test]
fn economy_status_reports_missing_initialization() {
    let (config, _) = fixture();
    let uninitialized = Economy::default();
    for (economy, config) in [
        (None, Some(&config)),
        (Some(&uninitialized), Some(&config)),
        (Some(&uninitialized), None),
    ] {
        assert_eq!(
            admin_status(economy, config),
            crate::admin_types::AdminResponse::Error("economy not initialized".into())
        );
    }
    let initialized = Economy::from_state(EconomyState::default());
    assert_eq!(
        admin_status(Some(&initialized), None),
        crate::admin_types::AdminResponse::Error("economy config not loaded".into())
    );
}

pub(super) fn fixture() -> (Config, EconomyState) {
    let recipe = Recipe {
        spell_id: 2152,
        expansion: 0,
        output_item_id: 2304,
        output_count: 1,
        ingredients: vec![(2318, 1)],
    };
    let markets = [(2318, 10), (2304, 100)]
        .into_iter()
        .map(|(item_id, price)| {
            (
                item_id,
                MarketItem {
                    item_id,
                    anchor_price: price,
                    min_price: price,
                    max_price: price,
                    popularity_bps: 10000,
                },
            )
        })
        .collect();
    let config = Config {
        cycle_secs: 60,
        gatherer_listing_limit: 2,
        gatherer_stack_max: 2,
        gatherer_capital: 1000,
        gatherables: vec![2318],
        vendor_supplies: BTreeMap::new(),
        customer_budget: 0,
        customer_quantity: 0,
        items: [(2318, "Light Leather"), (2304, "Light Armor Kit")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    ItemMetadata {
                        name: name.into(),
                        quality: 1,
                        required_level: 1,
                        vendor_sell_price: 1,
                    },
                )
            })
            .collect(),
        markets,
        merchants: vec![MerchantConfig {
            entry: 3532,
            expansion: 0,
            capital_limit: 1000,
            capacity: 2,
            recipes: vec![recipe],
        }],
    };
    let mut state = EconomyState::default();
    state.merchants.insert(
        3532,
        MerchantState {
            gold: 1000,
            ..Default::default()
        },
    );
    (config, state)
}

#[test]
fn economy_config_accepts_u64_input_prices_but_bounds_vendor_outputs() {
    let (mut config, _) = fixture();
    let high = u64::from(u32::MAX) + 10;
    let input = config.markets.get_mut(&2318).unwrap();
    input.anchor_price = high;
    input.min_price = high;
    input.max_price = high;
    assert!(
        config.validate().is_ok(),
        "u64 AH ingredient prices are valid"
    );
    let output = config.markets.get_mut(&2304).unwrap();
    output.anchor_price = u64::from(u32::MAX);
    output.min_price = output.anchor_price;
    output.max_price = output.anchor_price;
    assert!(
        config.validate().is_ok(),
        "u32 maximum is a valid vendor quote"
    );
    config.markets.get_mut(&2304).unwrap().max_price += 1;
    assert!(
        config.validate().is_err(),
        "overflowing vendor output must fail explicitly"
    );
    config.markets.get_mut(&2304).unwrap().max_price -= 1;
    config.markets.get_mut(&2318).unwrap().popularity_bps = 10001;
    assert!(
        config.validate().is_err(),
        "input popularity bounds remain independent of price width"
    );
}

fn supplier_fixture() -> (Config, EconomyState) {
    let (mut config, state) = fixture();
    config.merchants[0].recipes[0].ingredients.push((2320, 2));
    config.items.insert(
        2320,
        ItemMetadata {
            name: "Coarse Thread".into(),
            quality: 1,
            required_level: 0,
            vendor_sell_price: 2,
        },
    );
    config.markets.insert(
        2320,
        MarketItem {
            item_id: 2320,
            anchor_price: 99,
            min_price: 99,
            max_price: 99,
            popularity_bps: 10000,
        },
    );
    let mut json = serde_json::to_value(config).unwrap();
    json["vendor_supplies"] = serde_json::json!({
        "2320": { "vendor_entry": 2225, "slot": 0, "purchase_price": 10,
            "buy_count": 1, "max_count": 0, "restock_secs": 0 }
    });
    (serde_json::from_value(json).unwrap(), state)
}

#[test]
fn economy_supplier_thread_uses_actual_price_with_raw_auction_materials() {
    let db = persistence::temp_database();
    let (config, mut state) = supplier_fixture();
    let mut auctions = AuctionHouseState::default();
    run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 1).unwrap();
    let merchant = &state.merchants[&3532];
    assert_eq!(
        merchant.stock.len(),
        1,
        "thread is bought without an AH offer"
    );
    assert_eq!(merchant.stock[0].ingredient_cost, 30);
    assert_eq!(
        merchant.gold, 960,
        "20 leather stack + 20 actual thread price"
    );
    assert_eq!(
        merchant.materials,
        vec![model::MaterialLot {
            item_id: 2318,
            count: 1,
            ingredient_cost: 10,
        }]
    );
    assert_eq!(store::load_state(&db).unwrap(), Some(state.clone()));
    assert!(auctions.auctions.values().all(|a| a.item.item_id != 2320));
}

#[test]
fn economy_supplier_capital_refusal_keeps_payment_and_materials_unchanged() {
    let db = persistence::temp_database();
    let (mut config, mut state) = supplier_fixture();
    config.gatherables.clear();
    let merchant = state.merchants.get_mut(&3532).unwrap();
    merchant.gold = 19;
    merchant.materials.push(model::MaterialLot {
        item_id: 2318,
        count: 1,
        ingredient_cost: 10,
    });
    let before = merchant.clone();
    run_cycle(
        &db,
        &mut AuctionHouseState::default(),
        &mut state,
        &config,
        1000,
        &mut || 0,
    )
    .unwrap();
    assert_eq!(
        state.merchants[&3532], before,
        "cannot afford two 10-copper purchases"
    );
}

#[test]
fn economy_supplier_never_substitutes_an_auction_when_supplier_is_unaffordable() {
    let db = persistence::temp_database();
    let (config, mut state) = supplier_fixture();
    let mut auctions = AuctionHouseState::default();
    let offer = persistence::AuctionData {
        auction_id: 77,
        seller_character_id: 88,
        seller_name: "Thread seller".into(),
        item: persistence::InventoryItemData {
            item_guid: 99,
            item_id: 2320,
            name: "Coarse Thread".into(),
            quality: 1,
            required_level: 0,
            stack_count: 2,
            vendor_sell_price: 2,
            unique_auction: false,
        },
        stack_count: 2,
        min_bid: 2,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(2),
        expires_at_unix_secs: 2000,
        faction: 0,
        deposit: 0,
    };
    let mut config = config;
    config.gatherables.clear();
    let market = config.markets.get_mut(&2320).unwrap();
    market.anchor_price = 1;
    market.min_price = 1;
    market.max_price = 1;
    state.merchants.get_mut(&3532).unwrap().gold = 19;
    state
        .merchants
        .get_mut(&3532)
        .unwrap()
        .materials
        .push(model::MaterialLot {
            item_id: 2318,
            count: 1,
            ingredient_cost: 10,
        });
    store::commit_market(
        &db,
        Some(&state),
        &store::MarketChanges {
            auctions: vec![offer.clone()],
            ..Default::default()
        },
    )
    .unwrap();
    auctions.auctions.insert(77, offer.clone());
    let before = state.merchants[&3532].clone();
    run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 0).unwrap();
    assert_eq!(state.merchants[&3532], before);
    assert_eq!(auctions.auctions[&77], offer);
    assert_eq!(persistence::load_auctions(&db), vec![offer.clone()]);
    assert!(persistence::mail_store::load_mailbox(&db, 88).is_empty());

    state.merchants.get_mut(&3532).unwrap().gold = 1000;
    run_cycle(&db, &mut auctions, &mut state, &config, 1060, &mut || 0).unwrap();
    assert_eq!(
        state.merchants[&3532].gold, 980,
        "supplier wins even over a cheaper auction"
    );
    assert_eq!(state.merchants[&3532].stock[0].ingredient_cost, 30);
    assert_eq!(auctions.auctions[&77], offer);
    assert!(persistence::mail_store::load_mailbox(&db, 88).is_empty());
}

#[test]
fn economy_supplier_bundle_leftovers_survive_database_reopen_and_refund_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("supplier.redb");
    let db = redb::Database::create(&path).unwrap();
    let (mut config, mut state) = supplier_fixture();
    config.gatherables.clear();
    // Actual Refreshing Spring Water bundle: five items for five copper.
    config.merchants[0].recipes[0].ingredients[1] = (159, 2);
    config.items.insert(
        159,
        ItemMetadata {
            name: "Refreshing Spring Water".into(),
            quality: 1,
            required_level: 0,
            vendor_sell_price: 1,
        },
    );
    config.markets.insert(
        159,
        MarketItem {
            item_id: 159,
            anchor_price: 1,
            min_price: 1,
            max_price: 1,
            popularity_bps: 10000,
        },
    );
    let mut json = serde_json::to_value(&config).unwrap();
    json["vendor_supplies"] = serde_json::json!({
        "159": { "vendor_entry": 2352, "slot": 0, "purchase_price": 5,
            "buy_count": 5, "max_count": 0, "restock_secs": 0 }
    });
    config = serde_json::from_value(json).unwrap();
    state
        .merchants
        .get_mut(&3532)
        .unwrap()
        .materials
        .push(model::MaterialLot {
            item_id: 2318,
            count: 2,
            ingredient_cost: 20,
        });
    let mut auctions = AuctionHouseState::default();
    run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 0).unwrap();
    assert_eq!(state.merchants[&3532].gold, 995);
    assert_eq!(state.merchants[&3532].stock[0].ingredient_cost, 12);
    drop(db);
    let db = redb::Database::open(&path).unwrap();
    let mut restored = store::load_state(&db).unwrap().unwrap();
    run_cycle(&db, &mut auctions, &mut restored, &config, 1001, &mut || 0).unwrap();
    assert_eq!(restored, state, "restart cannot repeat cycle payment");
    run_cycle(&db, &mut auctions, &mut restored, &config, 1060, &mut || 0).unwrap();
    let merchant = &restored.merchants[&3532];
    assert_eq!(
        merchant.gold, 995,
        "second craft uses retained paid lot without charging"
    );
    assert_eq!(
        merchant
            .stock
            .iter()
            .map(|b| b.ingredient_cost)
            .sum::<u64>(),
        24
    );
    assert_eq!(
        merchant.materials,
        vec![model::MaterialLot {
            item_id: 159,
            count: 1,
            ingredient_cost: 1
        }]
    );
    config.merchants[0].recipes.clear();
    run_cycle(
        &db,
        &mut auctions,
        &mut restored,
        &config,
        1000 + model::STOCK_LIFETIME_SECONDS + 60,
        &mut || 0,
    )
    .unwrap();
    assert_eq!(restored.refund_issued, 24);
    assert_eq!(restored.merchants[&3532].gold, 1000);
    assert_eq!(restored.withdrawn, 19);
    assert_eq!(restored.merchants[&3532].materials[0].ingredient_cost, 1);
}

#[test]
fn economy_supplier_full_output_capacity_prevents_procurement() {
    let db = persistence::temp_database();
    let (mut config, mut state) = supplier_fixture();
    config.gatherables.clear();
    state
        .merchants
        .get_mut(&3532)
        .unwrap()
        .stock
        .push(StockBatch {
            id: 1,
            item_id: 2304,
            count: 2,
            ingredient_cost: 20,
            unit_price: 100,
            expires_at: 2000,
        });
    let before = state.merchants[&3532].clone();
    run_cycle(
        &db,
        &mut AuctionHouseState::default(),
        &mut state,
        &config,
        1000,
        &mut || 0,
    )
    .unwrap();
    assert_eq!(state.merchants[&3532], before);
}

#[test]
fn economy_supplier_rejects_limited_stock_without_partial_cycle_commit() {
    let db = persistence::temp_database();
    let (config, mut state) = supplier_fixture();
    let mut json = serde_json::to_value(config).unwrap();
    json["vendor_supplies"]["2320"]["max_count"] = 10.into();
    json["vendor_supplies"]["2320"]["restock_secs"] = 60.into();
    let config: Config = serde_json::from_value(json).unwrap();
    let before = state.clone();
    let mut auctions = AuctionHouseState::default();
    let error = run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 0).unwrap_err();
    assert!(error.contains("limited"), "{error}");
    assert_eq!(state, before);
    assert!(auctions.auctions.is_empty());
    assert!(store::load_state(&db).unwrap().is_none());
    assert!(persistence::load_auctions(&db).is_empty());
}

#[test]
fn economy_supplier_rejects_extended_cost_without_partial_cycle_commit() {
    let db = persistence::temp_database();
    let (config, mut state) = supplier_fixture();
    let mut json = serde_json::to_value(config).unwrap();
    json["vendor_supplies"]["2320"]["extended_cost"] = 42.into();
    let config: Config = serde_json::from_value(json).unwrap();
    let before = state.clone();
    let mut auctions = AuctionHouseState::default();
    let error = run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 0).unwrap_err();
    assert!(error.contains("extended"), "{error}");
    assert_eq!(state, before);
    assert!(auctions.auctions.is_empty());
    assert!(store::load_state(&db).unwrap().is_none());
    assert!(persistence::load_auctions(&db).is_empty());
}

#[test]
fn economy_supplier_failed_cycle_does_not_publish_payment_or_gatherer_supply() {
    let db = persistence::temp_database();
    let (config, mut state) = supplier_fixture();
    state.next_batch_id = u64::MAX;
    let before = state.clone();
    let mut auctions = AuctionHouseState::default();
    let error = run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 1).unwrap_err();
    assert!(error.contains("batch ID"), "{error}");
    assert_eq!(state, before);
    assert!(auctions.auctions.is_empty());
    assert!(store::load_state(&db).unwrap().is_none());
    assert!(persistence::load_auctions(&db).is_empty());
    assert_eq!(persistence::load_inventory(&db, 1).gold, 0);
    assert!(persistence::mail_store::load_mailbox(&db, 1).is_empty());
}

#[test]
fn economy_cycle_buys_real_auction_materials_and_persists_crafted_stock() {
    let db = persistence::temp_database();
    let (config, mut state) = fixture();
    let mut auctions = AuctionHouseState::default();
    run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 1).unwrap();
    let merchant = &state.merchants[&3532];
    assert_eq!(merchant.stock.len(), 1);
    assert_eq!(merchant.stock[0].item_id, 2304);
    assert_eq!(merchant.stock[0].ingredient_cost, 10);
    assert_eq!(merchant.gold, 980);
    assert_eq!(merchant.materials[0].count, 1);
    assert_eq!(merchant.materials[0].ingredient_cost, 10);
    assert_eq!(store::load_state(&db).unwrap(), Some(state.clone()));
    let persisted: BTreeMap<_, _> = persistence::load_auctions(&db)
        .into_iter()
        .map(|auction| (auction.auction_id, auction))
        .collect();
    let runtime: BTreeMap<_, _> = auctions
        .auctions
        .iter()
        .map(|(id, auction)| (*id, auction.clone()))
        .collect();
    assert_eq!(persisted, runtime);
    let saved = state.clone();
    run_cycle(&db, &mut auctions, &mut state, &config, 1001, &mut || 0).unwrap();
    assert_eq!(state, saved, "no repeated spend within the persisted cycle");
}

#[test]
fn economy_expiry_refunds_paid_cost_once_and_never_sale_price() {
    let db = persistence::temp_database();
    let (mut config, mut state) = fixture();
    config.gatherables.clear();
    config.merchants[0].recipes.clear();
    state.merchants.get_mut(&3532).unwrap().gold = 900;
    state
        .merchants
        .get_mut(&3532)
        .unwrap()
        .stock
        .push(StockBatch {
            id: 1,
            item_id: 2304,
            count: 1,
            ingredient_cost: 20,
            unit_price: 100,
            expires_at: 1000,
        });
    let mut auctions = AuctionHouseState::default();
    run_cycle(&db, &mut auctions, &mut state, &config, 1000, &mut || 0).unwrap();
    assert_eq!(state.merchants[&3532].gold, 920);
    assert!(state.merchants[&3532].stock.is_empty());
    assert_eq!(state.refund_issued, 20);
    let mut restored = store::load_state(&db).unwrap().unwrap();
    run_cycle(&db, &mut auctions, &mut restored, &config, 1060, &mut || 0).unwrap();
    assert_eq!(restored.refund_issued, 20);
    assert_eq!(restored.merchants[&3532].gold, 920);
}

#[test]
fn economy_customers_consume_with_global_quantity_and_budget_limits() {
    let db = persistence::temp_database();
    let (mut config, mut state) = fixture();
    config.gatherables.clear();
    config.merchants[0].recipes.clear();
    config.customer_budget = 150;
    config.customer_quantity = 1;
    let merchant = state.merchants.get_mut(&3532).unwrap();
    merchant.gold = 950;
    merchant.stock.push(StockBatch {
        id: 1,
        item_id: 2304,
        count: 2,
        ingredient_cost: 40,
        unit_price: 100,
        expires_at: 2000,
    });
    run_cycle(
        &db,
        &mut AuctionHouseState::default(),
        &mut state,
        &config,
        1000,
        &mut || 0,
    )
    .unwrap();
    assert_eq!(state.customer_spent, 100);
    assert_eq!(state.merchants[&3532].stock[0].count, 1);
    assert_eq!(state.merchants[&3532].gold, 1000);
    assert_eq!(state.withdrawn, 50);
}

fn returned_goods_fixture(
    db: &redb::Database,
) -> (
    Config,
    EconomyState,
    u64,
    Vec<persistence::mail_store::MailData>,
) {
    use persistence::mail_store::{MailData, MailItemData, MailSender};
    let (mut config, mut state) = fixture();
    config.merchants.clear();
    state.merchants.clear();
    let seller = persistence::next_id(db, "character");
    state.gatherers.insert(2318, seller);
    state.last_cycle = 1000;
    let items = [(2318, 4, 15), (2304, 3, 42)].map(|(item_id, count, value)| {
        persistence::InventoryItemData {
            item_guid: persistence::next_id(db, "item_instance"),
            item_id,
            name: config.items[&item_id].name.clone(),
            quality: 1,
            required_level: 1,
            stack_count: count,
            vendor_sell_price: value,
            unique_auction: false,
        }
    });
    let delivered = MailData {
        mail_id: 1,
        sender: MailSender::AuctionHouse,
        subject: "Returned goods".into(),
        body: String::new(),
        items: items
            .into_iter()
            .enumerate()
            .map(|(slot, item)| MailItemData {
                slot: slot as u8,
                item,
            })
            .collect(),
        money: 20,
        cod: 0,
        delivers_at: 1000,
        expires_at: 100_000,
        read: false,
        returned: false,
        cod_payment: false,
    };
    let pending = MailData {
        mail_id: 2,
        delivers_at: 2000,
        ..delivered.clone()
    };
    let mailbox = vec![delivered, pending];
    store::commit_market(
        db,
        Some(&state),
        &store::MarketChanges {
            inventories: vec![(
                seller,
                persistence::InventoryData {
                    gold: 900,
                    items: vec![],
                },
            )],
            mailboxes: vec![(seller, mailbox.clone())],
            ..Default::default()
        },
    )
    .unwrap();
    (config, state, seller, mailbox)
}

#[test]
fn economy_collector_preserves_every_delivered_attachment_and_pending_mail() {
    let db = persistence::temp_database();
    let (mut config, mut state, seller, mailbox) = returned_goods_fixture(&db);
    config.gatherer_listing_limit = 0;
    let mut auctions = AuctionHouseState::default();
    run_cycle(&db, &mut auctions, &mut state, &config, 1060, &mut || {
        panic!("no supply requested")
    })
    .unwrap();
    let expected_items: Vec<_> = mailbox[0]
        .items
        .iter()
        .map(|attachment| attachment.item.clone())
        .collect();
    assert_eq!(
        persistence::load_inventory(&db, seller),
        persistence::InventoryData {
            gold: 920,
            items: expected_items.clone()
        }
    );
    assert_eq!(
        persistence::mail_store::load_mailbox(&db, seller),
        vec![mailbox[1].clone()]
    );
    run_cycle(&db, &mut auctions, &mut state, &config, 1120, &mut || {
        panic!("no supply requested")
    })
    .unwrap();
    assert_eq!(
        persistence::load_inventory(&db, seller).items,
        expected_items
    );
    assert_eq!(persistence::load_inventory(&db, seller).gold, 920);
    assert!(auctions.auctions.is_empty());
}

#[test]
fn economy_failed_collector_commit_keeps_mail_items_and_runtime_state() {
    let db = persistence::temp_database();
    let (mut config, mut state, seller, mailbox) = returned_goods_fixture(&db);
    config.gatherer_listing_limit = 1;
    let before = state.clone();
    let mut auctions = AuctionHouseState::default();
    let mut competing_inventory = persistence::load_inventory(&db, seller);
    competing_inventory.gold -= 1;
    let mut draws = [1, 0, 0].into_iter();
    let error = run_cycle(&db, &mut auctions, &mut state, &config, 1060, &mut || {
        // A real competing inventory write between snapshot and market commit.
        persistence::save_inventory(&db, seller, &competing_inventory);
        draws.next().expect("one attempted listing")
    })
    .unwrap_err();
    assert!(
        error.contains("commit market") && error.contains("changed since"),
        "{error}"
    );
    assert_eq!(state, before);
    assert_eq!(store::load_state(&db).unwrap(), Some(before));
    assert!(auctions.auctions.is_empty());
    assert!(persistence::load_auctions(&db).is_empty());
    assert_eq!(
        persistence::load_inventory(&db, seller),
        competing_inventory
    );
    assert_eq!(persistence::mail_store::load_mailbox(&db, seller), mailbox);
    run_cycle(&db, &mut auctions, &mut state, &config, 1060, &mut || 1).unwrap();
    let listed = auctions.auctions.values().next().unwrap();
    assert_eq!(listed.item.item_id, 2318);
    assert_eq!(listed.stack_count, 2);
    let inventory = persistence::load_inventory(&db, seller);
    assert_eq!(
        inventory.items[0].item_guid,
        mailbox[0].items[0].item.item_guid
    );
    assert_eq!(inventory.items[0].stack_count + listed.stack_count, 4);
    assert_eq!(inventory.items[1], mailbox[0].items[1].item);
    assert_eq!(inventory.gold, 919 - listed.deposit);
}

#[test]
fn achievements_criteria_auctions_native_economy_purchase_credits_only_online_seller() {
    use crate::auction_house::tests::criteria::{prepare_auction_players, progress};
    use shared::protocol::{AuctionChannel, AuctionDuration, CreateAuction};
    for online in [true, false] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let (mut h, _, buyer) = prepare_auction_players(file.path());
        let seller = h.character_id;
        let mut inventory = persistence::load_inventory(h.db(), seller);
        let guid = persistence::next_id(h.db(), "item_instance");
        inventory.items.push(persistence::InventoryItemData {
            item_guid: guid,
            item_id: 2318,
            name: "Light Leather".into(),
            quality: 1,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 15,
            unique_auction: false,
        });
        persistence::save_inventory(h.db(), seller, &inventory);
        h.send::<_, AuctionChannel>(CreateAuction {
            item_guid: guid,
            stack_count: 1,
            min_bid: 1,
            buyout_price: Some(10),
            duration: AuctionDuration::Short,
        });
        let auction = h
            .app
            .world()
            .resource::<AuctionHouseState>()
            .auctions
            .values()
            .next()
            .unwrap()
            .clone();
        if !online {
            h.app
                .world_mut()
                .entity_mut(h.connection)
                .remove::<crate::auth::InWorld>();
        }
        let (mut config, mut state) = fixture();
        config.gatherables.clear();
        h.app
            .world_mut()
            .resource_scope(|world, mut auctions: Mut<AuctionHouseState>| {
                let db = &world.resource::<GameDatabase>().db;
                run_cycle(
                    db,
                    &mut auctions,
                    &mut state,
                    &config,
                    unix_now(),
                    &mut || 0,
                )
                .unwrap();
            });
        for _ in 0..4 {
            h.app.update();
        }
        assert!(persistence::load_auctions(h.db()).is_empty());
        let payout = 10 + auction.deposit;
        assert_eq!(
            persistence::mail_store::load_mailbox(h.db(), seller)[0].money,
            payout
        );
        assert_eq!(progress(&h, seller, 4225), online.then_some(payout));
        assert_eq!(progress(&h, seller, 4223), online.then_some(10));
        assert_eq!(progress(&h, buyer, 4221), None);
        if !online {
            h.app
                .world_mut()
                .entity_mut(h.connection)
                .insert(crate::auth::InWorld {
                    character_id: seller,
                });
            for _ in 0..3 {
                h.app.update();
            }
            assert_eq!(progress(&h, seller, 4225), None);
        }
        drop(h);
        let reopened = redb::Database::open(file.path()).unwrap();
        assert_eq!(
            persistence::load_achievements(&reopened, seller)
                .criteria_progress
                .get(&4225)
                .copied(),
            online.then_some(payout)
        );
    }
}
