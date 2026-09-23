use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Channel, Connected, Message, PeerId, RemoteId};
use shared::components::{CharacterAppearance, EquipmentVisualSlot, Position};
use shared::protocol::{
    AcceptTrade, BagSlotItem, ConfirmTrade, EquipmentSlot, InitiateTrade, InventorySlotChange,
    ItemLocation, ItemStack, SetTradeItem, TradeChannel, TradeStateUpdate,
};

use super::*;
use crate::persistence::ClientId;
use crate::persistence::inventory_store::ItemInstanceData;

const LINEN: u32 = 2589;
const PEACEBLOOM: u32 = 2447;
const COPPER_ORE: u32 = 2770;
const WORN_SHORTSWORD: u32 = 25;

fn bag(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

fn setup_app(db: Database) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<InventorySnapshot>();
    app.register_message::<EquipmentSnapshot>();
    app.register_message::<InventoryDelta>();
    app.register_message::<InventoryError>();
    app.register_message::<SwapItem>();
    app.register_message::<SplitItem>();
    app.register_message::<DestroyItem>();
    app.register_message::<UseItem>();
    app.register_message::<EquipItem>();
    app.register_message::<SortBags>();
    app.register_message::<InitiateTrade>();
    app.register_message::<AcceptTrade>();
    app.register_message::<SetTradeItem>();
    app.register_message::<ConfirmTrade>();
    app.register_message::<TradeStateUpdate>();
    app.insert_resource(GameDatabase { db });
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins(InventoryPlugin);
    app.add_plugins(crate::trade::TradePlugin);
    app.finish();
    app.cleanup();
    app
}

struct Client {
    character_id: u64,
    player: Entity,
    connection: Entity,
}

fn spawn_client(app: &mut App, account_id: u64, name: &str) -> Client {
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) =
        persistence::create_character(db, account_id, name, 1, 1).expect("create character");
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            ClientId(character_id),
            EquipmentAppearance::default(),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id },
            PlayerLink(player),
        ))
        .id();
    add_message_pair::<InventorySnapshot>(app, connection);
    add_message_pair::<EquipmentSnapshot>(app, connection);
    add_message_pair::<InventoryDelta>(app, connection);
    add_message_pair::<InventoryError>(app, connection);
    add_message_pair::<SwapItem>(app, connection);
    add_message_pair::<SplitItem>(app, connection);
    add_message_pair::<DestroyItem>(app, connection);
    add_message_pair::<UseItem>(app, connection);
    add_message_pair::<InitiateTrade>(app, connection);
    add_message_pair::<AcceptTrade>(app, connection);
    add_message_pair::<SetTradeItem>(app, connection);
    add_message_pair::<ConfirmTrade>(app, connection);
    add_message_pair::<TradeStateUpdate>(app, connection);
    Client {
        character_id,
        player,
        connection,
    }
}

fn add_message_pair<M: Message>(app: &mut App, connection: Entity) {
    app.world_mut().entity_mut(connection).insert((
        MessageSender::<M>::default(),
        MessageReceiver::<M>::default(),
    ));
}

fn enter_world(app: &mut App, client: &Client) {
    app.world_mut()
        .entity_mut(client.connection)
        .insert(InWorld {
            character_id: client.character_id,
        });
    app.update();
}

fn send<M: Message, C: Channel>(app: &mut App, client: &Client, message: M) {
    app.world_mut()
        .entity_mut(client.connection)
        .get_mut::<MessageSender<M>>()
        .expect("connection sender")
        .send::<C>(message);
}

fn request<M: Message>(app: &mut App, client: &Client, message: M) {
    send::<M, InventoryChannel>(app, client, message);
    app.update();
    app.update();
}

fn received<M: Message>(app: &mut App, client: &Client) -> Vec<M> {
    app.world_mut()
        .entity_mut(client.connection)
        .get_mut::<MessageReceiver<M>>()
        .expect("connection receiver")
        .receive()
        .collect()
}

fn db(app: &App) -> &Database {
    &app.world().resource::<GameDatabase>().db
}

fn guid_of(app: &App, character_id: u64, item_id: u32) -> u64 {
    persistence::load_inventory(db(app), character_id)
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .expect("item in inventory")
        .item_guid
}

fn stack(guid: u64, item_id: u32, count: u32) -> ItemStack {
    ItemStack {
        item_guid: guid,
        item_id,
        count,
        durability: None,
        soulbound: false,
    }
}

