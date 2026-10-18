use super::*;
use crate::economy::{
    Config, Economy,
    model::{EconomyState, MerchantConfig, MerchantState, StockBatch},
    store,
};

fn economy_pending_fixture() -> Harness {
    let mut h = harness().expect("world vendor fixture required");
    let mut state = EconomyState::default();
    state.merchants.insert(
        GODRIC_ROTHGAR,
        MerchantState {
            gold: 0,
            stock: vec![StockBatch {
                id: 7,
                item_id: 2302,
                count: 3,
                ingredient_cost: 60,
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
        customer_budget: 100,
        customer_quantity: 1,
        items: Default::default(),
        markets: [(
            2302,
            crate::economy::model::MarketItem {
                item_id: 2302,
                anchor_price: 100,
                min_price: 100,
                max_price: 100,
                popularity_bps: 10000,
            },
        )]
        .into_iter()
        .collect(),
        merchants: vec![MerchantConfig {
            entry: GODRIC_ROTHGAR,
            expansion: 0,
            recipes: Vec::new(),
            capital_limit: 1000,
            capacity: 3,
        }],
    };
    let db = &h.app.world().resource::<GameDatabase>().db;
    store::init_state_table(db).unwrap();
    store::commit_market(db, Some(&state), &store::MarketChanges::default()).unwrap();
    h.app
        .insert_resource(config)
        .insert_resource(Economy::from_state(state));
    h.app
        .init_resource::<crate::auction_house::AuctionHouseState>();
    h.app.add_systems(
        Update,
        crate::economy::publish_economy_market.before(crate::merchant::MerchantRequests),
    );
    h
}

fn hold_economy_cycle(h: &mut Harness, now: u64) -> std::sync::mpsc::Sender<()> {
    use std::sync::mpsc;
    crate::economy::fifo_tests::advance_fixture(&mut h.app, now);
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    let (ready, entered) = mpsc::channel();
    let (release, held) = mpsc::channel();
    h.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("hold economy fixture", move |_| {
            ready.send(()).unwrap();
            held.recv().unwrap();
            Ok(())
        });
    entered
        .recv_timeout(std::time::Duration::from_secs(20))
        .unwrap();
    crate::economy::fifo_tests::advance_fixture(&mut h.app, now);
    assert!(h.app.world().resource::<Economy>().market_commit_pending());
    release
}

#[test]
fn economy_pending_vendor_purchases_preserve_fifo_and_committed_cycle_state() {
    let mut h = economy_pending_fixture();
    let npc = h.open(GODRIC_ROTHGAR);
    let before = h.inventory().gold;
    let release = hold_economy_cycle(&mut h, 1000);
    h.buy(npc, 7, 2302, 2);
    h.buy(npc, 7, 2302, 1);
    assert_eq!(h.inventory().gold, before);
    assert!(h.failures().is_empty());
    release.send(()).unwrap();
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    for _ in 0..4 {
        h.app.update();
    }
    let state = h.app.world().resource::<Economy>().state.clone();
    assert_eq!(state.last_cycle, 1000);
    assert_eq!(state.customer_spent, 100);
    assert_eq!(state.merchants[&GODRIC_ROTHGAR].gold, 300);
    assert!(state.merchants[&GODRIC_ROTHGAR].stock.is_empty());
    assert_eq!(h.inventory().gold, before - 200);
    assert_eq!(h.failures(), vec![MerchantError::ItemNotFound]);
    assert_eq!(
        store::load_state(&h.app.world().resource::<GameDatabase>().db)
            .unwrap()
            .unwrap(),
        state
    );
}

#[test]
fn economy_failed_cycle_releases_vendor_purchases_and_next_cycle_commits() {
    let mut h = economy_pending_fixture();
    let npc = h.open(GODRIC_ROTHGAR);
    let before = h.inventory().gold;
    let release = hold_economy_cycle(&mut h, 1000);
    h.buy(npc, 7, 2302, 2);
    h.buy(npc, 7, 2302, 1);
    let mut changed = h.app.world().resource::<Economy>().state.clone();
    changed.withdrawn = 7;
    store::commit_market(
        &h.app.world().resource::<GameDatabase>().db,
        Some(&changed),
        &store::MarketChanges::default(),
    )
    .unwrap();
    h.app.world_mut().resource_mut::<Economy>().state = changed;
    release.send(()).unwrap();
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    for _ in 0..4 {
        h.app.update();
    }
    assert!(!h.app.world().resource::<Economy>().market_commit_pending());
    assert_eq!(h.app.world().resource::<Economy>().state.customer_spent, 0);
    assert_eq!(h.app.world().resource::<Economy>().state.withdrawn, 7);
    assert_eq!(h.inventory().gold, before - 300);
    assert!(h.failures().is_empty());
    crate::economy::fifo_tests::advance_fixture(&mut h.app, 1001);
    assert!(!h.app.world().resource::<Economy>().market_commit_pending());
    for _ in 0..3 {
        crate::economy::fifo_tests::advance_fixture(&mut h.app, 1060);
        crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    }
    assert_eq!(h.app.world().resource::<Economy>().state.last_cycle, 1060);
    assert_eq!(h.app.world().resource::<Economy>().state.withdrawn, 7);
    assert_eq!(
        store::load_state(&h.app.world().resource::<GameDatabase>().db)
            .unwrap()
            .unwrap(),
        h.app.world().resource::<Economy>().state
    );
}

#[test]
fn economy_pending_commit_does_not_defer_normal_vendor() {
    let mut h = economy_pending_fixture();
    let npc = h.open(BROTHER_DANIL);
    let item = h
        .app
        .world()
        .resource::<Inbox<VendorInventory>>()
        .0
        .last()
        .unwrap()
        .items[0]
        .clone();
    let before = h.inventory().gold;
    let release = hold_economy_cycle(&mut h, 1000);
    h.buy(npc, item.slot, item.item_id, 1);
    assert!(h.inventory().gold < before);
    assert!(h.failures().is_empty());
    assert!(h.app.world().resource::<Economy>().market_commit_pending());
    release.send(()).unwrap();
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    h.app.update();
}

#[test]
fn economic_merchant_purchase_uses_finite_stock_and_atomic_player_payment() {
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
