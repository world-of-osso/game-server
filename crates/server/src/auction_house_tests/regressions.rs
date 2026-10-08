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
    app.insert_resource(GameDatabase { db });
    app.insert_resource(ItemTemplates(HashMap::from([(
        6948,
        ItemTemplate {
            bonding,
            ..Default::default()
        },
    )])));
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