/// Give the character a Worn Shortsword (inventory type 21, main hand) in its bags.
fn grant_bag_weapon(app: &App, character_id: u64) -> u64 {
    let db = db(app);
    let guid = persistence::next_id(db, "item_instance");
    let mut record = load_inventory_record(db, character_id).unwrap();
    record.inventory.items.push(persistence::InventoryItemData {
        item_guid: guid,
        item_id: WORN_SHORTSWORD,
        name: "Worn Shortsword".into(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 7,
        unique_auction: false,
    });
    record.instances.insert(
        guid,
        ItemInstanceData {
            inventory_type: 21,
            soulbound: false,
        },
    );
    save_inventory_record(db, character_id, &record);
    guid
}

#[test]
fn enter_world_sends_owner_snapshots_with_starter_items() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    let linen = guid_of(&app, client.character_id, LINEN);
    let peacebloom = guid_of(&app, client.character_id, PEACEBLOOM);
    let ore = guid_of(&app, client.character_id, COPPER_ORE);

    enter_world(&mut app, &client);

    let snapshots = received::<InventorySnapshot>(&mut app, &client);
    assert_eq!(snapshots.len(), 1);
    let bags = &snapshots[0].bags;
    assert_eq!(
        bags.iter().map(|b| (b.bag, b.size)).collect::<Vec<_>>(),
        vec![(0, 16), (1, 0), (2, 0), (3, 0), (4, 0)]
    );
    assert_eq!(
        bags[0].items,
        vec![
            BagSlotItem {
                slot: 0,
                item: stack(linen, LINEN, 20)
            },
            BagSlotItem {
                slot: 1,
                item: stack(peacebloom, PEACEBLOOM, 10)
            },
            BagSlotItem {
                slot: 2,
                item: stack(ore, COPPER_ORE, 10)
            },
        ]
    );
    let equipment = received::<EquipmentSnapshot>(&mut app, &client);
    assert_eq!(equipment, vec![EquipmentSnapshot { items: vec![] }]);
}

#[test]
fn swap_within_backpack_moves_item_and_emits_delta() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    enter_world(&mut app, &client);
    let linen = guid_of(&app, client.character_id, LINEN);

    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(0),
            to: bag(7),
        },
    );

    assert_eq!(
        received::<InventoryDelta>(&mut app, &client),
        vec![InventoryDelta {
            changes: vec![
                InventorySlotChange {
                    location: bag(0),
                    item: None
                },
                InventorySlotChange {
                    location: bag(7),
                    item: Some(stack(linen, LINEN, 20))
                },
            ]
        }]
    );
    let record = load_inventory_record(db(&app), client.character_id).unwrap();
    let placement = record
        .placements
        .iter()
        .find(|p| p.item_guid == linen)
        .unwrap();
    assert_eq!((placement.bag, placement.slot), (0, 7));
}

#[test]
fn equipping_weapon_into_main_hand_updates_equipment_appearance() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    let sword = grant_bag_weapon(&app, client.character_id);
    enter_world(&mut app, &client);

    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: ItemLocation::Equipment(EquipmentSlot::MainHand),
        },
    );

    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let appearance = app
        .world()
        .get::<EquipmentAppearance>(client.player)
        .unwrap();
    assert_eq!(appearance.entries.len(), 1);
    let entry = &appearance.entries[0];
    assert_eq!(entry.slot, EquipmentVisualSlot::MainHand);
    assert_eq!(entry.item_id, Some(WORN_SHORTSWORD));
    assert_eq!(entry.inventory_type, 21);
    let deltas = received::<InventoryDelta>(&mut app, &client);
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].changes[0].location, bag(3));
    assert_eq!(deltas[0].changes[0].item, None);
    let equipped = &deltas[0].changes[1];
    assert_eq!(
        equipped.location,
        ItemLocation::Equipment(EquipmentSlot::MainHand)
    );
    assert_eq!(equipped.item.as_ref().unwrap().item_guid, sword);
    let persisted = persistence::load_character(db(&app), client.character_id).unwrap();
    assert_eq!(persisted.equipment_appearance, *appearance);
}

#[test]
fn equipping_non_weapon_into_main_hand_is_rejected() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    enter_world(&mut app, &client);

    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(0),
            to: ItemLocation::Equipment(EquipmentSlot::MainHand),
        },
    );

    let errors = received::<InventoryError>(&mut app, &client);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, InventoryErrorReason::NotEquippable);
    assert!(received::<InventoryDelta>(&mut app, &client).is_empty());
}

