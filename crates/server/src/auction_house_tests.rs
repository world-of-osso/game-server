use super::*;
use crate::persistence;

#[test]
fn min_next_bid_rounds_up() {
    let auction = AuctionData {
        auction_id: 1,
        seller_character_id: 1,
        seller_name: "seller".into(),
        item: InventoryItemData {
            item_guid: 1,
            item_id: 1,
            name: "Linen Cloth".into(),
            quality: 1,
            required_level: 1,
            stack_count: 5,
            vendor_sell_price: 1,
            unique_auction: false,
        },
        stack_count: 5,
        min_bid: 100,
        current_bid: Some(100),
        current_bidder_character_id: Some(2),
        buyout_price: Some(150),
        expires_at_unix_secs: unix_now() + 60,
        faction: 0,
        deposit: 0,
    };
    assert_eq!(min_next_bid(&auction), 105);
}

#[test]
fn create_auction_moves_items_from_inventory_into_state() {
    let db = persistence::temp_database();
    let (character_id, _) =
        persistence::create_character(&db, 1, "Seller", 1, 1).expect("character");
    let inventory_before = persistence::load_inventory(&db, character_id);
    let source_item = inventory_before.items[0].clone();
    let original_count = source_item.stack_count;
    let mut state = AuctionHouseState::default();

    let response = create_auction(
        &db,
        &mut state,
        character_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 5,
            min_bid: 100,
            buyout_price: Some(150),
            duration: AuctionDuration::Medium,
        },
    );

    assert!(response.success);
    assert_eq!(state.auctions.len(), 1);
    let inventory_after = persistence::load_inventory(&db, character_id);
    let remaining = inventory_after
        .items
        .iter()
        .find(|item| item.item_guid == source_item.item_guid)
        .expect("remaining source item");
    assert_eq!(remaining.stack_count, original_count - 5);
    let auction = state.auctions.values().next().expect("auction");
    assert_eq!(auction.item.stack_count, 5);
    assert_eq!(auction.min_bid, 100);
}

#[test]
fn place_bid_refunds_previous_bidder_and_updates_current_bid() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_a_id, _) =
        persistence::create_character(&db, 2, "BidderA", 1, 1).expect("bidder a");
    let (bidder_b_id, _) =
        persistence::create_character(&db, 3, "BidderB", 1, 1).expect("bidder b");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();
    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 2,
            min_bid: 100,
            buyout_price: Some(200),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");
    let bidder_a_gold_before = persistence::load_inventory(&db, bidder_a_id).gold;
    let bidder_b_gold_before = persistence::load_inventory(&db, bidder_b_id).gold;

    let first = place_bid(&db, &mut state, bidder_a_id, auction_id, 100);
    let second = place_bid(&db, &mut state, bidder_b_id, auction_id, 105);

    assert!(first.success);
    assert!(second.success);
    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(105));
    assert_eq!(auction.current_bidder_character_id, Some(bidder_b_id));
    assert_eq!(
        persistence::load_inventory(&db, bidder_a_id).gold,
        bidder_a_gold_before
    );
    assert_eq!(
        persistence::load_inventory(&db, bidder_b_id).gold,
        bidder_b_gold_before - 105
    );
}

fn test_auction(id: u64, name: &str, quality: u8, required_level: u16) -> AuctionData {
    AuctionData {
        auction_id: id,
        seller_character_id: 1,
        seller_name: "Seller".into(),
        item: InventoryItemData {
            item_guid: id,
            item_id: id as u32,
            name: name.into(),
            quality,
            required_level,
            stack_count: 1,
            vendor_sell_price: 1,
            unique_auction: false,
        },
        stack_count: 1,
        min_bid: 100,
        current_bid: None,
        current_bidder_character_id: None,
        buyout_price: Some(200),
        expires_at_unix_secs: unix_now() + 3600,
        faction: 0,
        deposit: 0,
    }
}

