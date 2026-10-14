use super::*;

#[test]
fn transmogacq_auction_won_mail_keeps_boe_uncollected() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (seller, _) = persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).unwrap();
    let (buyer, _) = persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).unwrap();
    let templates =
        crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap();
    let mut inventory = persistence::load_inventory(&db, seller);
    let guid = persistence::next_id(&db, "item_instance");
    inventory.items.push(persistence::InventoryItemData {
        item_guid: guid,
        item_id: 25,
        name: "Worn Shortsword".into(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 1,
        unique_auction: false,
    });
    persistence::save_inventory(&db, seller, &inventory);
    let mut state = AuctionHouseState::default();
    assert!(
        create_auction(
            &db,
            &templates,
            &mut state,
            seller,
            crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
            &CreateAuction {
                item_guid: guid,
                stack_count: 1,
                min_bid: 100,
                buyout_price: Some(500),
                duration: AuctionDuration::Short
            }
        )
        .success
    );
    let id = *state.auctions.keys().next().unwrap();
    assert!(buyout_auction(&db, &mut state, buyer, id).success);
    let mail_id = mail_store::load_mailbox(&db, buyer)[0].mail_id;
    take_mail(&db, buyer, mail_id, &mut GoldUpdateQueue::default());
    assert!(
        persistence::load_inventory(&db, buyer)
            .items
            .iter()
            .any(|item| item.item_id == 25)
    );
    assert!(!crate::persistence::appearance_store::load_account_appearances(&db, 2).has(154));
    drop(db);
    let reopened = redb::Database::open(file.path()).unwrap();
    persistence::create_tables(&reopened);
    assert!(!crate::persistence::appearance_store::load_account_appearances(&reopened, 2).has(154));
}

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
        &ItemTemplates::default(),
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
    app.insert_resource(crate::persistence::GameDatabase::new(db));
    let db = std::sync::Arc::clone(
        &app.world()
            .resource::<crate::persistence::GameDatabase>()
            .db,
    );
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
            &ItemTemplates::default(),
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

        let mail = mail_store::load_mailbox(&db, seller_id);
        let sale_proceeds = 500 - (500 * HOUSE_CUT_BPS / 10_000);
        assert_eq!(mail[0].money, sale_proceeds + deposit);
        take_mail(&db, seller_id, mail[0].mail_id, &mut gold_updates);
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
        &ItemTemplates::default(),
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

    let results = search_auctions(&state.auctions, &text_query(&snapshotted_name), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(results.total_results, 1);
    assert_eq!(results.results[0].item.name, snapshotted_name);
}

#[test]
fn empty_auction_house_search_returns_gracefully() {
    let state = AuctionHouseState::default();

    let r = search_auctions(&state.auctions, &text_query(""), &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());

    let mut query = text_query("sword");
    query.quality = Some(3);
    query.min_level = Some(10);
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
    assert_eq!(r.total_results, 0);
    assert!(r.results.is_empty());

    let mut query = text_query("");
    query.page = 5;
    let r = search_auctions(&state.auctions, &query, &|_| {
        shared::item_data::ItemDefinitionSource::Retail
    });
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
        &ItemTemplates::default(),
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

    let seller_mail = mail_store::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    assert!(seller_mail[0].money > 0);

    let seller_gold_before_claim = persistence::load_inventory(&db, seller_id).gold;
    take_mail(
        &db,
        seller_id,
        seller_mail[0].mail_id,
        &mut GoldUpdateQueue::default(),
    );
    let seller_gold_after_claim = persistence::load_inventory(&db, seller_id).gold;
    assert!(seller_gold_after_claim > seller_gold_before_claim);
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
        &ItemTemplates::default(),
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
        &ItemTemplates::default(),
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
        &ItemTemplates::default(),
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
    crate::economy::store::commit_market(
        &db,
        None,
        &crate::economy::store::MarketChanges {
            deleted_auctions: vec![auction_id],
            mails: super::super::operations::sale_mails(&db, &auction, winner_id, winning_bid),
            ..Default::default()
        },
    )
    .unwrap();

    let seller_mail = mail_store::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    let dep = deposit_cost(source_item.vendor_sell_price, 2, AuctionDuration::Short);
    let expected_proceeds = 200 - (200 * HOUSE_CUT_BPS / 10_000) + dep;
    assert_eq!(seller_mail[0].money, expected_proceeds);
    assert!(seller_mail[0].items.is_empty());

    let bidder_mail = mail_store::load_mailbox(&db, bidder_id);
    assert_eq!(bidder_mail.len(), 1);
    assert_eq!(bidder_mail[0].money, 0);
    let won_item = bidder_mail[0]
        .items
        .first()
        .map(|attached| &attached.item)
        .expect("won item");
    assert_eq!(won_item.item_id, source_item.item_id);
    assert_eq!(won_item.stack_count, 2);
}

