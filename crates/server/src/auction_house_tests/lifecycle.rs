use super::*;

#[test]
fn cancel_own_auction_returns_item_via_mail() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (other_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Other", 1, 1).expect("other");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 3,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    let denied = cancel_auction(&db, &mut state, other_id, auction_id);
    assert!(!denied.success);
    assert!(state.auctions.contains_key(&auction_id));

    let response = cancel_auction(&db, &mut state, seller_id, auction_id);
    assert!(response.success);
    assert!(!state.auctions.contains_key(&auction_id));

    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    assert_eq!(seller_mail[0].attached_money, 0);
    let returned_item = seller_mail[0]
        .attached_item
        .as_ref()
        .expect("returned item");
    assert_eq!(returned_item.item_id, source_item.item_id);
    assert_eq!(returned_item.stack_count, 3);
}

#[test]
fn cancel_auction_with_bids_rejected() {
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
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");
    place_bid(&db, &mut state, bidder_id, auction_id, 100);

    let response = cancel_auction(&db, &mut state, seller_id, auction_id);
    assert!(!response.success);
    assert!(state.auctions.contains_key(&auction_id));
}

#[test]
fn bid_on_own_auction_rejected() {
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
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");
    let seller_gold_after_create = persistence::load_inventory(&db, seller_id).gold;

    let bid_response = place_bid(&db, &mut state, seller_id, auction_id, 100);
    assert!(!bid_response.success);
    assert!(
        state
            .auctions
            .get(&auction_id)
            .unwrap()
            .current_bid
            .is_none()
    );

    let buyout_response = buyout_auction(&db, &mut state, seller_id, auction_id);
    assert!(!buyout_response.success);
    assert!(state.auctions.contains_key(&auction_id));

    let seller_gold_after = persistence::load_inventory(&db, seller_id).gold;
    assert_eq!(seller_gold_after, seller_gold_after_create);
}

#[test]
fn bid_only_auction_rejects_buyout() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    let response = create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: None,
            duration: AuctionDuration::Medium,
        },
    );
    assert!(response.success);
    let auction_id = *state.auctions.keys().next().expect("auction id");

    let results = search_auctions(&state.auctions, &text_query(""));
    assert_eq!(results.results[0].buyout_price, None);

    let buyout = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(!buyout.success);
    assert!(state.auctions.contains_key(&auction_id));

    let bid = place_bid(&db, &mut state, buyer_id, auction_id, 100);
    assert!(bid.success);
    assert_eq!(
        state.auctions.get(&auction_id).unwrap().current_bid,
        Some(100)
    );
}

#[test]
fn create_auction_rejected_with_insufficient_deposit() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    let deposit = deposit_cost(source_item.vendor_sell_price, 1, AuctionDuration::Long);
    let mut inv = persistence::load_inventory(&db, seller_id);
    inv.gold = u64::from(deposit) - 1;
    persistence::save_inventory(&db, seller_id, &inv);

    let response = create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 10,
            buyout_price: Some(100),
            duration: AuctionDuration::Long,
        },
    );
    assert!(!response.success);
    assert!(state.auctions.is_empty());
    assert_eq!(
        persistence::load_inventory(&db, seller_id).gold,
        u64::from(deposit) - 1
    );
    assert_eq!(
        persistence::load_inventory(&db, seller_id).items.len(),
        inv.items.len()
    );
}

#[test]
fn max_auction_slots_per_character_enforced() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let mut state = AuctionHouseState::default();

    for i in 1..=MAX_AUCTIONS_PER_CHARACTER {
        let mut auction = test_auction(i as u64, "Item", 1, 1);
        auction.seller_character_id = seller_id;
        state.auctions.insert(i as u64, auction);
    }

    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let response = create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 10,
            buyout_price: Some(100),
            duration: AuctionDuration::Short,
        },
    );
    assert!(!response.success);
    assert_eq!(state.auctions.len(), MAX_AUCTIONS_PER_CHARACTER);
}