fn text_query(text: &str) -> AuctionSearchQuery {
    AuctionSearchQuery {
        text: text.into(),
        page: 0,
        page_size: 50,
        min_level: None,
        max_level: None,
        quality: None,
        usable_only: false,
        sort_field: AuctionSortField::Name,
        sort_dir: AuctionSortDir::Asc,
        faction: 0,
    }
}

#[test]
fn create_auction_appears_in_search_results() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("character");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    let response = create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 3,
            min_bid: 100,
            buyout_price: Some(200),
            duration: AuctionDuration::Medium,
        },
    );
    assert!(response.success);

    let results = search_auctions(&state.auctions, &text_query(""));
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results.len(), 1);
    let listing = &results.results[0];
    assert_eq!(listing.item.name, source_item.name);
    assert_eq!(listing.stack_count, 3);
    assert_eq!(listing.min_bid, 100);
    assert_eq!(listing.buyout_price, Some(200));
    assert_eq!(listing.owner_name, "Seller");

    let name_prefix = &source_item.name[..3].to_ascii_lowercase();
    let name_results = search_auctions(&state.auctions, &text_query(name_prefix));
    assert_eq!(name_results.total_results, 1);

    let empty_results = search_auctions(&state.auctions, &text_query("zzz_nonexistent_item"));
    assert_eq!(empty_results.total_results, 0);
    assert!(empty_results.results.is_empty());
}

#[test]
fn bid_recorded_and_underbid_rejected() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_id, _) = persistence::create_character(&db, 2, "Bidder", 1, 1).expect("bidder");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();
    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 2,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    // First bid at min_bid succeeds
    let first = place_bid(&db, &mut state, bidder_id, auction_id, 100);
    assert!(first.success);
    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(100));
    assert_eq!(auction.current_bidder_character_id, Some(bidder_id));

    // Bid below min_next_bid (105 = 100 + 5%) is rejected
    let low_bid = place_bid(&db, &mut state, bidder_id, auction_id, 104);
    assert!(!low_bid.success);
    // Current bid unchanged
    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(100));

    // Bid at exactly min_next_bid succeeds
    let raise = place_bid(&db, &mut state, bidder_id, auction_id, 105);
    assert!(raise.success);
    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(105));
}

#[test]
fn search_filters_by_name_quality_and_level_range() {
    let mut state = AuctionHouseState::default();
    state
        .auctions
        .insert(1, test_auction(1, "Iron Sword", 2, 10));
    state
        .auctions
        .insert(2, test_auction(2, "Steel Sword", 3, 30));
    state
        .auctions
        .insert(3, test_auction(3, "Linen Cloth", 1, 5));
    state.auctions.insert(4, test_auction(4, "Iron Ore", 1, 1));

    // Filter by name: "sword" matches 2
    let results = search_auctions(&state.auctions, &text_query("sword"));
    assert_eq!(results.total_results, 2);

    // Filter by name: "iron" matches Iron Sword + Iron Ore
    let results = search_auctions(&state.auctions, &text_query("iron"));
    assert_eq!(results.total_results, 2);

    // Filter by quality: rare (3) matches only Steel Sword
    let mut query = text_query("");
    query.quality = Some(3);
    let results = search_auctions(&state.auctions, &query);
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results[0].item.name, "Steel Sword");

    // Filter by min_level: >= 10 matches Iron Sword (10) + Steel Sword (30)
    let mut query = text_query("");
    query.min_level = Some(10);
    let results = search_auctions(&state.auctions, &query);
    assert_eq!(results.total_results, 2);

    // Filter by max_level: <= 5 matches Linen Cloth (5) + Iron Ore (1)
    let mut query = text_query("");
    query.max_level = Some(5);
    let results = search_auctions(&state.auctions, &query);
    assert_eq!(results.total_results, 2);

    // Combined: name "iron" + level range 1..=10
    let mut query = text_query("iron");
    query.min_level = Some(1);
    query.max_level = Some(10);
    let results = search_auctions(&state.auctions, &query);
    assert_eq!(results.total_results, 2);

    // Combined: name "sword" + quality >= 3
    let mut query = text_query("sword");
    query.quality = Some(3);
    let results = search_auctions(&state.auctions, &query);
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results[0].item.name, "Steel Sword");
}

