use super::*;

fn recipe() -> Recipe {
    Recipe {
        spell_id: 100,
        expansion: 2,
        output_item_id: 200,
        output_count: 3,
        ingredients: vec![(300, 2)],
    }
}

fn config() -> MerchantConfig {
    MerchantConfig {
        entry: 10,
        expansion: 2,
        recipes: vec![recipe()],
        capital_limit: 100,
        capacity: 5,
    }
}

fn merchant() -> MerchantState {
    MerchantState {
        gold: 100,
        stock: vec![],
    }
}

fn batch() -> StockBatch {
    StockBatch {
        id: 7,
        item_id: 200,
        count: 3,
        ingredient_cost: 10,
        unit_price: 8,
        expires_at: 86_410,
    }
}

#[test]
fn crafting_pays_real_cost_and_sets_exact_day_expiry() {
    let mut state = merchant();
    craft_stock(&mut state, &config(), &recipe(), 10, 8, 10, 7).unwrap();
    assert_eq!(state.gold, 90);
    assert_eq!(state.stock, vec![batch()]);
}

#[test]
fn capacity_is_shared_across_products_without_reservation() {
    let mut state = merchant();
    let mut other = recipe();
    other.spell_id = 101;
    other.output_item_id = 201;
    let mut cfg = config();
    cfg.recipes.push(other.clone());
    craft_stock(&mut state, &cfg, &recipe(), 10, 8, 10, 7).unwrap();
    let before = state.clone();
    assert!(craft_stock(&mut state, &cfg, &other, 10, 8, 10, 8).is_err());
    assert_eq!(state, before);
    sell_stock(&mut state, 7, 1).unwrap();
    craft_stock(&mut state, &cfg, &other, 10, 8, 10, 8).unwrap();
    assert_eq!(state.stock.iter().map(|b| b.count).sum::<u32>(), 5);
}

#[test]
fn partial_sales_leave_exact_refundable_cost_and_do_not_credit_gold() {
    let mut state = MerchantState {
        gold: 90,
        stock: vec![batch()],
    };
    assert_eq!(sell_stock(&mut state, 7, 1).unwrap(), (200, 8));
    assert_eq!(state.stock[0].ingredient_cost, 7);
    assert_eq!(state.stock[0].count, 2);
    assert_eq!(sell_stock(&mut state, 7, 1).unwrap(), (200, 8));
    assert_eq!(state.stock[0].ingredient_cost, 4);
    assert_eq!(expire_stock(&mut state, 86_410).unwrap(), 4);
    assert_eq!(state.gold, 90);
    assert!(state.stock.is_empty());
}

#[test]
fn expiry_is_inclusive_and_idempotent() {
    let mut state = MerchantState {
        gold: 0,
        stock: vec![batch()],
    };
    assert_eq!(expire_stock(&mut state, 86_409).unwrap(), 0);
    assert_eq!(state.stock.len(), 1);
    assert_eq!(expire_stock(&mut state, 86_410).unwrap(), 10);
    assert_eq!(expire_stock(&mut state, 86_410).unwrap(), 0);
}

#[test]
fn selling_all_removes_batch_and_rejects_invalid_quantities_atomically() {
    let mut state = MerchantState {
        gold: 0,
        stock: vec![batch()],
    };
    for (id, count) in [(7, 0), (7, 4), (8, 1)] {
        let before = state.clone();
        assert!(sell_stock(&mut state, id, count).is_err());
        assert_eq!(state, before);
    }
    assert_eq!(sell_stock(&mut state, 7, 3).unwrap(), (200, 24));
    assert!(state.stock.is_empty());
}

#[test]
fn sale_price_overflow_does_not_modify_stock() {
    let mut item = batch();
    item.unit_price = u64::MAX;
    let mut state = MerchantState {
        gold: 0,
        stock: vec![item],
    };
    let before = state.clone();
    assert!(sell_stock(&mut state, 7, 2).is_err());
    assert_eq!(state, before);
}

#[test]
fn expiry_refund_overflow_does_not_remove_any_batches() {
    let mut first = batch();
    first.ingredient_cost = u64::MAX;
    let mut second = batch();
    second.id = 8;
    let mut state = MerchantState {
        gold: 0,
        stock: vec![first, second],
    };
    let before = state.clone();
    assert!(expire_stock(&mut state, 86_410).is_err());
    assert_eq!(state, before);
}

