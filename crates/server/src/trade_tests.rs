use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Connected, PeerId, RemoteId};
use shared::protocol::TradeChannel;
use shared::trade::{CompletedTrade, TRADE_SLOT_COUNT, TradeItem, TradeOffer};

use super::*;
use crate::trade::trade_support::build_trade_party_snapshot;

fn setup_trade_offer_app(db: redb::Database) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<ClearTradeItem>();
    app.register_message::<SetTradeMoney>();
    app.register_message::<TradeStateUpdate>();
    app.insert_resource(GameDatabase { db });
    app.insert_resource(ServerTradeState::default());
    app.add_systems(Update, process_clear_item_requests);
    app.add_systems(Update, process_set_money_requests);
    app.finish();
    app.cleanup();
    app
}

fn spawn_trade_player(app: &mut App, character_id: u64, name: &str) -> Entity {
    app.world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            ClientId(character_id),
        ))
        .id()
}

fn spawn_trade_connection(app: &mut App, peer_id: u64, player: Entity) -> Entity {
    app.world_mut()
        .spawn((
            RemoteId(PeerId::Local(peer_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: peer_id,
            },
            PlayerLink(player),
            MessageSender::<ClearTradeItem>::default(),
            MessageReceiver::<ClearTradeItem>::default(),
            MessageSender::<SetTradeMoney>::default(),
            MessageReceiver::<SetTradeMoney>::default(),
            MessageSender::<TradeStateUpdate>::default(),
            MessageReceiver::<TradeStateUpdate>::default(),
        ))
        .id()
}

fn send_set_money(app: &mut App, connection: Entity, copper: u32) {
    app.world_mut()
        .entity_mut(connection)
        .get_mut::<MessageSender<SetTradeMoney>>()
        .expect("connection should have a SetTradeMoney sender")
        .send::<TradeChannel>(SetTradeMoney { copper });
}

fn receive_trade_updates(app: &mut App, connection: Entity) -> Vec<TradeStateUpdate> {
    app.world_mut()
        .entity_mut(connection)
        .get_mut::<MessageReceiver<TradeStateUpdate>>()
        .expect("connection should have a TradeStateUpdate receiver")
        .receive()
        .collect()
}

fn trade_offer(item_guid: u64, count: u16, gold: u32) -> TradeOffer {
    let mut offer = TradeOffer::default();
    offer.slots[0] = Some(TradeItem { item_guid, count });
    offer.gold = gold;
    offer
}

#[test]
fn idle_trade_offer_receivers_do_not_resolve_players_or_emit_updates() {
    let _test_lock = TRADE_OFFER_TEST_LOCK.lock().unwrap();
    TRADE_OFFER_PLAYER_RESOLUTIONS.store(0, Ordering::SeqCst);

    let db = persistence::temp_database();
    let (first_id, _) = persistence::create_character_with_test_bags(&db, 1, "TraderOne", 1, 1)
        .expect("first character");
    let (second_id, _) = persistence::create_character_with_test_bags(&db, 2, "TraderTwo", 1, 1)
        .expect("second character");
    let mut app = setup_trade_offer_app(db);
    let first_player = spawn_trade_player(&mut app, first_id, "TraderOne");
    let second_player = spawn_trade_player(&mut app, second_id, "TraderTwo");
    let first_connection = spawn_trade_connection(&mut app, first_id, first_player);
    let second_connection = spawn_trade_connection(&mut app, second_id, second_player);

    app.update();
    app.update();
    app.update();

    assert_eq!(
        TRADE_OFFER_PLAYER_RESOLUTIONS.load(Ordering::SeqCst),
        0,
        "idle trade offer receivers must not resolve linked players"
    );
    assert!(receive_trade_updates(&mut app, first_connection).is_empty());
    assert!(receive_trade_updates(&mut app, second_connection).is_empty());
    assert!(
        app.world()
            .resource::<ServerTradeState>()
            .manager
            .is_empty()
    );
}

#[test]
fn queued_trade_money_requests_apply_in_order_and_broadcast_each_update() {
    let _test_lock = TRADE_OFFER_TEST_LOCK.lock().unwrap();
    TRADE_OFFER_PLAYER_RESOLUTIONS.store(0, Ordering::SeqCst);

    let db = persistence::temp_database();
    let (first_id, _) = persistence::create_character_with_test_bags(&db, 1, "TraderOne", 1, 1)
        .expect("first character");
    let (second_id, _) = persistence::create_character_with_test_bags(&db, 2, "TraderTwo", 1, 1)
        .expect("second character");
    let mut app = setup_trade_offer_app(db);
    let first_player = spawn_trade_player(&mut app, first_id, "TraderOne");
    let second_player = spawn_trade_player(&mut app, second_id, "TraderTwo");
    let first_connection = spawn_trade_connection(&mut app, first_id, first_player);
    let second_connection = spawn_trade_connection(&mut app, second_id, second_player);
    {
        let mut trade = app.world_mut().resource_mut::<ServerTradeState>();
        trade
            .manager
            .initiate_trade(first_id, second_id, (0.0, 0.0, 0.0), (0.0, 0.0, 0.0), 1)
            .expect("trade should start");
        trade
            .manager
            .accept_trade(second_id)
            .expect("trade should open");
    }

    app.update();
    let _ = receive_trade_updates(&mut app, first_connection);
    let _ = receive_trade_updates(&mut app, second_connection);
    TRADE_OFFER_PLAYER_RESOLUTIONS.store(0, Ordering::SeqCst);

    send_set_money(&mut app, first_connection, 100);
    send_set_money(&mut app, first_connection, 200);
    send_set_money(&mut app, first_connection, 300);
    app.update();
    app.update();

    assert_eq!(
        TRADE_OFFER_PLAYER_RESOLUTIONS.load(Ordering::SeqCst),
        1,
        "one queued request batch should resolve the local player once"
    );
    let first_updates = receive_trade_updates(&mut app, first_connection);
    let second_updates = receive_trade_updates(&mut app, second_connection);
    assert_eq!(first_updates.len(), 3);
    assert_eq!(second_updates.len(), 3);
    assert!(
        first_updates
            .iter()
            .all(|update| update.error.is_none() && update.message.is_none())
    );
    assert!(
        second_updates
            .iter()
            .all(|update| update.error.is_none() && update.message.is_none())
    );
    assert_eq!(
        app.world()
            .resource::<ServerTradeState>()
            .manager
            .get_session(first_id)
            .expect("trade should remain open")
            .initiator_offer
            .gold,
        300
    );
}

#[test]
fn missing_trade_player_errors_only_for_a_queued_request() {
    let _test_lock = TRADE_OFFER_TEST_LOCK.lock().unwrap();
    TRADE_OFFER_PLAYER_RESOLUTIONS.store(0, Ordering::SeqCst);

    let db = persistence::temp_database();
    let mut app = setup_trade_offer_app(db);
    let stale_player = app.world_mut().spawn_empty().id();
    assert!(app.world_mut().despawn(stale_player));
    let connection = spawn_trade_connection(&mut app, 1, stale_player);

    app.update();
    assert_eq!(TRADE_OFFER_PLAYER_RESOLUTIONS.load(Ordering::SeqCst), 0);
    assert!(receive_trade_updates(&mut app, connection).is_empty());

    send_set_money(&mut app, connection, 100);
    app.update();
    app.update();

    assert_eq!(TRADE_OFFER_PLAYER_RESOLUTIONS.load(Ordering::SeqCst), 1);
    assert_eq!(
        receive_trade_updates(&mut app, connection),
        vec![TradeStateUpdate {
            trade: None,
            message: None,
            error: Some("trade is unavailable: local player entity missing".into()),
        }]
    );
}

#[test]
fn execute_completed_trade_moves_items_and_gold() {
    let db = persistence::temp_database();
    let (initiator_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "TraderOne", 1, 1).expect("initiator");
    let (target_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "TraderTwo", 1, 1).expect("target");

    let initiator_item = persistence::load_inventory(&db, initiator_id).items[0].clone();
    let target_item = persistence::load_inventory(&db, target_id).items[1].clone();
    let initiator_gold_before = persistence::load_inventory(&db, initiator_id).gold;
    let target_gold_before = persistence::load_inventory(&db, target_id).gold;

    let trade = CompletedTrade {
        initiator: initiator_id,
        target: target_id,
        initiator_offer: trade_offer(initiator_item.item_guid, 1, 1_500),
        target_offer: trade_offer(target_item.item_guid, 2, 900),
    };

    let mut app = App::new();
    app.add_plugins(crate::gold::GoldPlugin);
    let initiator_player = app
        .world_mut()
        .spawn((
            Player {
                name: "TraderOne".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            ClientId(initiator_id),
            shared::components::Gold(initiator_gold_before),
        ))
        .id();
    let target_player = app
        .world_mut()
        .spawn((
            Player {
                name: "TraderTwo".into(),
                race: 1,
                class: 1,
                appearance: shared::components::CharacterAppearance::default(),
            },
            ClientId(target_id),
            shared::components::Gold(target_gold_before),
        ))
        .id();
    {
        let mut gold_updates = app
            .world_mut()
            .resource_mut::<crate::gold::GoldUpdateQueue>();
        execute_completed_trade(&db, &trade, &mut gold_updates).expect("trade should complete");
    }
    app.update();

    let initiator_inventory = persistence::load_inventory(&db, initiator_id);
    let target_inventory = persistence::load_inventory(&db, target_id);

    assert_eq!(
        initiator_inventory.gold,
        initiator_gold_before - 1_500 + 900
    );
    assert_eq!(target_inventory.gold, target_gold_before - 900 + 1_500);
    assert_eq!(
        app.world()
            .entity(initiator_player)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        initiator_inventory.gold
    );
    assert_eq!(
        app.world()
            .entity(target_player)
            .get::<shared::components::Gold>()
            .unwrap()
            .0,
        target_inventory.gold
    );
    assert!(
        initiator_inventory
            .items
            .iter()
            .any(|item| item.item_guid == target_item.item_guid && item.stack_count == 2)
    );
    assert!(
        target_inventory
            .items
            .iter()
            .any(|item| item.item_guid == initiator_item.item_guid && item.stack_count == 1)
    );
}

#[test]
fn execute_completed_trade_fails_when_item_is_missing() {
    let db = persistence::temp_database();
    let (initiator_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "TraderOne", 1, 1).expect("initiator");
    let (target_id, _) =
        persistence::create_character_with_test_bags(&db, 2, "TraderTwo", 1, 1).expect("target");

    let trade = CompletedTrade {
        initiator: initiator_id,
        target: target_id,
        initiator_offer: trade_offer(999_999, 1, 0),
        target_offer: TradeOffer::default(),
    };

    let mut gold_updates = crate::gold::GoldUpdateQueue::default();
    let error =
        execute_completed_trade(&db, &trade, &mut gold_updates).expect_err("trade should fail");
    assert!(error.contains("no longer in inventory"));
    assert!(gold_updates.is_empty());
}

#[test]
fn build_trade_party_snapshot_uses_inventory_metadata() {
    let db = persistence::temp_database();
    let (character_id, _) =
        persistence::create_character_with_test_bags(&db, 1, "TraderOne", 1, 1).expect("character");
    let inventory = persistence::load_inventory(&db, character_id);
    let item = inventory.items[0].clone();
    let snapshot = build_trade_party_snapshot(
        &db,
        character_id,
        "TraderOne".into(),
        true,
        &trade_offer(item.item_guid, 3, 777),
    );

    assert_eq!(snapshot.name, "TraderOne");
    assert!(snapshot.accepted);
    assert_eq!(snapshot.gold, 777);
    assert_eq!(snapshot.slots.len(), TRADE_SLOT_COUNT);
    let slot = snapshot.slots[0]
        .as_ref()
        .expect("slot should be populated");
    assert_eq!(slot.item_guid, item.item_guid);
    assert_eq!(slot.item_id, item.item_id);
    assert_eq!(slot.name, item.name);
    assert_eq!(slot.stack_count, 3);
}