#[test]
fn search_returns_empty_for_non_matching_filters() {
    let mut state = AuctionHouseState::default();
    state
        .auctions
        .insert(1, test_auction(1, "Iron Sword", 2, 20));

    // Non-matching quality
    let mut query = text_query("");
    query.quality = Some(4);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);

    // min_level above item level
    let mut query = text_query("");
    query.min_level = Some(30);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);

    // max_level below item level
    let mut query = text_query("");
    query.max_level = Some(10);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);

    // Non-matching text
    let r = search_auctions(&state.auctions, &text_query("zzz"));
    assert_eq!(r.total_results, 0);

    // Combined: matching text but wrong quality
    let mut query = text_query("iron");
    query.quality = Some(4);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);
}

#[test]
fn search_with_all_filters_combined() {
    let mut state = AuctionHouseState::default();
    state
        .auctions
        .insert(1, test_auction(1, "Iron Sword", 2, 20));
    state
        .auctions
        .insert(2, test_auction(2, "Iron Shield", 3, 20));
    state.auctions.insert(3, test_auction(3, "Iron Ore", 2, 5));
    state
        .auctions
        .insert(4, test_auction(4, "Steel Sword", 2, 30));

    // All three filters: name="iron" + quality=2 + level 10..=25
    let mut query = text_query("iron");
    query.quality = Some(2);
    query.min_level = Some(10);
    query.max_level = Some(25);
    let r = search_auctions(&state.auctions, &query);
    // Only Iron Sword matches (name=iron, quality=2, level=20)
    // Iron Shield: quality=3 (miss), Iron Ore: level=5 (miss), Steel Sword: name miss
    assert_eq!(r.total_results, 1);
    assert_eq!(r.results[0].item.name, "Iron Sword");

    // Relax level to include Iron Ore (level 5)
    let mut query = text_query("iron");
    query.quality = Some(2);
    query.min_level = Some(1);
    query.max_level = Some(25);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 2);

    // All filters match nothing
    let mut query = text_query("iron");
    query.quality = Some(4);
    query.min_level = Some(10);
    query.max_level = Some(25);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);
}

#[test]
fn faction_isolation_filters_auctions() {
    let mut state = AuctionHouseState::default();
    let mut alliance = test_auction(1, "Alliance Sword", 2, 10);
    alliance.faction = 1;
    let mut horde = test_auction(2, "Horde Axe", 2, 10);
    horde.faction = 2;
    let mut neutral = test_auction(3, "Neutral Cloth", 1, 5);
    neutral.faction = 0;
    state.auctions.insert(1, alliance);
    state.auctions.insert(2, horde);
    state.auctions.insert(3, neutral);

    // Alliance sees alliance + neutral (2 results)
    let mut query = text_query("");
    query.faction = 1;
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 2);
    let names: Vec<&str> = r.results.iter().map(|l| l.item.name.as_str()).collect();
    assert!(names.contains(&"Alliance Sword"));
    assert!(names.contains(&"Neutral Cloth"));

    // Horde sees horde + neutral (2 results)
    let mut query = text_query("");
    query.faction = 2;
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 2);
    let names: Vec<&str> = r.results.iter().map(|l| l.item.name.as_str()).collect();
    assert!(names.contains(&"Horde Axe"));
    assert!(names.contains(&"Neutral Cloth"));

    // Faction 0 (neutral/admin) sees all
    let r = search_auctions(&state.auctions, &text_query(""));
    assert_eq!(r.total_results, 3);
}

