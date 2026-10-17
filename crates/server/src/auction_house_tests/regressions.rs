use super::*;
use crate::auction_house::{AuctionHousePlugin, AuctionHouseState};
use crate::gold::GoldUpdateQueue;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::persistence::inventory_store::{ITEM_INSTANCES, ItemInstanceData};
use crate::persistence::{self, GameDatabase, InventoryData, InventoryItemData};
use lightyear::connection::host::HostClient;
use lightyear::prelude::{Connected, PeerId, RemoteId};
use shared::protocol::{
    AuctionChannel, AuctionDuration, AuctionOperationResponse, CreateAuction, PlaceBid,
    ProtocolPlugin,
};
use std::{collections::HashMap, time::Duration};

#[derive(Resource, Default)]
struct Responses(Vec<AuctionOperationResponse>);

fn collect_responses(
    mut receivers: Query<&mut MessageReceiver<AuctionOperationResponse>>,
    mut responses: ResMut<Responses>,
) {
    for mut receiver in &mut receivers {
        responses.0.extend(receiver.receive());
    }
}

fn request_app(bonding: u8, bound: bool) -> (App, Entity, u64, u64) {
    let db = persistence::temp_database();
    let (seller, _) =
        persistence::create_character_with_test_bags(&db, 1, "Seller", 1, 1).expect("seller");
    let item = InventoryItemData {
        item_guid: 92,
        item_id: 6948,
        name: "Hearthstone".into(),
        quality: 1,
        required_level: 0,
        stack_count: 1,
        vendor_sell_price: 100,
        unique_auction: false,
    };
    persistence::save_inventory(
        &db,
        seller,
        &InventoryData {
            gold: 1_000_000,
            items: vec![item],
        },
    );
    if bound {
        let txn = db.begin_write().unwrap();
        txn.open_table(ITEM_INSTANCES)
            .unwrap()
            .insert(
                92,
                bitcode::encode(&ItemInstanceData { soulbound: true }).as_slice(),
            )
            .unwrap();
        txn.commit().unwrap();
    }
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.insert_resource(GameDatabase::new(db));
    app.insert_resource(ItemTemplates {
        retail: HashMap::from([(
            6948,
            ItemTemplate {
                bonding,
                ..Default::default()
            },
        )]),
        ..Default::default()
    });
    app.init_resource::<AuctionHouseState>();
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins(AuctionHousePlugin);
    app.init_resource::<Responses>();
    app.add_systems(First, collect_responses);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            crate::auth::InWorld {
                character_id: seller,
            },
            ActiveInteraction {
                npc: Entity::PLACEHOLDER,
                role: Some(NpcRole::AuctionHouse),
                auction_house: Some(1),
                gossip_actions: Vec::new(),
            },
            MessageSender::<CreateAuction>::default(),
            MessageReceiver::<CreateAuction>::default(),
            MessageSender::<PlaceBid>::default(),
            MessageReceiver::<PlaceBid>::default(),
            MessageSender::<AuctionOperationResponse>::default(),
            MessageReceiver::<AuctionOperationResponse>::default(),
        ))
        .id();
    (app, connection, seller, 92)
}

fn post(app: &mut App, connection: Entity, guid: u64) -> AuctionOperationResponse {
    app.world_mut()
        .entity_mut(connection)
        .get_mut::<MessageSender<CreateAuction>>()
        .unwrap()
        .send::<AuctionChannel>(CreateAuction {
            item_guid: guid,
            stack_count: 1,
            min_bid: 100,
            buyout_price: None,
            duration: AuctionDuration::Short,
        });
    app.update();
    app.update();
    app.update();
    app.world_mut()
        .resource_mut::<Responses>()
        .0
        .pop()
        .expect("auction response")
}

#[test]
fn ahserver_bound_pickup_and_quest_requests_leave_inventory_untouched() {
    for bonding in [1, 4] {
        assert_bound_rejected(bonding, false);
    }
}

#[test]
fn ahserver_bound_equipped_and_used_instances_are_rejected() {
    for bonding in [2, 3] {
        assert_bound_rejected(bonding, true);
    }
}

