use super::*;

#[test]
fn bid_on_expired_auction_fails_gracefully() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Bidder", 1, 1).expect("bidder");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let bidder_gold_before = persistence::load_inventory(&db, bidder_id).gold;
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    state.auctions.remove(&auction_id);

    let bid = place_bid(&db, &mut state, bidder_id, auction_id, 100);
    assert!(!bid.success);

    let buyout = buyout_auction(&db, &mut state, bidder_id, auction_id);
    assert!(!buyout.success);

    assert_eq!(
        persistence::load_inventory(&db, bidder_id).gold,
        bidder_gold_before
    );
}

#[test]
fn deposit_refunded_on_successful_sale() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let seller_gold_before = persistence::load_inventory(&db, seller_id).gold;
    let buyer_gold_before = persistence::load_inventory(&db, buyer_id).gold;
    let mut state = AuctionHouseState::default();
    let mut app = App::new();
    app.add_plugins(crate::gold::GoldPlugin);
    let seller = app
        .world_mut()
        .spawn((
            shared::components::Player {
                name: "Seller".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            crate::persistence::ClientId(seller_id),
            shared::components::Gold(seller_gold_before),
        ))
        .id();
    let buyer = app
        .world_mut()
        .spawn((
            shared::components::Player {
                name: "Buyer".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            crate::persistence::ClientId(buyer_id),
            shared::components::Gold(buyer_gold_before),
        ))
        .id();

    let deposit = deposit_cost(source_item.vendor_sell_price, 1, AuctionDuration::Long);
    assert!(deposit > 0, "test needs non-zero deposit");

    {
        let mut gold_updates = app
            .world_mut()
            .resource_mut::<crate::gold::GoldUpdateQueue>();
        let create = super::super::handlers::create_auction_with_gold_update(
            &db,
            &mut state,
            seller_id,
            crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
            &CreateAuction {
                item_guid: source_item.item_guid,
                stack_count: 1,
                min_bid: 100,
                buyout_price: Some(500),
                duration: AuctionDuration::Long,
            },
            &mut gold_updates,
        );
        assert!(create.success);
        let after_list = persistence::load_inventory(&db, seller_id).gold;
        assert_eq!(after_list, seller_gold_before - u64::from(deposit));

        let auction_id = *state.auctions.keys().next().expect("auction id");
        let buyout = super::super::handlers::buyout_auction_with_gold_updates(
            &db,
            &mut state,
            buyer_id,
            auction_id,
            &mut gold_updates,
        );
        assert!(buyout.success);

        let mail = persistence::load_mailbox(&db, seller_id);
        let sale_proceeds = 500 - (500 * HOUSE_CUT_BPS / 10_000);
        assert_eq!(mail[0].attached_money, sale_proceeds + deposit);
        let claim = super::super::handlers::claim_mail_with_gold_update(
            &db,
            seller_id,
            mail[0].mail_id,
            &mut gold_updates,
        );
        assert!(claim.success);
    }
    app.update();

    let seller_gold = persistence::load_inventory(&db, seller_id).gold;
    let buyer_gold = persistence::load_inventory(&db, buyer_id).gold;
    let sale_proceeds = 500 - (500 * HOUSE_CUT_BPS / 10_000);
    assert_eq!(seller_gold, seller_gold_before + u64::from(sale_proceeds));
    assert_eq!(buyer_gold, buyer_gold_before - 500);
    assert_eq!(
        app.world()
            .entity(seller)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        seller_gold
    );
    assert_eq!(
        app.world()
            .entity(buyer)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        buyer_gold
    );
}

#[test]
fn auction_snapshot_independent_of_inventory_changes() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    let auction = state.auctions.values().next().expect("auction");
    let snapshotted_name = auction.item.name.clone();
    let snapshotted_quality = auction.item.quality;

    let mut inv = persistence::load_inventory(&db, seller_id);
    if let Some(item) = inv
        .items
        .iter_mut()
        .find(|i| i.item_id == source_item.item_id)
    {
        item.name = "MODIFIED".into();
        item.quality = 5;
    }
    persistence::save_inventory(&db, seller_id, &inv);

    let auction = state.auctions.values().next().expect("auction");
    assert_eq!(auction.item.name, snapshotted_name);
    assert_eq!(auction.item.quality, snapshotted_quality);

    let results = search_auctions(&state.auctions, &text_query(&snapshotted_name));
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results[0].item.name, snapshotted_name);
}