#[test]
fn sort_by_price_time_remaining_and_name() {
    let mut state = AuctionHouseState::default();

    let mut a = test_auction(1, "Copper Ore", 1, 1);
    a.min_bid = 50;
    a.buyout_price = Some(300);
    a.expires_at_unix_secs = unix_now() + 10; // Short (≤60s)

    let mut b = test_auction(2, "Iron Ore", 2, 10);
    b.min_bid = 200;
    b.buyout_price = Some(100);
    b.expires_at_unix_secs = unix_now() + 3600 * 5; // Long (≤6hr)

    let mut c = test_auction(3, "Arcane Dust", 1, 5);
    c.min_bid = 100;
    c.buyout_price = Some(200);
    c.expires_at_unix_secs = unix_now() + 300; // Medium (≤30min)

    state.auctions.insert(1, a);
    state.auctions.insert(2, b);
    state.auctions.insert(3, c);

    let sort = |field, dir| {
        search_auctions(
            &state.auctions,
            &AuctionSearchQuery {
                text: String::new(),
                page: 0,
                page_size: 50,
                min_level: None,
                max_level: None,
                quality: None,
                usable_only: false,
                sort_field: field,
                sort_dir: dir,
                faction: 0,
            },
        )
    };
    let names = |r: AuctionSearchResults| -> Vec<String> {
        r.results.into_iter().map(|l| l.item.name).collect()
    };

    // Sort by name ascending
    let r = sort(AuctionSortField::Name, AuctionSortDir::Asc);
    assert_eq!(names(r), ["Arcane Dust", "Copper Ore", "Iron Ore"]);

    // Sort by name descending
    let r = sort(AuctionSortField::Name, AuctionSortDir::Desc);
    assert_eq!(names(r), ["Iron Ore", "Copper Ore", "Arcane Dust"]);

    // Sort by min_bid ascending: 50, 100, 200
    let r = sort(AuctionSortField::MinBid, AuctionSortDir::Asc);
    assert_eq!(names(r), ["Copper Ore", "Arcane Dust", "Iron Ore"]);

    // Sort by buyout ascending: 100, 200, 300
    let r = sort(AuctionSortField::Buyout, AuctionSortDir::Asc);
    assert_eq!(names(r), ["Iron Ore", "Arcane Dust", "Copper Ore"]);

    // Sort by time remaining ascending: Short, Medium, Long
    let r = sort(AuctionSortField::TimeLeft, AuctionSortDir::Asc);
    assert_eq!(names(r), ["Copper Ore", "Arcane Dust", "Iron Ore"]);
}

#[test]
fn pagination_respects_page_boundaries() {
    let mut state = AuctionHouseState::default();
    let total = 75;
    for i in 1..=total {
        state
            .auctions
            .insert(i, test_auction(i, &format!("Item {i:03}"), 1, 1));
    }

    let page = |page_num: u32, page_size: u32| {
        search_auctions(
            &state.auctions,
            &AuctionSearchQuery {
                text: String::new(),
                page: page_num,
                page_size,
                min_level: None,
                max_level: None,
                quality: None,
                usable_only: false,
                sort_field: AuctionSortField::Name,
                sort_dir: AuctionSortDir::Asc,
                faction: 0,
            },
        )
    };

    // Default page size clamped to MAX_PAGE_SIZE (50)
    let r = page(0, 50);
    assert_eq!(r.total_results, 75);
    assert_eq!(r.results.len(), 50);

    // Second page gets remaining 25
    let r = page(1, 50);
    assert_eq!(r.total_results, 75);
    assert_eq!(r.results.len(), 25);

    // Third page is empty
    let r = page(2, 50);
    assert_eq!(r.total_results, 75);
    assert_eq!(r.results.len(), 0);

    // Page size > MAX_PAGE_SIZE clamped to 50
    let r = page(0, 100);
    assert_eq!(r.results.len(), 50);

    // Smaller page size works
    let r = page(0, 10);
    assert_eq!(r.results.len(), 10);
    let r = page(7, 10);
    assert_eq!(r.results.len(), 5); // 75 - 70 = 5

    // Page size 0 clamped to 1
    let r = page(0, 0);
    assert_eq!(r.results.len(), 1);
}