fn assert_bound_rejected(bonding: u8, bound: bool) {
    let (mut app, connection, seller, guid) = request_app(bonding, bound);
    let before = persistence::load_inventory(&app.world().resource::<GameDatabase>().db, seller);
    let response = post(&mut app, connection, guid);
    assert!(
        !response.success,
        "bonding={bonding}, instance_bound={bound}: {response:?}"
    );
    assert_eq!(response.message, "cannot auction a soulbound item");
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!(persistence::load_inventory(db, seller), before);
    assert!(persistence::load_auctions(db).is_empty());
    assert!(app.world().resource::<GoldUpdateQueue>().is_empty());
    assert!(
        app.world()
            .resource::<AuctionHouseState>()
            .auctions
            .is_empty()
    );
}

#[test]
fn ahserver_bound_unbound_equip_item_remains_auctionable() {
    let (mut app, connection, seller, guid) = request_app(2, false);
    assert!(post(&mut app, connection, guid).success);
    let db = &app.world().resource::<GameDatabase>().db;
    assert!(persistence::load_inventory(db, seller).items.is_empty());
    assert_eq!(persistence::load_auctions(db).len(), 1);
}

#[test]
fn ahserver_money_outbid_200_copper_preserves_refund_without_invented_body() {
    assert_outbid_money(200);
}

#[test]
fn ahserver_money_outbid_mixed_and_u64_amounts_keep_every_coin() {
    for amount in [12_345, 4_294_967_395] {
        assert_outbid_money(amount);
    }
}

fn assert_outbid_money(amount: u64) {
    let (mut app, seller_connection, _, guid) = request_app(0, false);
    assert!(post(&mut app, seller_connection, guid).success);
    let auction_id = *app
        .world()
        .resource::<AuctionHouseState>()
        .auctions
        .keys()
        .next()
        .unwrap();
    let next_bid = amount + amount.div_ceil(20);
    let (a, a_id) = bidder_connection(&mut app, 2, "BidderA", amount + 998_800);
    let (b, _) = bidder_connection(&mut app, 3, "BidderB", next_bid + 998_800);
    assert!(bid(&mut app, a, auction_id, amount).success);
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!(persistence::load_inventory(db, a_id).gold, 998_800);
    assert!(bid(&mut app, b, auction_id, next_bid).success);
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!(persistence::load_inventory(db, a_id).gold, 998_800 + amount);
    let mail = persistence::mail_store::load_mailbox(db, a_id);
    assert_eq!(mail.len(), 1);
    assert_eq!(mail[0].subject, "AUCTION_OUTBID_MAIL_SUBJECT:Hearthstone");
    assert_eq!(
        mail[0].money, 0,
        "refund was already credited, not a mail attachment"
    );
    assert_eq!(mail[0].body, "");
}

fn bidder_connection(app: &mut App, account: u64, name: &str, copper: u64) -> (Entity, u64) {
    let db = &app.world().resource::<GameDatabase>().db;
    let (id, _) = persistence::create_character_with_test_bags(db, account, name, 1, 1).unwrap();
    persistence::save_inventory(
        db,
        id,
        &InventoryData {
            gold: copper,
            items: Vec::new(),
        },
    );
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account)),
            Connected,
            HostClient { buffer: Vec::new() },
            crate::auth::InWorld { character_id: id },
            ActiveInteraction {
                npc: Entity::PLACEHOLDER,
                role: Some(NpcRole::AuctionHouse),
                auction_house: Some(1),
                gossip_actions: Vec::new(),
            },
            MessageSender::<PlaceBid>::default(),
            MessageReceiver::<PlaceBid>::default(),
            MessageSender::<AuctionOperationResponse>::default(),
            MessageReceiver::<AuctionOperationResponse>::default(),
        ))
        .id();
    (connection, id)
}

fn bid(
    app: &mut App,
    connection: Entity,
    auction_id: u64,
    amount: u64,
) -> AuctionOperationResponse {
    app.world_mut()
        .entity_mut(connection)
        .get_mut::<MessageSender<PlaceBid>>()
        .unwrap()
        .send::<AuctionChannel>(PlaceBid { auction_id, amount });
    for _ in 0..3 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<Responses>()
        .0
        .pop()
        .expect("bid response")
}
