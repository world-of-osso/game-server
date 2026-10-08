use super::*;

#[test]
fn create_auction_appears_in_search_results() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("character");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();

    let response = create_auction(
        &db,
        &ItemTemplates::default(),
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
    assert!(response.success);

    let results = search_auctions(&state.auctions, &text_query(""), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results.len(), 1);
    let listing = &results.results[0];
    assert_eq!(listing.item.name, source_item.name);
    assert_eq!(listing.stack_count, 3);
    assert_eq!(listing.min_bid, 100);
    assert_eq!(listing.buyout_price, Some(200));
    assert_eq!(listing.owner_name, "Seller");

    let name_prefix = &source_item.name[..3].to_ascii_lowercase();
    let name_results = search_auctions(&state.auctions, &text_query(name_prefix), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(name_results.total_results, 1);

    let empty_results = search_auctions(
        &state.auctions,
        &text_query("zzz_nonexistent_item"),
        &|_| shared::item_data::ItemDefinitionSource::Retail,
    );
    assert_eq!(empty_results.total_results, 0);
    assert!(empty_results.results.is_empty());
}

#[test]
fn bid_recorded_and_underbid_rejected() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (bidder_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Bidder", 1, 1).expect("bidder");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let mut state = AuctionHouseState::default();
    create_auction(
        &db,
        &ItemTemplates::default(),
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 2,
            min_bid: 100,
            buyout_price: Some(500),
            duration: AuctionDuration::Medium,
        },
    );
    let auction_id = *state.auctions.keys().next().expect("auction id");

    let first = place_bid(&db, &mut state, bidder_id, auction_id, 100);
    assert!(first.success);
    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(100));
    assert_eq!(auction.current_bidder_character_id, Some(bidder_id));

    let low_bid = place_bid(&db, &mut state, bidder_id, auction_id, 104);
    assert!(!low_bid.success);
    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(100));

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

    let results = search_auctions(&state.auctions, &text_query("sword"), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 2);

    let results = search_auctions(&state.auctions, &text_query("iron"), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 2);

    let mut query = text_query("");
    query.quality = Some(3);
    let results = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results[0].item.name, "Steel Sword");

    let mut query = text_query("");
    query.min_level = Some(10);
    let results = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 2);

    let mut query = text_query("");
    query.max_level = Some(5);
    let results = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 2);

    let mut query = text_query("iron");
    query.min_level = Some(1);
    query.max_level = Some(10);
    let results = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 2);

    let mut query = text_query("sword");
    query.quality = Some(3);
    let results = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results[0].item.name, "Steel Sword");
}

#[test]
fn search_returns_empty_for_non_matching_filters() {
    let mut state = AuctionHouseState::default();
    state
        .auctions
        .insert(1, test_auction(1, "Iron Sword", 2, 20));

    let mut query = text_query("");
    query.quality = Some(4);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 0);

    let mut query = text_query("");
    query.min_level = Some(30);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 0);

    let mut query = text_query("");
    query.max_level = Some(10);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 0);

    let r = search_auctions(&state.auctions, &text_query("zzz"), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 0);

    let mut query = text_query("iron");
    query.quality = Some(4);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
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

    let mut query = text_query("iron");
    query.quality = Some(2);
    query.min_level = Some(10);
    query.max_level = Some(25);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 1);
    assert_eq!(r.results[0].item.name, "Iron Sword");

    let mut query = text_query("iron");
    query.quality = Some(2);
    query.min_level = Some(1);
    query.max_level = Some(25);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 2);

    let mut query = text_query("iron");
    query.quality = Some(4);
    query.min_level = Some(10);
    query.max_level = Some(25);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
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

    let mut query = text_query("");
    query.faction = 1;
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 2);
    let names: Vec<&str> = r.results.iter().map(|l| l.item.name.as_str()).collect();
    assert!(names.contains(&"Alliance Sword"));
    assert!(names.contains(&"Neutral Cloth"));

    let mut query = text_query("");
    query.faction = 2;
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 2);
    let names: Vec<&str> = r.results.iter().map(|l| l.item.name.as_str()).collect();
    assert!(names.contains(&"Horde Axe"));
    assert!(names.contains(&"Neutral Cloth"));

    let r = search_auctions(&state.auctions, &text_query(""), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 3);
}

#[test]
fn sort_by_price_time_remaining_and_name() {
    let mut state = AuctionHouseState::default();

    let mut a = test_auction(1, "Copper Ore", 1, 1);
    a.min_bid = 50;
    a.buyout_price = Some(300);
    a.expires_at_unix_secs = unix_now() + 10;

    let mut b = test_auction(2, "Iron Ore", 2, 10);
    b.min_bid = 200;
    b.buyout_price = Some(100);
    b.expires_at_unix_secs = unix_now() + 3600 * 5;

    let mut c = test_auction(3, "Arcane Dust", 1, 5);
    c.min_bid = 100;
    c.buyout_price = Some(200);
    c.expires_at_unix_secs = unix_now() + 300;

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
                ..Default::default()
            },
            &|_| shared::item_data::ItemDefinitionSource::Retail,
        )
    };
    let names = |r: AuctionSearchResults| -> Vec<String> {
        r.results.into_iter().map(|l| l.item.name).collect()
    };

    let r = sort(AuctionSortField::Name, AuctionSortDir::Asc);
    assert_eq!(names(r), ["Arcane Dust", "Copper Ore", "Iron Ore"]);

    let r = sort(AuctionSortField::Name, AuctionSortDir::Desc);
    assert_eq!(names(r), ["Iron Ore", "Copper Ore", "Arcane Dust"]);

    let r = sort(AuctionSortField::MinBid, AuctionSortDir::Asc);
    assert_eq!(names(r), ["Copper Ore", "Arcane Dust", "Iron Ore"]);

    let r = sort(AuctionSortField::Buyout, AuctionSortDir::Asc);
    assert_eq!(names(r), ["Iron Ore", "Arcane Dust", "Copper Ore"]);

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
                ..Default::default()
            },
            &|_| shared::item_data::ItemDefinitionSource::Retail,
        )
    };

    let r = page(0, 50);
    assert_eq!(r.total_results, 75);
    assert_eq!(r.results.len(), 50);

    let r = page(1, 50);
    assert_eq!(r.total_results, 75);
    assert_eq!(r.results.len(), 25);

    let r = page(2, 50);
    assert_eq!(r.total_results, 75);
    assert_eq!(r.results.len(), 0);

    let r = page(0, 100);
    assert_eq!(r.results.len(), 50);

    let r = page(0, 10);
    assert_eq!(r.results.len(), 10);
    let r = page(7, 10);
    assert_eq!(r.results.len(), 5);

    let r = page(0, 0);
    assert_eq!(r.results.len(), 1);
}