#[test]
fn cancel_own_auction_returns_item_via_mail() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (other_id, _) = persistence::create_character(&db, 2, "Other", 1, 1).expect("other");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 3,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    // Non-owner cannot cancel
    let denied = cancel_auction(&db, &mut state, other_id, auction_id);
    assert!(!denied.success);
    assert!(state.auctions.contains_key(&auction_id));

    // Owner cancels successfully
    let response = cancel_auction(&db, &mut state, seller_id, auction_id);
    assert!(response.success);
    assert!(!state.auctions.contains_key(&auction_id));

    // Item returned via mail
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
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_id, _) = persistence::create_character(&db, 2, "Bidder", 1, 1).expect("bidder");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
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
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");
    // Capture gold after deposit deducted
    let seller_gold_after_create = persistence::load_inventory(&db, seller_id).gold;

    // Bid on own auction rejected
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

    // Buyout own auction also rejected
    let buyout_response = buyout_auction(&db, &mut state, seller_id, auction_id);
    assert!(!buyout_response.success);
    assert!(state.auctions.contains_key(&auction_id));

    // Gold unchanged after failed bid/buyout attempts
    let seller_gold_after = persistence::load_inventory(&db, seller_id).gold;
    assert_eq!(seller_gold_after, seller_gold_after_create);
}

#[test]
fn bid_only_auction_rejects_buyout() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    let response = create_auction(
        &db,
        &mut state,
        seller_id,
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

    // Search shows no buyout price
    let results = search_auctions(&state.auctions, &text_query(""));
    assert_eq!(results.results[0].buyout_price, None);

    // Buyout is rejected
    let buyout = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(!buyout.success);
    assert!(state.auctions.contains_key(&auction_id));

    // Bidding still works
    let bid = place_bid(&db, &mut state, buyer_id, auction_id, 100);
    assert!(bid.success);
    assert_eq!(
        state.auctions.get(&auction_id).unwrap().current_bid,
        Some(100),
    );
}

#[test]
fn create_auction_rejected_with_insufficient_deposit() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    // Drain seller gold to just below the deposit cost
    let deposit = deposit_cost(source_item.vendor_sell_price, 1, AuctionDuration::Long);
    let mut inv = persistence::load_inventory(&db, seller_id);
    inv.gold = deposit - 1;
    persistence::save_inventory(&db, seller_id, &inv);

    let response = create_auction(
        &db,
        &mut state,
        seller_id,
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
    // Gold and inventory unchanged
    assert_eq!(
        persistence::load_inventory(&db, seller_id).gold,
        deposit - 1
    );
    assert_eq!(
        persistence::load_inventory(&db, seller_id).items.len(),
        inv.items.len(),
    );
}

#[test]
fn max_auction_slots_per_character_enforced() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let mut state = AuctionHouseState::default();

    // Fill state with MAX_AUCTIONS_PER_CHARACTER auctions for this seller
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
    // Unit test for duration_secs
    assert_eq!(duration_secs(AuctionDuration::Short), 30 * 60);
    assert_eq!(duration_secs(AuctionDuration::Medium), 2 * 60 * 60);
    assert_eq!(duration_secs(AuctionDuration::Long), 8 * 60 * 60);

    // Integration: create_auction sets expires_at correctly
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
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
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_a, _) = persistence::create_character(&db, 2, "BidderA", 1, 1).expect("a");
    let (bidder_b, _) = persistence::create_character(&db, 3, "BidderB", 1, 1).expect("b");
    let (bidder_c, _) = persistence::create_character(&db, 4, "BidderC", 1, 1).expect("c");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(1000),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    // A bids 100, B tries 100 (stale — min is now 105), C bids 105
    let a = place_bid(&db, &mut state, bidder_a, auction_id, 100);
    assert!(a.success);

    let b_stale = place_bid(&db, &mut state, bidder_b, auction_id, 100);
    assert!(!b_stale.success, "stale bid at old minimum rejected");

    let c = place_bid(&db, &mut state, bidder_c, auction_id, 105);
    assert!(c.success);

    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(105));
    assert_eq!(auction.current_bidder_character_id, Some(bidder_c));

    // A was refunded, B was never charged
    let a_gold = persistence::load_inventory(&db, bidder_a).gold;
    let b_gold = persistence::load_inventory(&db, bidder_b).gold;
    let initial_gold = 100_000;
    assert_eq!(a_gold, initial_gold, "A refunded");
    assert_eq!(b_gold, initial_gold, "B never charged");
}