#[test]
fn credit_respects_limit_and_returns_exact_excess_without_overflow() {
    let mut state = merchant();
    state.gold = 90;
    assert_eq!(credit_capital(&mut state, 25, 100), 15);
    assert_eq!(state.gold, 100);
    state.gold = u64::MAX - 2;
    assert_eq!(credit_capital(&mut state, u64::MAX, u64::MAX), u64::MAX - 2);
    assert_eq!(state.gold, u64::MAX);
    assert_eq!(credit_capital(&mut state, 5, 100), 5);
    assert_eq!(state.gold, u64::MAX);
}

#[test]
fn crafting_rejects_invalid_inputs_without_changing_state() {
    let cfg = config();
    let mut state = merchant();
    for (cost, price, now) in [(101, 8, 10), (10, 0, 10), (10, 8, u64::MAX)] {
        let before = state.clone();
        assert!(craft_stock(&mut state, &cfg, &recipe(), cost, price, now, 7).is_err());
        assert_eq!(state, before);
    }
    for invalid in [
        Recipe {
            expansion: 3,
            ..recipe()
        },
        Recipe {
            output_count: 0,
            ..recipe()
        },
        Recipe {
            output_item_id: 0,
            ..recipe()
        },
        Recipe {
            ingredients: vec![(300, 0)],
            ..recipe()
        },
        Recipe {
            ingredients: vec![],
            ..recipe()
        },
        Recipe {
            spell_id: 999,
            ..recipe()
        },
    ] {
        let before = state.clone();
        assert!(craft_stock(&mut state, &cfg, &invalid, 10, 8, 10, 7).is_err());
        assert_eq!(state, before);
    }
    craft_stock(&mut state, &cfg, &recipe(), 10, 8, 10, 7).unwrap();
    let before = state.clone();
    assert!(craft_stock(&mut state, &cfg, &recipe(), 10, 8, 10, 7).is_err());
    assert_eq!(state, before);
}

#[test]
fn large_partial_sale_prorates_without_intermediate_overflow() {
    let mut item = batch();
    item.count = u32::MAX;
    item.ingredient_cost = u64::MAX;
    item.unit_price = 1;
    let mut state = MerchantState {
        gold: 0,
        stock: vec![item],
    };
    let sold = u32::MAX - 1;
    assert_eq!(
        sell_stock(&mut state, 7, sold).unwrap(),
        (200, u64::from(sold))
    );
    let expected =
        u64::MAX - (u128::from(u64::MAX) * u128::from(sold) / u128::from(u32::MAX)) as u64;
    assert_eq!(expire_stock(&mut state, 86_410).unwrap(), expected);
}

fn market_item(item_id: u32, weight: u32) -> MarketItem {
    MarketItem {
        item_id,
        anchor_price: 8,
        min_price: 4,
        max_price: 12,
        popularity_bps: weight,
    }
}

#[test]
fn supplied_price_draws_stay_inside_inclusive_bounds() {
    let item = market_item(200, 1);
    assert_eq!(quote_price(&item, 0).unwrap(), 4);
    assert_eq!(quote_price(&item, 8).unwrap(), 12);
    assert_eq!(quote_price(&item, 9).unwrap(), 4);
    for draw in [0, 1, 8, 9, 10, u64::MAX] {
        let price = quote_price(&item, draw).unwrap();
        assert!((4..=12).contains(&price));
    }
    let fixed = MarketItem {
        min_price: 8,
        max_price: 8,
        ..item.clone()
    };
    assert_eq!(quote_price(&fixed, u64::MAX).unwrap(), 8);
    let wide = MarketItem {
        min_price: 1,
        max_price: u64::MAX,
        ..item
    };
    assert_eq!(quote_price(&wide, u64::MAX - 1).unwrap(), u64::MAX);
}

#[test]
fn invalid_price_ranges_are_errors_not_clamped() {
    let valid = market_item(200, 1);
    for invalid in [
        MarketItem {
            min_price: 13,
            ..valid.clone()
        },
        MarketItem {
            min_price: 0,
            ..valid.clone()
        },
        MarketItem {
            anchor_price: 3,
            ..valid.clone()
        },
        MarketItem {
            anchor_price: 13,
            ..valid.clone()
        },
        MarketItem {
            item_id: 0,
            ..valid
        },
    ] {
        assert!(quote_price(&invalid, 0).is_err());
    }
}

