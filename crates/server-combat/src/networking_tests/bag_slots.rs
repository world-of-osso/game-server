//! Bag slots over real UDP: equipping a Small Brown Pouch (item 4496, 6 slots) from
//! the backpack resends the owner's `InventorySnapshot` with container 1 at 6 slots.

use super::sheet_stats::{advance, network_client, network_server};
use super::*;
use crate::auth::{Authenticated, InWorld};
use shared::components::{EquipmentAppearance, Player};
use shared::protocol::{
    EquipmentSlot, InventoryChannel, InventoryDelta, InventorySnapshot, ItemLocation, SwapItem,
};
use std::net::UdpSocket;

const SMALL_BROWN_POUCH: u32 = 4496;

/// Inventory messages the client received, read every frame like the game client.
#[derive(Resource, Default)]
struct Received {
    snapshots: Vec<InventorySnapshot>,
    deltas: Vec<InventoryDelta>,
}

fn collect_inventory_messages(
    mut received: ResMut<Received>,
    mut snapshots: Query<&mut MessageReceiver<InventorySnapshot>>,
    mut deltas: Query<&mut MessageReceiver<InventoryDelta>>,
) {
    for mut receiver in &mut snapshots {
        received.snapshots.extend(receiver.receive());
    }
    for mut receiver in &mut deltas {
        received.deltas.extend(receiver.receive());
    }
}

/// Sizes of containers 0-5 in every snapshot received since the last call.
fn received_sizes(client: &mut App) -> Vec<Vec<(u8, u8)>> {
    std::mem::take(&mut client.world_mut().resource_mut::<Received>().snapshots)
        .iter()
        .map(|snapshot| {
            snapshot
                .bags
                .iter()
                .map(|bag| (bag.bag, bag.size))
                .collect()
        })
        .collect()
}

/// Put a pouch in the character's bags; it lands in the first free backpack slot (3).
fn grant_pouch(db: &redb::Database, character_id: u64) -> u64 {
    let guid = persistence::next_id(db, "item_instance");
    let mut inventory = persistence::load_inventory(db, character_id);
    inventory.items.push(persistence::InventoryItemData {
        item_guid: guid,
        item_id: SMALL_BROWN_POUCH,
        name: "Small Brown Pouch".into(),
        quality: 1,
        required_level: 0,
        stack_count: 1,
        vendor_sell_price: 1,
        unique_auction: false,
    });
    persistence::save_inventory(db, character_id, &inventory);
    guid
}

#[test]
fn equipping_a_pouch_sends_the_client_a_six_slot_bag_one() {
    let world_db = crate::test_data::world_db_path();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let address = socket.local_addr().unwrap();
    drop(socket);
    let mut server = network_server(address, &world_db);
    server.add_plugins(crate::inventory::InventoryPlugin);
    server.init_resource::<crate::gold::GoldUpdateQueue>();
    let db = persistence::temp_database();
    let (character_id, _) =
        persistence::create_character_with_test_bags(&db, 941, "Alice", 1, 1).unwrap();
    let pouch = grant_pouch(&db, character_id);
    server.insert_resource(GameDatabase::new(db));
    let (mut client, connection) = network_client(address, 941);
    client.init_resource::<Received>();
    client.add_systems(Update, collect_inventory_messages);
    let mut clients = [client];
    for app in std::iter::once(&mut server).chain(clients.iter_mut()) {
        app.finish();
        app.cleanup();
    }
    advance(&mut server, &mut clients, 250);
    assert!(clients[0].world().get::<Connected>(connection).is_some());

    let player = server
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            crate::maps::InMap::continent(0),
            Player {
                name: "Alice".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
            EquipmentAppearance::default(),
        ))
        .id();
    let link = server
        .world_mut()
        .query_filtered::<Entity, With<RemoteId>>()
        .single(server.world())
        .unwrap();
    server.world_mut().entity_mut(link).insert((
        PlayerLink(player),
        Authenticated { account_id: 941 },
        InWorld { character_id },
    ));
    advance(&mut server, &mut clients, 100);
    assert_eq!(
        received_sizes(&mut clients[0]),
        vec![vec![(0, 16), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]]
    );

    clients[0]
        .world_mut()
        .get_mut::<MessageSender<SwapItem>>(connection)
        .unwrap()
        .send::<InventoryChannel>(SwapItem {
            from: ItemLocation::Bag { bag: 0, slot: 3 },
            to: ItemLocation::Equipment(EquipmentSlot::Bag1),
        });
    advance(&mut server, &mut clients, 100);

    assert_eq!(
        received_sizes(&mut clients[0]),
        vec![vec![(0, 16), (1, 6), (2, 0), (3, 0), (4, 0), (5, 0)]]
    );
    let equipped = clients[0]
        .world()
        .resource::<Received>()
        .deltas
        .iter()
        .flat_map(|delta| &delta.changes)
        .find(|change| change.location == ItemLocation::Equipment(EquipmentSlot::Bag1))
        .and_then(|change| change.item.as_ref())
        .map(|item| (item.item_guid, item.item_id));
    assert_eq!(equipped, Some((pouch, SMALL_BROWN_POUCH)));
}