#[test]
fn bid_at_buyout_price_triggers_immediate_sale() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
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

    // Auction removed (sale completed)
    assert!(!state.auctions.contains_key(&auction_id));

    // Buyer receives item, seller receives gold minus house cut
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
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 500,
            buyout_price: Some(1000),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    // Drain buyer gold to 499
    let mut buyer_inv = persistence::load_inventory(&db, buyer_id);
    buyer_inv.gold = 499;
    persistence::save_inventory(&db, buyer_id, &buyer_inv);

    // Bid at min_bid (500) fails — only have 499
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

    // Buyout (1000) also fails
    let buyout_response = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(!buyout_response.success);
    assert!(state.auctions.contains_key(&auction_id));

    // Gold unchanged
    assert_eq!(persistence::load_inventory(&db, buyer_id).gold, 499);
}

#[test]
fn outbid_sends_notification_mail_to_previous_bidder() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_a_id, _) =
        persistence::create_character(&db, 2, "BidderA", 1, 1).expect("bidder a");
    let (bidder_b_id, _) =
        persistence::create_character(&db, 3, "BidderB", 1, 1).expect("bidder b");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
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

    // BidderB outbids BidderA
    let response = place_bid(&db, &mut state, bidder_b_id, auction_id, 105);
    assert!(response.success);

    // BidderA receives outbid notification mail
    let mail_a = persistence::load_mailbox(&db, bidder_a_id);
    assert_eq!(mail_a.len(), 1);
    assert_eq!(mail_a[0].subject, "Outbid");
    assert!(mail_a[0].body.contains(&source_item.name));
    assert_eq!(mail_a[0].attached_money, 0);
    assert!(mail_a[0].attached_item.is_none());

    // BidderB has no mail yet
    assert!(persistence::load_mailbox(&db, bidder_b_id).is_empty());
}

#[test]
fn expired_auction_without_bid_returns_item_to_seller() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 4,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    // Force expiry by setting time in the past
    state
        .auctions
        .get_mut(&auction_id)
        .expect("auction")
        .expires_at_unix_secs = 0;

    // Replicate expire_auctions logic for the no-bidder path
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

    // Seller receives item back via mail
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
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    // Create and expire auction (no bids → item returned via mail)
    create_auction(
        &db,
        &mut state,
        seller_id,
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

    // Claim mail to get item back in inventory
    let mail = persistence::load_mailbox(&db, seller_id);
    let claim = claim_mail(&db, seller_id, mail[0].mail_id);
    assert!(claim.success);

    // Re-list the returned item
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

#[test]
fn bid_on_expired_auction_fails_gracefully() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_id, _) = persistence::create_character(&db, 2, "Bidder", 1, 1).expect("bidder");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let bidder_gold_before = persistence::load_inventory(&db, bidder_id).gold;
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    // Auction expires (removed from state) before bid arrives
    state.auctions.remove(&auction_id);

    let bid = place_bid(&db, &mut state, bidder_id, auction_id, 100);
    assert!(!bid.success);

    let buyout = buyout_auction(&db, &mut state, bidder_id, auction_id);
    assert!(!buyout.success);

    // Bidder gold unchanged
    assert_eq!(
        persistence::load_inventory(&db, bidder_id).gold,
        bidder_gold_before
    );
}