#[test]
fn duration_tiers_set_correct_expiry() {
    assert_eq!(duration_secs(AuctionDuration::Short), 30 * 60);
    assert_eq!(duration_secs(AuctionDuration::Medium), 2 * 60 * 60);
    assert_eq!(duration_secs(AuctionDuration::Long), 8 * 60 * 60);

    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let now = unix_now();

    for (duration, expected_secs) in [
        (AuctionDuration::Short, 30 * 60),
        (AuctionDuration::Medium, 2 * 60 * 60),
        (AuctionDuration::Long, 8 * 60 * 60),
    ] {
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
                min_bid: 10,
                buyout_price: Some(100),
                duration,
            },
        );
        let auction = state.auctions.values().next().expect("auction");
        let diff = auction.expires_at_unix_secs.abs_diff(now + expected_secs);
        assert!(
            diff <= 2,
            "{duration:?}: expiry off by {diff}s (expected ~{expected_secs}s from now)",
        );
    }
}

#[test]
fn concurrent_bids_respect_updated_minimum() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_a, _) =
        persistence::create_character_with_test_bags(&db, 2, "BidderA", 1, 1).expect("a");
    let (bidder_b, _) =
        persistence::create_character_with_test_bags(&db, 3, "BidderB", 1, 1).expect("b");
    let (bidder_c, _) =
        persistence::create_character_with_test_bags(&db, 4, "BidderC", 1, 1).expect("c");
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
            buyout_price: Some(1000),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    let a = place_bid(&db, &mut state, bidder_a, auction_id, 100);
    assert!(a.success);

    let b_stale = place_bid(&db, &mut state, bidder_b, auction_id, 100);
    assert!(!b_stale.success, "stale bid at old minimum rejected");

    let c = place_bid(&db, &mut state, bidder_c, auction_id, 105);
    assert!(c.success);

    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(105));
    assert_eq!(auction.current_bidder_character_id, Some(bidder_c));

    let a_gold = persistence::load_inventory(&db, bidder_a).gold;
    let b_gold = persistence::load_inventory(&db, bidder_b).gold;
    let initial_gold = 100_000;
    assert_eq!(a_gold, initial_gold, "A refunded");
    assert_eq!(b_gold, initial_gold, "B never charged");
}

#[test]
fn bid_at_buyout_price_triggers_immediate_sale() {
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

    let response = place_bid(&db, &mut state, buyer_id, auction_id, 500);
    assert!(response.success);

    assert!(!state.auctions.contains_key(&auction_id));

    let buyer_mail = persistence::load_mailbox(&db, buyer_id);
    assert_eq!(buyer_mail.len(), 1);
    assert!(buyer_mail[0].attached_item.is_some());

    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    let dep = deposit_cost(source_item.vendor_sell_price, 1, AuctionDuration::Medium);
    let expected_proceeds = 500 - (500 * HOUSE_CUT_BPS / 10_000) + dep;
    assert_eq!(seller_mail[0].attached_money, expected_proceeds);
}

#[test]
fn insufficient_gold_for_bid_and_buyout() {
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
            min_bid: 500,
            buyout_price: Some(1000),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    let mut buyer_inv = persistence::load_inventory(&db, buyer_id);
    buyer_inv.gold = 499;
    persistence::save_inventory(&db, buyer_id, &buyer_inv);

    let bid_response = place_bid(&db, &mut state, buyer_id, auction_id, 500);
    assert!(!bid_response.success);
    assert!(
        state
            .auctions
            .get(&auction_id)
            .unwrap()
            .current_bid
            .is_none()
    );

    let buyout_response = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(!buyout_response.success);
    assert!(state.auctions.contains_key(&auction_id));

    assert_eq!(persistence::load_inventory(&db, buyer_id).gold, 499);
}

#[test]
fn outbid_sends_notification_mail_to_previous_bidder() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_a_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "BidderA", 1, 1).expect("bidder a");
    let (bidder_b_id, _) =
        persistence::create_character_with_test_bags(&db, 3, "BidderB", 1, 1).expect("bidder b");
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
            buyout_price: Some(1000),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    place_bid(&db, &mut state, bidder_a_id, auction_id, 100);
    assert!(persistence::load_mailbox(&db, bidder_a_id).is_empty());

    let response = place_bid(&db, &mut state, bidder_b_id, auction_id, 105);
    assert!(response.success);

    let mail_a = persistence::load_mailbox(&db, bidder_a_id);
    assert_eq!(mail_a.len(), 1);
    assert_eq!(mail_a[0].subject, "Outbid");
    assert!(mail_a[0].body.contains(&source_item.name));
    assert_eq!(mail_a[0].attached_money, 0);
    assert!(mail_a[0].attached_item.is_none());

    assert!(persistence::load_mailbox(&db, bidder_b_id).is_empty());
}