#[test]
fn consumers_obey_supplied_budget_quantity_and_draw_limits() {
    let market = BTreeMap::from([(200, market_item(200, 10))]);
    for (quantity_limit, budget, draws, expected) in [
        (3, 15, vec![0, 0, 0], (8, 1)),
        (1, 100, vec![0, 0, 0], (8, 1)),
        (3, 100, vec![0], (8, 1)),
        (3, 7, vec![0, 0], (0, 0)),
        (0, 100, vec![0], (0, 0)),
        (3, 0, vec![0], (0, 0)),
    ] {
        let mut state = MerchantState {
            gold: 90,
            stock: vec![batch()],
        };
        assert_eq!(
            consume_stock(
                &mut state,
                &market,
                &ConsumerPolicy {
                    quantity_limit,
                    budget
                },
                &draws
            )
            .unwrap(),
            expected
        );
        assert_eq!(state.gold, 90);
        assert_eq!(state.stock[0].count, 3 - expected.1);
    }
}

#[test]
fn consumer_selection_uses_popularity_and_excludes_unaffordable_goods() {
    let mut other = batch();
    other.id = 8;
    other.item_id = 201;
    let mut ignored = batch();
    ignored.id = 9;
    ignored.item_id = 202;
    let market = BTreeMap::from([
        (200, market_item(200, 1)),
        (201, market_item(201, 3)),
        (202, market_item(202, 0)),
    ]);
    let initial = MerchantState {
        gold: 0,
        stock: vec![batch(), other, ignored],
    };
    for (draw, sold_index) in [(0, 0), (1, 1), (3, 1), (4, 0)] {
        let mut state = initial.clone();
        assert_eq!(
            consume_stock(
                &mut state,
                &market,
                &ConsumerPolicy {
                    quantity_limit: 1,
                    budget: 8
                },
                &[draw]
            )
            .unwrap(),
            (8, 1)
        );
        assert_eq!(state.stock[sold_index].count, 2);
        assert_eq!(state.stock[2].count, 3);
    }
    let mut state = initial;
    state.stock[0].unit_price = 9;
    consume_stock(
        &mut state,
        &market,
        &ConsumerPolicy {
            quantity_limit: 1,
            budget: 8,
        },
        &[0],
    )
    .unwrap();
    assert_eq!(state.stock[0].count, 3);
    assert_eq!(state.stock[1].count, 2);
}

#[test]
fn missing_or_mismatched_market_data_is_atomic_error() {
    let mut state = MerchantState {
        gold: 0,
        stock: vec![batch()],
    };
    for market in [
        BTreeMap::new(),
        BTreeMap::from([(200, market_item(201, 1))]),
    ] {
        let before = state.clone();
        assert!(
            consume_stock(
                &mut state,
                &market,
                &ConsumerPolicy {
                    quantity_limit: 1,
                    budget: 8
                },
                &[0]
            )
            .is_err()
        );
        assert_eq!(state, before);
    }
}

#[test]
fn crafting_validates_configured_recipe_not_only_membership() {
    for invalid in [
        Recipe {
            expansion: 3,
            ..recipe()
        },
        Recipe {
            output_count: 0,
            ..recipe()
        },
        Recipe {
            output_item_id: 0,
            ..recipe()
        },
        Recipe {
            spell_id: 0,
            ..recipe()
        },
        Recipe {
            ingredients: vec![(0, 1)],
            ..recipe()
        },
        Recipe {
            ingredients: vec![(300, 0)],
            ..recipe()
        },
        Recipe {
            ingredients: vec![],
            ..recipe()
        },
    ] {
        let cfg = MerchantConfig {
            recipes: vec![invalid.clone()],
            ..config()
        };
        let mut state = merchant();
        let before = state.clone();
        assert!(craft_stock(&mut state, &cfg, &invalid, 10, 8, 10, 7).is_err());
        assert_eq!(state, before);
    }
}

#[test]
fn persisted_state_round_trips_all_accounting_fields() {
    let state = EconomyState {
        merchants: BTreeMap::from([(
            10,
            MerchantState {
                gold: 90,
                stock: vec![batch()],
            },
        )]),
        last_cycle: 5,
        next_batch_id: 8,
        customer_spent: 12,
        refund_issued: 4,
        withdrawn: 3,
    };
    let bytes = bitcode::encode(&state);
    assert_eq!(bitcode::decode::<EconomyState>(&bytes).unwrap(), state);
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(serde_json::from_str::<EconomyState>(&json).unwrap(), state);
}