#[test]
fn deposit_refunded_on_successful_sale() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let seller_gold_before = persistence::load_inventory(&db, seller_id).gold;
    let mut state = AuctionHouseState::default();

    let deposit = deposit_cost(source_item.vendor_sell_price, 1, AuctionDuration::Long);
    assert!(deposit > 0, "test needs non-zero deposit");

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Long,
        },
    );
    // Deposit deducted
    let after_list = persistence::load_inventory(&db, seller_id).gold;
    assert_eq!(after_list, seller_gold_before - deposit);

    let auction_id = *state.auctions.keys().next().expect("auction id");
    buyout_auction(&db, &mut state, buyer_id, auction_id);

    // Seller mail includes proceeds + deposit refund
    let mail = persistence::load_mailbox(&db, seller_id);
    let sale_proceeds = 500 - (500 * HOUSE_CUT_BPS / 10_000);
    assert_eq!(mail[0].attached_money, sale_proceeds + deposit);

    // After claiming, seller has original gold - deposit + proceeds + deposit = original + proceeds
    claim_mail(&db, seller_id, mail[0].mail_id);
    assert_eq!(
        persistence::load_inventory(&db, seller_id).gold,
        seller_gold_before + sale_proceeds,
    );
}

#[test]
fn auction_snapshot_independent_of_inventory_changes() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
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

    // Modify the remaining inventory item's name and quality
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

    // Auction snapshot is unchanged
    let auction = state.auctions.values().next().expect("auction");
    assert_eq!(auction.item.name, snapshotted_name);
    assert_eq!(auction.item.quality, snapshotted_quality);

    // Search returns original stats
    let results = search_auctions(&state.auctions, &text_query(&snapshotted_name));
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results[0].item.name, snapshotted_name);
}

#[test]
fn empty_auction_house_search_returns_gracefully() {
    let state = AuctionHouseState::default();

    // Empty text query on empty house
    let r = search_auctions(&state.auctions, &text_query(""));
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());

    // Filtered query on empty house
    let mut query = text_query("sword");
    query.quality = Some(3);
    query.min_level = Some(10);
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());

    // Pagination on empty house
    let mut query = text_query("");
    query.page = 5;
    let r = search_auctions(&state.auctions, &query);
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());
}

#[test]
fn seller_offline_auction_still_completes() {
    // Seller creates auction, then "goes offline" (no further interaction).
    // Buyer buys out — seller receives gold via mail, claimable on next login.
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    // Seller is offline — buyer buys out without seller involvement
    let response = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(response.success);

    // Seller's mail is persisted — available on next login
    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    assert!(seller_mail[0].attached_money > 0);

    // Seller claims mail on next login
    let seller_gold_before_claim = persistence::load_inventory(&db, seller_id).gold;
    claim_mail(&db, seller_id, seller_mail[0].mail_id);
    let seller_gold_after_claim = persistence::load_inventory(&db, seller_id).gold;
    assert!(seller_gold_after_claim > seller_gold_before_claim);
}

#[test]
fn gold_overflow_saturates_at_max() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");

    // Set seller gold near u32::MAX
    let mut inv = persistence::load_inventory(&db, seller_id);
    inv.gold = u32::MAX - 10;
    persistence::save_inventory(&db, seller_id, &inv);

    // Claim mail with 100 gold — should saturate, not wrap
    deliver_mail(&db, seller_id, "Test", "gold overflow", 100, None);
    let mail = persistence::load_mailbox(&db, seller_id);
    claim_mail(&db, seller_id, mail[0].mail_id);

    let final_gold = persistence::load_inventory(&db, seller_id).gold;
    assert_eq!(final_gold, u32::MAX, "gold saturates at u32::MAX");
}

