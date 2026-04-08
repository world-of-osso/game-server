use super::*;

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
