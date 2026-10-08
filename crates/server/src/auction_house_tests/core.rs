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
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("character");
    let inventory_before = persistence::load_inventory(&db, character_id);
    let source_item = inventory_before.items[0].clone();
    let original_count = source_item.stack_count;
    let mut state = AuctionHouseState::default();

    let response = create_auction(
        &db,
        &ItemTemplates::default(),
        &mut state,
        character_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
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
fn successful_auction_deposit_updates_online_gold() {
    let db = persistence::temp_database();
    let (character_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("character");
    let inventory_before = persistence::load_inventory(&db, character_id);
    let source_item = inventory_before.items[0].clone();
    let mut state = AuctionHouseState::default();
    let mut app = App::new();
    app.add_plugins(crate::gold::GoldPlugin);
    let player = app
        .world_mut()
        .spawn((
            shared::components::Player {
                name: "Seller".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            crate::persistence::ClientId(character_id),
            shared::components::Gold(inventory_before.gold),
        ))
        .id();

    let response = {
        let mut gold_updates = app
            .world_mut()
            .resource_mut::<crate::gold::GoldUpdateQueue>();
        super::super::handlers::create_auction_with_gold_update(
            &db,
            &ItemTemplates::default(),
            &mut state,
            character_id,
            crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
            &CreateAuction {
                item_guid: source_item.item_guid,
                stack_count: 5,
                min_bid: 100,
                buyout_price: Some(150),
                duration: AuctionDuration::Medium,
            },
            &mut gold_updates,
        )
    };
    assert!(response.success);
    app.update();

    let persisted_gold = persistence::load_inventory(&db, character_id).gold;
    assert!(persisted_gold < inventory_before.gold);
    assert_eq!(
        app.world()
            .entity(player)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        persisted_gold
    );
}

#[test]
fn place_bid_refunds_previous_bidder_and_updates_current_bid() {
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
        &ItemTemplates::default(),
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
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
    let mut app = App::new();
    app.add_plugins(crate::gold::GoldPlugin);
    let bidder_a = app
        .world_mut()
        .spawn((
            shared::components::Player {
                name: "BidderA".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            crate::persistence::ClientId(bidder_a_id),
            shared::components::Gold(bidder_a_gold_before),
        ))
        .id();
    let bidder_b = app
        .world_mut()
        .spawn((
            shared::components::Player {
                name: "BidderB".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            crate::persistence::ClientId(bidder_b_id),
            shared::components::Gold(bidder_b_gold_before),
        ))
        .id();

    let (first, second) = {
        let mut gold_updates = app
            .world_mut()
            .resource_mut::<crate::gold::GoldUpdateQueue>();
        let first = super::super::handlers::place_bid_with_gold_updates(
            &db,
            &mut state,
            bidder_a_id,
            auction_id,
            100,
            &mut gold_updates,
        );
        let second = super::super::handlers::place_bid_with_gold_updates(
            &db,
            &mut state,
            bidder_b_id,
            auction_id,
            105,
            &mut gold_updates,
        );
        (first, second)
    };
    app.update();

    assert!(first.success);
    assert!(second.success);
    let auction = state.auctions.get(&auction_id).expect("auction");
    assert_eq!(auction.current_bid, Some(105));
    assert_eq!(auction.current_bidder_character_id, Some(bidder_b_id));
    assert_eq!(
        persistence::load_inventory(&db, bidder_a_id).gold,
        bidder_a_gold_before
    );
    let bidder_a_gold = persistence::load_inventory(&db, bidder_a_id).gold;
    let bidder_b_gold = persistence::load_inventory(&db, bidder_b_id).gold;
    assert_eq!(bidder_a_gold, bidder_a_gold_before);
    assert_eq!(bidder_b_gold, bidder_b_gold_before - 105);
    assert_eq!(
        app.world()
            .entity(bidder_a)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        bidder_a_gold
    );
    assert_eq!(
        app.world()
            .entity(bidder_b)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        bidder_b_gold
    );
}