#[test]
fn unique_item_prevents_duplicate_listing() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let mut state = AuctionHouseState::default();

    // Mark the first item as unique_auction
    let mut inv = persistence::load_inventory(&db, seller_id);
    inv.items[0].unique_auction = true;
    // Give extra stack so we can try listing twice
    inv.items[0].stack_count = 10;
    persistence::save_inventory(&db, seller_id, &inv);

    let item = persistence::load_inventory(&db, seller_id).items[0].clone();

    // First listing succeeds
    let first = create_auction(
        &db,
        &mut state,
        seller_id,
        &CreateAuction {
            item_guid: item.item_guid,
            stack_count: 3,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Short,
        },
    );
    assert!(first.success);

    // Second listing of same item_id rejected
    let remaining = persistence::load_inventory(&db, seller_id).items[0].clone();
    let second = create_auction(
        &db,
        &mut state,
        seller_id,
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
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_id, _) = persistence::create_character(&db, 2, "Bidder", 1, 1).expect("bidder");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
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

    // Force expiry
    state
        .auctions
        .get_mut(&auction_id)
        .expect("auction")
        .expires_at_unix_secs = 0;

    // Replicate expire_auctions with-bid path
    let auction = state.auctions.remove(&auction_id).expect("auction");
    let winner_id = auction.current_bidder_character_id.expect("has bidder");
    let winning_bid = auction.current_bid.unwrap_or(auction.min_bid);
    persistence::delete_auction(&db, auction_id);
    complete_sale(&db, &auction, winner_id, winning_bid);

    // Seller receives gold minus house cut plus deposit refund
    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    let dep = deposit_cost(source_item.vendor_sell_price, 2, AuctionDuration::Short);
    let expected_proceeds = 200 - (200 * HOUSE_CUT_BPS / 10_000) + dep;
    assert_eq!(seller_mail[0].attached_money, expected_proceeds);
    assert!(seller_mail[0].attached_item.is_none());

    // Winner receives item
    let bidder_mail = persistence::load_mailbox(&db, bidder_id);
    assert_eq!(bidder_mail.len(), 1);
    assert_eq!(bidder_mail[0].attached_money, 0);
    let won_item = bidder_mail[0].attached_item.as_ref().expect("won item");
    assert_eq!(won_item.item_id, source_item.item_id);
    assert_eq!(won_item.stack_count, 2);
}

#[test]
fn house_cut_deducted_on_successful_sale() {
    // Test multiple prices to verify 5% cut formula including truncation
    // Expected = price - (price * 500 / 10000) + deposit
    // deposit = vendor_sell_price(13) * stack_count(1) * short_mult(1) = 13
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
        let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
        let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
        let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
        let mut state = AuctionHouseState::default();

        create_auction(
            &db,
            &mut state,
            seller_id,
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
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let buyer_gold_before = persistence::load_inventory(&db, buyer_id).gold;
    let mut state = AuctionHouseState::default();

    create_auction(
        &db,
        &mut state,
        seller_id,
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

    // Auction removed from state
    assert!(!state.auctions.contains_key(&auction_id));

    // Buyer gold deducted by buyout price
    let buyer_gold_after = persistence::load_inventory(&db, buyer_id).gold;
    assert_eq!(buyer_gold_after, buyer_gold_before - 300);

    // Buyer receives item via mail
    let buyer_mail = persistence::load_mailbox(&db, buyer_id);
    assert_eq!(buyer_mail.len(), 1);
    let buyer_item = buyer_mail[0].attached_item.as_ref().expect("item in mail");
    assert_eq!(buyer_item.item_id, source_item.item_id);
    assert_eq!(buyer_item.stack_count, 2);

    // Seller receives gold minus house cut plus deposit refund
    let seller_mail = persistence::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    let deposit = deposit_cost(source_item.vendor_sell_price, 2, AuctionDuration::Medium);
    let expected_gold = 300 - (300 * HOUSE_CUT_BPS / 10_000) + deposit;
    assert_eq!(seller_mail[0].attached_money, expected_gold);
}

#[test]
fn buyout_and_claim_mail_delivers_item_and_gold() {
    let db = persistence::temp_database();
    let (seller_id, _) = persistence::create_character(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) = persistence::create_character(&db, 2, "Buyer", 1, 1).expect("buyer");
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