#[test]
fn empty_auction_house_search_returns_gracefully() {
    let state = AuctionHouseState::default();

    let r = search_auctions(&state.auctions, &text_query(""));
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());

    let mut query = text_query("sword");
    query.quality = Some(3);
    query.min_level = Some(10);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());

    let mut query = text_query("");
    query.page = 5;
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());
}

#[test]
fn seller_offline_auction_still_completes() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    let response = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(response.success);

    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    assert!(seller_mail[0].attached_money > 0);

    let seller_gold_before_claim = persistence::load_inventory(&db, seller_id).gold;
    claim_mail(&db, seller_id, seller_mail[0].mail_id);
    let seller_gold_after_claim = persistence::load_inventory(&db, seller_id).gold;
    assert!(seller_gold_after_claim > seller_gold_before_claim);
}

#[test]
fn gold_overflow_saturates_at_max() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");

    let mut inv = persistence::load_inventory(&db, seller_id);
    inv.gold = crate::gold::MAX_MONEY - 10;
    persistence::save_inventory(&db, seller_id, &inv);

    deliver_mail(&db, seller_id, "Test", "gold overflow", 100, None);
    let mail = persistence::load_mailbox(&db, seller_id);
    claim_mail(&db, seller_id, mail[0].mail_id);

    let final_gold = persistence::load_inventory(&db, seller_id).gold;
    assert_eq!(
        final_gold,
        crate::gold::MAX_MONEY,
        "gold stops at the Retail cap"
    );
}

#[test]
fn unique_item_prevents_duplicate_listing() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let mut state = AuctionHouseState::default();

    let mut inv = persistence::load_inventory(&db, seller_id);
    inv.items[0].unique_auction = true;
    inv.items[0].stack_count = 10;
    persistence::save_inventory(&db, seller_id, &inv);

    let item = persistence::load_inventory(&db, seller_id).items[0].clone();

    let first = create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: item.item_guid,
            stack_count: 3,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    assert!(first.success);

    let remaining = persistence::load_inventory(&db, seller_id).items[0].clone();
    let second = create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: remaining.item_guid,
            stack_count: 2,
            min_bid: 50,
            buyout_price: Some(200),
            duration: AuctionDuration::Short,
        },
    );
    assert!(!second.success);
    assert_eq!(state.auctions.len(), 1);
}

#[test]
fn expired_auction_with_bid_awards_winner_and_pays_seller() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Bidder", 1, 1).expect("bidder");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 2,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");
    place_bid(&db, &mut state, bidder_id, auction_id, 200);

    state
        .auctions
        .get_mut(&auction_id)
        .expect("auction")
        .expires_at_unix_secs = 0;

    let auction = state.auctions.remove(&auction_id).expect("auction");
    let winner_id = auction.current_bidder_character_id.expect("has bidder");
    let winning_bid = auction.current_bid.unwrap_or(auction.min_bid);
    persistence::delete_auction(&db, auction_id);
    complete_sale(&db, &auction, winner_id, winning_bid);

    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    let dep = deposit_cost(source_item.vendor_sell_price, 2, AuctionDuration::Short);
    let expected_proceeds = 200 - (200 * HOUSE_CUT_BPS / 10_000) + dep;
    assert_eq!(seller_mail[0].attached_money, expected_proceeds);
    assert!(seller_mail[0].attached_item.is_none());

    let bidder_mail = persistence::load_mailbox(&db, bidder_id);
    assert_eq!(bidder_mail.len(), 1);
    assert_eq!(bidder_mail[0].attached_money, 0);
    let won_item = bidder_mail[0].attached_item.as_ref().expect("won item");
    assert_eq!(won_item.item_id, source_item.item_id);
    assert_eq!(won_item.stack_count, 2);
}

