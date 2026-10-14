use super::*;
use crate::economy::{
    Config, Economy,
    model::{EconomyState, MerchantConfig, MerchantState, StockBatch},
    store,
};

fn economic_shop() -> (Harness, persistence::InventoryData) {
    let mut h = harness().expect("real world.db required for economic vendor test");
    let mut state = EconomyState::default();
    state.merchants.insert(
        GODRIC_ROTHGAR,
        MerchantState {
            gold: 950,
            stock: vec![StockBatch {
                id: 7,
                item_id: 2302,
                count: 2,
                ingredient_cost: 40,
                unit_price: 100,
                expires_at: crate::auction_house::unix_now() + 86400,
            }],
            ..Default::default()
        },
    );
    let config = Config {
        cycle_secs: 60,
        gatherer_listing_limit: 0,
        gatherer_stack_max: 1,
        gatherer_capital: 0,
        gatherables: Vec::new(),
        vendor_supplies: Default::default(),
        customer_budget: 0,
        customer_quantity: 0,
        items: Default::default(),
        markets: Default::default(),
        merchants: vec![MerchantConfig {
            entry: GODRIC_ROTHGAR,
            expansion: 0,
            recipes: Vec::new(),
            capital_limit: 1000,
            capacity: 2,
        }],
    };
    let db = &h.app.world().resource::<GameDatabase>().db;
    store::init_state_table(db).unwrap();
    store::commit_market(db, Some(&state), &store::MarketChanges::default()).unwrap();
    let before = persistence::load_inventory(db, h.character_id);
    h.app
        .insert_resource(config)
        .insert_resource(Economy::from_state(state));
    (h, before)
}

#[test]
fn older_economy_receipt_cannot_overwrite_a_later_committed_merchant_purchase() {
    let (mut h, _) = economic_shop();
    h.app
        .init_resource::<crate::auction_house::AuctionHouseState>();
    h.app
        .init_resource::<crate::auction_house::PendingMarketCommits>();
    let state = h.app.world().resource::<Economy>().state.clone();
    h.app.world_mut().resource_scope(
        |world, mut pending: bevy::prelude::Mut<crate::auction_house::PendingMarketCommits>| {
            crate::auction_house::enqueue_market_commit(
                world.resource::<GameDatabase>(),
                &mut pending,
                None,
                "preceding economy publication",
                move |db| {
                    store::commit_market(db, Some(&state), &store::MarketChanges::default())?;
                    let mut publication = crate::auction_house::MarketPublication::from_auctions(
                        crate::auction_house::load_auction_state(db),
                    );
                    publication.economy = Some(state);
                    Ok(publication)
                },
            );
        },
    );
    let npc = h.open(GODRIC_ROTHGAR);
    h.buy(npc, 7, 2302, 1);
    assert!(h.failures().is_empty());
    assert_eq!(
        store::load_state(&h.app.world().resource::<GameDatabase>().db)
            .unwrap()
            .unwrap()
            .merchants[&GODRIC_ROTHGAR]
            .stock[0]
            .count,
        1
    );
    crate::auction_house::tests::publish_market_once(&mut h.app);
    assert_eq!(
        h.app.world().resource::<Economy>().state.merchants[&GODRIC_ROTHGAR].stock[0].count,
        1
    );
}

#[test]
fn economic_merchant_purchase_uses_finite_stock_and_atomic_player_payment() {
    let (mut h, before) = economic_shop();
    let npc = h.open(GODRIC_ROTHGAR);
    let list = h
        .app
        .world()
        .resource::<Inbox<VendorInventory>>()
        .0
        .last()
        .unwrap();
    assert_eq!(
        list.items.len(),
        1,
        "economic catalog replaces the static vendor goods"
    );
    assert_eq!(
        (
            list.items[0].slot,
            list.items[0].item_id,
            list.items[0].price,
            list.items[0].num_available
        ),
        (7, 2302, 100, Some(2))
    );
    h.buy(npc, 7, 2302, 1);
    let state = &h.app.world().resource::<Economy>().state;
    assert_eq!(state.merchants[&GODRIC_ROTHGAR].stock[0].count, 1);
    assert_eq!(
        state.merchants[&GODRIC_ROTHGAR].stock[0].ingredient_cost,
        20
    );
    assert_eq!(state.merchants[&GODRIC_ROTHGAR].gold, 1000);
    assert_eq!(state.withdrawn, 50);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let after = persistence::load_inventory(db, h.character_id);
    assert_eq!(after.gold, before.gold - 100);
    assert_eq!(
        after
            .items
            .iter()
            .filter(|item| item.item_id == 2302)
            .map(|item| item.stack_count)
            .sum::<u32>(),
        1
    );
    assert_eq!(store::load_state(db).unwrap().unwrap(), *state);
    let saved = state.clone();
    h.buy(npc, 7, 2302, 2);
    assert_eq!(h.app.world().resource::<Economy>().state, saved);
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id),
        after
    );
    assert_eq!(
        h.app
            .world()
            .resource::<Inbox<MerchantFailed>>()
            .0
            .last()
            .unwrap()
            .error,
        MerchantError::SoldOut
    );
}

#[test]
fn source_requirements_economic_merchant_unique_limit_counts_producer_retail_only() {
    for id in [2947, 2512, 2101] {
        let mut h = harness().expect("world vendor fixture required");
        h.source_collision(id);
        let mut state = EconomyState::default();
        state.merchants.insert(
            GODRIC_ROTHGAR,
            MerchantState {
                gold: 0,
                stock: vec![StockBatch {
                    id: 7,
                    item_id: id,
                    count: 2,
                    ingredient_cost: 0,
                    unit_price: 1,
                    expires_at: crate::auction_house::unix_now() + 86400,
                }],
                ..Default::default()
            },
        );
        let config = Config {
            cycle_secs: 60,
            gatherer_listing_limit: 0,
            gatherer_stack_max: 1,
            gatherer_capital: 0,
            gatherables: Vec::new(),
            vendor_supplies: Default::default(),
            customer_budget: 0,
            customer_quantity: 0,
            items: Default::default(),
            markets: Default::default(),
            merchants: vec![MerchantConfig {
                entry: GODRIC_ROTHGAR,
                expansion: 0,
                recipes: Vec::new(),
                capital_limit: 1000,
                capacity: 2,
            }],
        };
        let db = &h.app.world().resource::<GameDatabase>().db;
        store::init_state_table(db).unwrap();
        store::commit_market(db, Some(&state), &store::MarketChanges::default()).unwrap();
        h.app
            .insert_resource(config)
            .insert_resource(Economy::from_state(state));
        let npc = h.open(GODRIC_ROTHGAR);
        h.buy(npc, 7, id, 1);
        assert!(
            h.failures().is_empty(),
            "Forever {id} must not block Retail stock"
        );
        assert_eq!(h.count_of(id), vec![1, 1]);
        let before = h.inventory();
        let stock_before = h.app.world().resource::<Economy>().state.clone();
        h.buy(npc, 7, id, 1);
        assert_eq!(h.failures(), vec![MerchantError::CantCarryMore]);
        assert_eq!(h.inventory(), before);
        assert_eq!(h.app.world().resource::<Economy>().state, stock_before);
    }
}
