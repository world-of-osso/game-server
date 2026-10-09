//! Expiry uses an injected clock, with real posting/bidding and reopened redb mail.
use super::*;
use crate::maps::difficulty::ServerClock;

#[test]
fn injected_clock_unsold_expiry_returns_item_by_persisted_mail() {
    assert_clock_expiry(false);
}

#[test]
fn injected_clock_bid_expiry_persists_seller_proceeds_and_buyer_item() {
    assert_clock_expiry(true);
}

fn advance_clock_through_admin(app: &mut App) {
    use crate::admin_types::{AdminRequest, AdminResponse};
    use crate::maps::difficulty::AdminInstances;
    let mut system =
        bevy::ecs::system::SystemState::<(AdminInstances, Res<GameDatabase>)>::new(app.world_mut());
    let (mut admin, db) = system.get_mut(app.world_mut()).unwrap();
    let response = admin.dispatch(&AdminRequest::ClockAdvance { seconds: 1 }, &db);
    assert!(matches!(
        response,
        Some(AdminResponse::ClockAdvanced { offset_secs: 1, .. })
    ));
}

fn assert_clock_expiry(with_bid: bool) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auction-clock.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let (seller, _) = persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).unwrap();
    let (buyer, _) = persistence::create_character_with_test_bags(&db, 2, "Buyer", 1, 1).unwrap();
    let mut item = persistence::load_inventory(&db, seller).items[0].clone();
    item.stack_count = 2;
    let buyer_gold = persistence::load_inventory(&db, buyer).gold;
    let mut state = AuctionHouseState::default();
    assert!(
        create_auction(
            &db,
            &ItemTemplates::default(),
            &mut state,
            seller,
            crate::npc_interaction::AUCTION_HOUSE_ALLIANCE,
            &CreateAuction {
                item_guid: item.item_guid,
                stack_count: 2,
                min_bid: 1_001,
                buyout_price: Some(2_000),
                duration: AuctionDuration::Short,
            },
        )
        .success
    );
    let auction_id = *state.auctions.keys().next().unwrap();
    if with_bid {
        assert!(place_bid(&db, &mut state, buyer, auction_id, 1_001).success);
        assert_eq!(
            persistence::load_inventory(&db, buyer).gold,
            buyer_gold - 1_001
        );
    }
    let auction = state.auctions[&auction_id].clone();
    let mut app = App::new();
    app.insert_resource(GameDatabase { db });
    app.insert_resource(state);
    app.insert_resource(ServerClock {
        fixed_unix: Some(auction.expires_at_unix_secs as i64 - 1),
        offset_secs: 0,
    });
    app.add_systems(Update, super::super::handlers::expire_auctions);
    app.update();
    assert_eq!(
        app.world().resource::<AuctionHouseState>().auctions[&auction_id],
        auction
    );
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!(persistence::load_auctions(db), vec![auction.clone()]);
    assert!(mail_store::load_mailbox(db, seller).is_empty());
    assert!(mail_store::load_mailbox(db, buyer).is_empty());

    advance_clock_through_admin(&mut app);
    app.update();
    app.update();
    assert!(
        app.world()
            .resource::<AuctionHouseState>()
            .auctions
            .is_empty(),
        "injected deadline must expire the auction without waiting for wall time"
    );
    drop(app);
    let db = redb::Database::open(&path).unwrap();
    assert!(persistence::load_auctions(&db).is_empty());
    let seller_mail = mail_store::load_mailbox(&db, seller);
    assert_eq!(
        seller_mail.len(),
        1,
        "repeated sweep must not duplicate mail"
    );
    assert!(matches!(seller_mail[0].sender, MailSender::AuctionHouse));
    if with_bid {
        assert_eq!(seller_mail[0].subject, "Auction sold");
        assert_eq!(seller_mail[0].money, 951 + auction.deposit);
        assert!(seller_mail[0].items.is_empty());
        let buyer_mail = mail_store::load_mailbox(&db, buyer);
        assert_eq!(buyer_mail.len(), 1);
        assert!(matches!(buyer_mail[0].sender, MailSender::AuctionHouse));
        assert_eq!(buyer_mail[0].subject, "Auction won");
        assert_eq!(buyer_mail[0].money, 0);
        assert_eq!(buyer_mail[0].items.len(), 1);
        assert_eq!(buyer_mail[0].items[0].item, item);
        assert_eq!(
            persistence::load_inventory(&db, buyer).gold,
            buyer_gold - 1_001
        );
    } else {
        assert_eq!(seller_mail[0].subject, "Auction Expired");
        assert_eq!(seller_mail[0].money, auction.deposit);
        assert_eq!(seller_mail[0].items.len(), 1);
        assert_eq!(seller_mail[0].items[0].item, item);
        assert!(mail_store::load_mailbox(&db, buyer).is_empty());
        assert_eq!(persistence::load_inventory(&db, buyer).gold, buyer_gold);
    }
}