#[test]
fn scheduled_auction_expiration_runs_at_most_once_per_second() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(GameDatabase { db });
    app.add_plugins(AuctionHousePlugin);

    let mut first = test_auction(1, "First item", 1, 1);
    first.seller_character_id = seller_id;
    first.expires_at_unix_secs = unix_now();
    app.world_mut()
        .resource_mut::<AuctionHouseState>()
        .auctions
        .insert(first.auction_id, first);
    app.world_mut().run_schedule(FixedUpdate);

    assert!(
        !app.world()
            .resource::<AuctionHouseState>()
            .auctions
            .contains_key(&1)
    );
    assert_eq!(
        persistence::load_mailbox(&app.world().resource::<GameDatabase>().db, seller_id,).len(),
        1
    );

    let mut second = test_auction(2, "Second item", 1, 1);
    second.seller_character_id = seller_id;
    second.expires_at_unix_secs = unix_now();
    app.world_mut()
        .resource_mut::<AuctionHouseState>()
        .auctions
        .insert(second.auction_id, second);

    for _ in 0..5 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert!(
        app.world()
            .resource::<AuctionHouseState>()
            .auctions
            .contains_key(&2),
        "repeated sub-second fixed ticks must not expire a newly due auction"
    );

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(750));
    app.world_mut().run_schedule(FixedUpdate);
    assert!(
        !app.world()
            .resource::<AuctionHouseState>()
            .auctions
            .contains_key(&2)
    );
    assert_eq!(
        persistence::load_mailbox(&app.world().resource::<GameDatabase>().db, seller_id,).len(),
        2
    );
}

#[test]
fn expired_auction_without_bid_returns_item_to_seller() {
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
            stack_count: 4,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    state
        .auctions
        .get_mut(&auction_id)
        .expect("auction")
        .expires_at_unix_secs = 0;

    let auction = state.auctions.remove(&auction_id).expect("auction");
    assert!(auction.current_bidder_character_id.is_none());
    persistence::delete_auction(&db, auction_id);
    deliver_mail(
        &db,
        auction.seller_character_id,
        "Auction expired",
        &format!(
            "Your auction for {} expired without a buyer.",
            auction.item.name
        ),
        0,
        Some(auction.item.clone()),
    );

    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    assert_eq!(seller_mail[0].attached_money, 0);
    let returned_item = seller_mail[0]
        .attached_item
        .as_ref()
        .expect("returned item");
    assert_eq!(returned_item.item_id, source_item.item_id);
    assert_eq!(returned_item.stack_count, 4);
}

#[test]
fn relist_after_expired_auction() {
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
            stack_count: 2,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");
    state
        .auctions
        .get_mut(&auction_id)
        .unwrap()
        .expires_at_unix_secs = 0;
    let auction = state.auctions.remove(&auction_id).unwrap();
    persistence::delete_auction(&db, auction_id);
    deliver_mail(
        &db,
        seller_id,
        "Auction expired",
        "expired",
        0,
        Some(auction.item),
    );

    let mail = persistence::load_mailbox(&db, seller_id);
    let claim = claim_mail(&db, seller_id, mail[0].mail_id);
    assert!(claim.success);

    let inv = persistence::load_inventory(&db, seller_id);
    let relisted_item = inv
        .items
        .iter()
        .find(|i| i.item_id == source_item.item_id && i.stack_count >= 2)
        .expect("returned item in inventory");
    let relist = create_auction(
        &db,
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: relisted_item.item_guid,
            stack_count: 2,
            min_bid: 200,
            buyout_price: Some(1000),
            duration: AuctionDuration::Medium,
        },
    );
    assert!(relist.success);
    assert_eq!(state.auctions.len(), 1);
}