#[test]
fn house_cut_deducted_on_successful_sale() {
    let deposit = 13_u64;
    let cases: &[(u64, u64)] = &[
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
            &ItemTemplates::default(),
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

        let seller_mail = mail_store::load_mailbox(&db, seller_id);
        assert_eq!(
            seller_mail[0].money, expected_proceeds,
            "buyout={buyout_price}: expected {expected_proceeds}, got {}",
            seller_mail[0].money,
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
        &ItemTemplates::default(),
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

    let buyer_mail = mail_store::load_mailbox(&db, buyer_id);
    assert_eq!(buyer_mail.len(), 1);
    let buyer_item = buyer_mail[0]
        .items
        .first()
        .map(|attached| &attached.item)
        .expect("item in mail");
    assert_eq!(buyer_item.item_id, source_item.item_id);
    assert_eq!(buyer_item.stack_count, 2);

    let seller_mail = mail_store::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail.len(), 1);
    let deposit = deposit_cost(source_item.vendor_sell_price, 2, AuctionDuration::Medium);
    let expected_gold = 300 - (300 * HOUSE_CUT_BPS / 10_000) + deposit;
    assert_eq!(seller_mail[0].money, expected_gold);
}

#[test]
fn buyout_mail_delivers_item_and_gold_taken_at_the_mailbox() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    let seller_gold_before = persistence::load_inventory(&db, seller_id).gold;
    let buyer_gold_before = persistence::load_inventory(&db, buyer_id).gold;
    let buyer_owned_before = owned(&db, buyer_id, source_item.item_id);
    let mut state = AuctionHouseState::default();
    let deposit = deposit_cost(source_item.vendor_sell_price, 3, AuctionDuration::Medium);

    create_auction(
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
    let auction_id = *state.auctions.keys().next().expect("auction id");
    let response = buyout_auction(&db, &mut state, buyer_id, auction_id);
    assert!(response.success);
    assert!(!state.auctions.contains_key(&auction_id));

    let seller_mail = mail_store::load_mailbox(&db, seller_id);
    let buyer_mail = mail_store::load_mailbox(&db, buyer_id);
    assert_eq!(seller_mail.len(), 1);
    assert_eq!(buyer_mail.len(), 1);
    assert_eq!(seller_mail[0].money, 190 + deposit);
    assert_eq!(
        buyer_mail[0]
            .items
            .first()
            .map(|attached| &attached.item)
            .expect("buyer item mail")
            .stack_count,
        3
    );

    let mut gold_updates = GoldUpdateQueue::default();
    take_mail(&db, seller_id, seller_mail[0].mail_id, &mut gold_updates);
    take_mail(&db, buyer_id, buyer_mail[0].mail_id, &mut gold_updates);
    assert_eq!(
        persistence::load_inventory(&db, seller_id).gold,
        seller_gold_before + 190
    );
    assert_eq!(
        persistence::load_inventory(&db, buyer_id).gold,
        buyer_gold_before - 200
    );
    assert_eq!(
        owned(&db, buyer_id, source_item.item_id),
        buyer_owned_before + 3
    );
}

fn owned(db: &redb::Database, character_id: u64, item_id: u32) -> u32 {
    persistence::load_inventory(db, character_id)
        .items
        .iter()
        .filter(|item| item.item_id == item_id)
        .map(|item| item.stack_count)
        .sum()
}

#[test]
fn buyout_above_the_u32_copper_range_pays_the_seller_in_full() {
    let db = persistence::temp_database();
    let (seller_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let (buyer_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).expect("buyer");
    let source_item = persistence::load_inventory(&db, seller_id).items[0].clone();
    // 1,000,000 gold: above u32::MAX copper (~429,496 gold), below Retail's cap.
    let buyout = 10_000_000_000_u64;
    let mut buyer_inventory = persistence::load_inventory(&db, buyer_id);
    buyer_inventory.gold = buyout + 5;
    persistence::save_inventory(&db, buyer_id, &buyer_inventory);
    let mut state = AuctionHouseState::default();

    let created = create_auction(
        &db,
        &ItemTemplates::default(),
        &mut state,
        seller_id,
        crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
        &CreateAuction {
            item_guid: source_item.item_guid,
            stack_count: 1,
            min_bid: 5_000_000_000,
            buyout_price: Some(buyout),
            duration: AuctionDuration::Short,
        },
    );
    assert!(created.success, "{}", created.message);
    let auction_id = *state.auctions.keys().next().expect("auction id");

    assert!(buyout_auction(&db, &mut state, buyer_id, auction_id).success);

    assert_eq!(persistence::load_inventory(&db, buyer_id).gold, 5);
    let deposit = deposit_cost(source_item.vendor_sell_price, 1, AuctionDuration::Short);
    let seller_mail = mail_store::load_mailbox(&db, seller_id);
    assert_eq!(seller_mail[0].money, 9_500_000_000 + deposit);
}

fn atomic_fixture(db: &redb::Database) -> AuctionHouseState {
    for (id, name) in [(1, "Seller"), (2, "Buyer"), (3, "Bidder")] {
        let (character, _) =
            persistence::create_character_with_test_bags(db, id, name, 1, 1).unwrap();
        assert_eq!(character, id);
        persistence::save_inventory(
            db,
            id,
            &InventoryData {
                gold: 10_000,
                items: vec![],
            },
        );
    }
    let mut auction = test_auction(91, "Copper Ore", 1, 1);
    auction.deposit = 37;
    auction.buyout_price = Some(1_001);
    persistence::save_auction(db, &auction);
    AuctionHouseState {
        auctions: [(91, auction)].into(),
        ..Default::default()
    }
}

pub(super) fn corrupt_auction_mailbox(db: &redb::Database, recipient: u64) {
    let tx = db.begin_write().unwrap();
    tx.open_table(mail_store::MAILBOXES)
        .unwrap()
        .insert(recipient, b"invalid".as_slice())
        .unwrap();
    crate::persistence_commit::commit(tx).unwrap();
}

#[test]
fn atomic_stale_bid_buyout_cancel_leave_all_balances_and_memory_unchanged() {
    for operation in ["bid", "buyout", "cancel"] {
        let db = persistence::temp_database();
        let mut state = atomic_fixture(&db);
        let original = state.auctions[&91].clone();
        persistence::delete_auction(&db, 91);
        let response = match operation {
            "bid" => place_bid(&db, &mut state, 2, 91, 100),
            "buyout" => buyout_auction(&db, &mut state, 2, 91),
            _ => cancel_auction(&db, &mut state, 1, 91),
        };
        assert!(
            !response.success,
            "{operation} must reject stale settlement"
        );
        assert_eq!(state.auctions[&91], original);
        assert!(persistence::load_auctions(&db).is_empty());
        for id in [1, 2, 3] {
            assert_eq!(persistence::load_inventory(&db, id).gold, 10_000);
            assert!(mail_store::load_mailbox(&db, id).is_empty());
        }
    }
}

#[test]
fn atomic_mail_failure_rolls_back_buyout_bid_and_cancel_before_publication() {
    for operation in ["bid", "buyout", "cancel"] {
        let db = persistence::temp_database();
        let mut state = atomic_fixture(&db);
        if operation != "cancel" {
            let auction = state.auctions.get_mut(&91).unwrap();
            auction.current_bid = Some(100);
            auction.current_bidder_character_id = Some(3);
            persistence::save_auction(&db, auction);
        }
        let original = state.auctions[&91].clone();
        corrupt_auction_mailbox(&db, if operation == "bid" { 3 } else { 1 });
        let response = match operation {
            "bid" => place_bid(&db, &mut state, 2, 91, 200),
            "buyout" => buyout_auction(&db, &mut state, 2, 91),
            _ => cancel_auction(&db, &mut state, 1, 91),
        };
        assert!(
            !response.success,
            "{operation} must return transaction failure"
        );
        assert_eq!(state.auctions[&91], original);
        assert_eq!(persistence::load_auctions(&db), vec![original]);
        for id in [1, 2, 3] {
            assert_eq!(persistence::load_inventory(&db, id).gold, 10_000);
        }
        assert!(mail_store::load_mailbox(&db, 2).is_empty());
    }
}

#[test]
fn atomic_concurrent_duplicate_buyout_charges_and_delivers_once() {
    use std::sync::{Arc, Barrier};
    let db = Arc::new(persistence::temp_database());
    let state = atomic_fixture(&db);
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            let mut state = AuctionHouseState {
                auctions: state.auctions.clone(),
                ..Default::default()
            };
            std::thread::spawn(move || {
                barrier.wait();
                buyout_auction(&db, &mut state, 2, 91).success
            })
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .filter(|success| *success)
            .count(),
        1
    );
    assert_eq!(persistence::load_inventory(&db, 2).gold, 8_999);
    assert!(persistence::load_auctions(&db).is_empty());
    let seller_mail = mail_store::load_mailbox(&db, 1);
    assert_eq!(seller_mail.len(), 1);
    assert_eq!(seller_mail[0].money, 988);
    assert_eq!(mail_store::load_mailbox(&db, 2).len(), 1);
}

#[test]
fn atomic_restart_rebid_outbid_buyout_persists_exact_escrow_and_delivery() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let mut state = atomic_fixture(&db);
    assert!(place_bid(&db, &mut state, 2, 91, 100).success);
    assert!(place_bid(&db, &mut state, 2, 91, 200).success);
    assert_eq!(persistence::load_inventory(&db, 2).gold, 9_800);
    drop(db);
    let db = redb::Database::open(file.path()).unwrap();
    let mut state = AuctionHouseState {
        auctions: persistence::load_auctions(&db)
            .into_iter()
            .map(|a| (a.auction_id, a))
            .collect(),
        ..Default::default()
    };
    assert!(place_bid(&db, &mut state, 3, 91, 300).success);
    assert_eq!(persistence::load_inventory(&db, 2).gold, 10_000);
    assert!(buyout_auction(&db, &mut state, 3, 91).success);
    drop(db);
    let db = redb::Database::open(file.path()).unwrap();
    assert!(persistence::load_auctions(&db).is_empty());
    assert_eq!(persistence::load_inventory(&db, 3).gold, 8_999);
    assert_eq!(mail_store::load_mailbox(&db, 1)[0].money, 988);
    assert_eq!(mail_store::load_mailbox(&db, 2).len(), 1);
    let delivery = mail_store::load_mailbox(&db, 3);
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].items[0].item.item_guid, 91);
}

