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

fn fixture() -> (Config, EconomyState) {
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