#[test]
fn splitting_twenty_linen_leaves_fifteen_and_five() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    enter_world(&mut app, &client);
    let linen = guid_of(&app, client.character_id, LINEN);

    request(
        &mut app,
        &client,
        SplitItem {
            from: bag(0),
            to: bag(5),
            count: 5,
        },
    );

    let deltas = received::<InventoryDelta>(&mut app, &client);
    assert_eq!(deltas.len(), 1);
    let changes = &deltas[0].changes;
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].location, bag(0));
    assert_eq!(changes[0].item, Some(stack(linen, LINEN, 15)));
    assert_eq!(changes[1].location, bag(5));
    let split = changes[1].item.as_ref().unwrap();
    assert_eq!((split.item_id, split.count), (LINEN, 5));
    assert_ne!(split.item_guid, linen);
    let mut counts: Vec<u32> = persistence::load_inventory(db(&app), client.character_id)
        .items
        .iter()
        .filter(|item| item.item_id == LINEN)
        .map(|item| item.stack_count)
        .collect();
    counts.sort();
    assert_eq!(counts, vec![5, 15]);
}

#[test]
fn destroying_a_stack_persists_its_removal() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    enter_world(&mut app, &client);

    request(
        &mut app,
        &client,
        DestroyItem {
            location: bag(1),
            count: 0,
        },
    );

    let items = persistence::load_inventory(db(&app), client.character_id).items;
    assert!(items.iter().all(|item| item.item_id != PEACEBLOOM));
    assert_eq!(items.len(), 2);
    assert_eq!(
        received::<InventoryDelta>(&mut app, &client),
        vec![InventoryDelta {
            changes: vec![InventorySlotChange {
                location: bag(1),
                item: None
            }]
        }]
    );
}

#[test]
fn invalid_bag_slot_returns_inventory_error() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    enter_world(&mut app, &client);
    let before = persistence::load_inventory(db(&app), client.character_id);

    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(0),
            to: bag(16),
        },
    );

    let errors = received::<InventoryError>(&mut app, &client);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, InventoryErrorReason::InternalBagError);
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id),
        before
    );
}

#[test]
fn using_item_without_spell_data_returns_clear_error() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    enter_world(&mut app, &client);

    request(
        &mut app,
        &client,
        UseItem {
            location: bag(0),
            target: None,
        },
    );

    let errors = received::<InventoryError>(&mut app, &client);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, InventoryErrorReason::CantDoRightNow);
    assert_eq!(
        errors[0].detail.as_deref(),
        Some("item 2589 has no on-use spell data on the server")
    );
}

#[test]
fn completed_trade_emits_inventory_deltas_to_both_owners() {
    let mut app = setup_app(persistence::temp_database());
    let giver = spawn_client(&mut app, 1, "Giver");
    let taker = spawn_client(&mut app, 2, "Taker");
    enter_world(&mut app, &giver);
    enter_world(&mut app, &taker);
    let linen = guid_of(&app, giver.character_id, LINEN);

    send::<_, TradeChannel>(
        &mut app,
        &giver,
        InitiateTrade {
            target_name: "Taker".into(),
        },
    );
    app.update();
    send::<_, TradeChannel>(&mut app, &taker, AcceptTrade);
    app.update();
    send::<_, TradeChannel>(
        &mut app,
        &giver,
        SetTradeItem {
            slot: 0,
            item_guid: linen,
            stack_count: 5,
        },
    );
    app.update();
    send::<_, TradeChannel>(&mut app, &giver, ConfirmTrade);
    app.update();
    send::<_, TradeChannel>(&mut app, &taker, ConfirmTrade);
    app.update();
    app.update();

    assert_eq!(
        received::<InventoryDelta>(&mut app, &giver),
        vec![InventoryDelta {
            changes: vec![InventorySlotChange {
                location: bag(0),
                item: Some(stack(linen, LINEN, 15))
            }]
        }]
    );
    let taker_deltas = received::<InventoryDelta>(&mut app, &taker);
    assert_eq!(taker_deltas.len(), 1);
    let change = &taker_deltas[0].changes;
    assert_eq!(change.len(), 1);
    assert_eq!(change[0].location, bag(3));
    let item = change[0].item.as_ref().unwrap();
    assert_eq!((item.item_id, item.count), (LINEN, 5));
}