#[test]
fn atomic_concurrent_create_lists_source_item_and_charges_deposit_once() {
    use std::sync::{Arc, Barrier};
    let db = Arc::new(persistence::temp_database());
    let (seller, _) = persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).unwrap();
    let mut inventory = persistence::load_inventory(&db, seller);
    inventory.items[0].stack_count = 1;
    let source = inventory.items[0].clone();
    let initial_gold = inventory.gold;
    persistence::save_inventory(&db, seller, &inventory);
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let db = db.clone();
            let barrier = barrier.clone();
            let item_guid = source.item_guid;
            std::thread::spawn(move || {
                barrier.wait();
                create_auction(
                    &db,
                    &ItemTemplates::default(),
                    &mut AuctionHouseState::default(),
                    seller,
                    0,
                    &CreateAuction {
                        item_guid,
                        stack_count: 1,
                        min_bid: 100,
                        buyout_price: Some(500),
                        duration: AuctionDuration::Short,
                    },
                )
                .success
            })
        })
        .collect();
    let successes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .filter(|success| *success)
        .count();
    assert_eq!(successes, 1);
    assert_eq!(persistence::load_auctions(&db).len(), 1);
    let inventory = persistence::load_inventory(&db, seller);
    assert_eq!(
        inventory.gold,
        initial_gold - deposit_cost(source.vendor_sell_price, 1, AuctionDuration::Short)
    );
    assert!(
        !inventory
            .items
            .iter()
            .any(|item| item.item_guid == source.item_guid)
    );
}

#[test]
fn atomic_expired_deadline_rejects_bid_and_buyout_before_sweep() {
    for operation in ["bid", "buyout"] {
        let db = persistence::temp_database();
        let mut state = atomic_fixture(&db);
        let auction = state.auctions.get_mut(&91).unwrap();
        auction.expires_at_unix_secs = unix_now() - 1;
        persistence::save_auction(&db, auction);
        let original = auction.clone();
        let response = if operation == "bid" {
            place_bid(&db, &mut state, 2, 91, 100)
        } else {
            buyout_auction(&db, &mut state, 2, 91)
        };
        assert!(
            !response.success,
            "{operation} must reject an expired deadline"
        );
        assert_eq!(state.auctions[&91], original);
        assert_eq!(persistence::load_auctions(&db), vec![original]);
        assert_eq!(persistence::load_inventory(&db, 2).gold, 10_000);
        assert!(mail_store::load_mailbox(&db, 1).is_empty());
        assert!(mail_store::load_mailbox(&db, 2).is_empty());
    }
}