#[test]
fn house_cut_deducted_on_successful_sale() {
    let deposit = 13_u32;
    let cases: &[(u32, u32)] = &[
        (200, 190 + deposit),
        (1000, 950 + deposit),
        (1, 1 + deposit),
        (19, 19 + deposit),
        (20, 19 + deposit),
        (33, 32 + deposit),
    ];

    for &(buyout_price, expected_proceeds) in cases {
        let db = persistence::temp_database();
        let (seller_id, _) =
            persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
        let (buyer_id, _) =
            persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
        let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
        let mut state = AuctionHouseState::default();

        create_auction(
            &db,
            &mut state,
            seller_id,
            crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
            &CreateAuction {
                item_guid: source_item.item_guid,
                stack_count: 1,
                min_bid: 1,
                buyout_price: Some(buyout_price),
                duration: AuctionDuration::Short,
            },
        );
        let auction_id = *state.auctions.keys().next().expect("auction id");
        buyout_auction(&db, &mut state, buyer_id, auction_id);

        let seller_mail = persistence::load_mailbox(&db, seller_id);
        assert_eq!(
            seller_mail[0].attached_money, expected_proceeds,
            "buyout={buyout_price}: expected {expected_proceeds}, got {}",
            seller_mail[0].attached_money,
        );
    }
}

#[test]
fn buyout_transfers_item_and_deducts_gold() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let buyer_gold_before = persistence::load_inventory(&db, buyer_id).gold;
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 2,
            min_bid: 50,
            buyout_price: Some(300),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    let response = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(response.success);

    assert!(!state.auctions.contains_key(&auction_id));

    let buyer_gold_after = persistence::load_inventory(&db, buyer_id).gold;
    assert_eq!(buyer_gold_after, buyer_gold_before - 300);

    let buyer_mail = persistence::load_mailbox(&db, buyer_id);
    assert_eq!(buyer_mail.len(), 1);
    let buyer_item = buyer_mail[0].attached_item.as_ref().expect("item in mail");
    assert_eq!(buyer_item.item_id, source_item.item_id);
    assert_eq!(buyer_item.stack_count, 2);

    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    let deposit = deposit_cost(source_item.vendor_sell_price, 2, AuctionDuration::Medium);
    let expected_gold = 300 - (300 * HOUSE_CUT_BPS / 10_000) + deposit;
    assert_eq!(seller_mail[0].attached_money, expected_gold);
}

#[test]
fn buyout_and_claim_mail_delivers_item_and_gold() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let seller_gold_before = persistence::load_inventory(&db, seller_id).gold;
    let buyer_gold_before = persistence::load_inventory(&db, buyer_id).gold;
    let buyer_item_count_before = persistence::load_inventory(&db, buyer_id).items.len();
    let mut state = AuctionHouseState::default();
    let deposit = deposit_cost(source_item.vendor_sell_price, 3, AuctionDuration::Medium);

    create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 3,
            min_bid: 100,
            buyout_price: Some(200),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");
    let response = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(response.success);
    assert!(!state.auctions.contains_key(&auction_id));

    let seller_mail = persistence::load_mailbox(&db, seller_id);
    let buyer_mail = persistence::load_mailbox(&db, buyer_id);
    assert_eq!(seller_mail.len(), 1);
    assert_eq!(buyer_mail.len(), 1);
    assert_eq!(seller_mail[0].attached_money, 190 + deposit);
    assert_eq!(
        buyer_mail[0]
            .attached_item
            .as_ref()
            .expect("buyer item mail")
            .stack_count,
        3
    );

    let seller_claim = claim_mail(&db, seller_id, seller_mail[0].mail_id);
    let buyer_claim = claim_mail(&db, buyer_id, buyer_mail[0].mail_id);
    assert!(seller_claim.success);
    assert!(buyer_claim.success);
    assert_eq!(
        persistence::load_inventory(&db, seller_id).gold,
        seller_gold_before + 190
    );
    assert_eq!(
        persistence::load_inventory(&db, buyer_id).gold,
        buyer_gold_before - 200
    );
    assert_eq!(
        persistence::load_inventory(&db, buyer_id).items.len(),
        buyer_item_count_before + 1
    );
}
