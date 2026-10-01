use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Channel, Connected, Message, PeerId, RemoteId};
use shared::components::{CharacterAppearance, EquipmentVisualSlot, Position};
use shared::protocol::{
    AcceptTrade, BagSlotItem, ConfirmTrade, EquipmentSlot, InitiateTrade, InventorySlotChange,
    ItemLocation, ItemStack, SetTradeItem, TradeChannel, TradeStateUpdate,
};

use super::*;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::persistence::ClientId;

const LINEN: u32 = 2589;
const PEACEBLOOM: u32 = 2447;
const COPPER_ORE: u32 = 2770;
const WORN_SHORTSWORD: u32 = 25;
const VERIGANS_FIST: u32 = 6953;

/// `content_item` rows for the items these tests use (world.db, Retail 12.1).
fn test_templates() -> ItemTemplates {
    let template =
        |inventory_type, max_stack, bonding, required_level, allowable_class| ItemTemplate {
            inventory_type,
            max_stack,
            bonding,
            required_level,
            allowable_class,
            allowable_race: u64::MAX,
            ..ItemTemplate::default()
        };
    ItemTemplates(
        [
            (WORN_SHORTSWORD, template(21, 1, 2, 1, -1)),
            (LINEN, template(0, 1000, 0, 0, -1)),
            (PEACEBLOOM, template(0, 1000, 0, 0, -1)),
            (COPPER_ORE, template(0, 1000, 0, 0, -1)),
            (VERIGANS_FIST, template(17, 1, 1, 0, 2)),
        ]
        .into_iter()
        .collect(),
    )
}

fn bag(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

fn setup_app(db: Database) -> App {
    setup_app_with(db, |_| {})
}

/// `setup_app` plus the plugins `extra` adds before the app finishes.
fn setup_app_with(db: Database, extra: impl FnOnce(&mut App)) -> App {
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
    app.register_message::<EquipItem>();
    app.register_message::<SortBags>();
    app.register_message::<UseItem>();
    app.register_message::<InitiateTrade>();
    app.register_message::<AcceptTrade>();
    app.register_message::<SetTradeItem>();
    app.register_message::<ConfirmTrade>();
    app.register_message::<TradeStateUpdate>();
    app.insert_resource(GameDatabase { db });
    app.init_resource::<GoldUpdateQueue>();
    app.insert_resource(test_templates());
    app.add_plugins(InventoryPlugin);
    app.add_plugins(crate::trade::TradePlugin);
    extra(&mut app);
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
        persistence::create_character_with_test_bags(db, account_id, name, 1, 1)
            .expect("create character");
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
    add_message_pair::<EquipItem>(app, connection);
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
fn grant_bag_item(app: &App, character_id: u64, item_id: u32, name: &str) -> u64 {
    let db = db(app);
    let guid = persistence::next_id(db, "item_instance");
    let mut inventory = persistence::load_inventory(db, character_id);
    inventory.items.push(persistence::InventoryItemData {
        item_guid: guid,
        item_id,
        name: name.into(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 7,
        unique_auction: false,
    });
    persistence::save_inventory(db, character_id, &inventory);
    guid
}

fn grant_bag_weapon(app: &App, character_id: u64) -> u64 {
    grant_bag_item(app, character_id, WORN_SHORTSWORD, "Worn Shortsword")
}

#[test]
fn enter_world_sends_owner_snapshots_with_granted_items() {
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
        .unwrap()
        .clone();
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
    let equipped = equipped.item.as_ref().unwrap();
    assert_eq!(equipped.item_guid, sword);
    assert!(equipped.soulbound, "Worn Shortsword binds on equip");
    let persisted = persistence::load_character(db(&app), client.character_id).unwrap();
    assert_eq!(persisted.equipment_appearance, appearance);
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
fn using_item_without_an_on_use_spell_returns_clear_error() {
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
        Some("item 2589 has no on-use spell")
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
    // The traded linen tops up the taker's own stack (AzerothCore `StoreItem`).
    assert_eq!(change[0].location, bag(0));
    let item = change[0].item.as_ref().unwrap();
    assert_eq!((item.item_id, item.count), (LINEN, 25));
}

#[test]
fn swapping_a_split_stack_back_merges_into_one_stack() {
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
    received::<InventoryDelta>(&mut app, &client);

    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(5),
            to: bag(0),
        },
    );

    assert_eq!(
        received::<InventoryDelta>(&mut app, &client),
        vec![InventoryDelta {
            changes: vec![
                InventorySlotChange {
                    location: bag(0),
                    item: Some(stack(linen, LINEN, 20))
                },
                InventorySlotChange {
                    location: bag(5),
                    item: None
                },
            ]
        }]
    );
    let linen_stacks: Vec<u32> = persistence::load_inventory(db(&app), client.character_id)
        .items
        .iter()
        .filter(|item| item.item_id == LINEN)
        .map(|item| item.stack_count)
        .collect();
    assert_eq!(linen_stacks, vec![20]);
}

#[test]
fn class_restricted_weapon_is_rejected_and_bind_on_pickup_is_soulbound() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    let fist = grant_bag_item(&app, client.character_id, VERIGANS_FIST, "Verigan's Fist");
    enter_world(&mut app, &client);
    let snapshot = received::<InventorySnapshot>(&mut app, &client);
    let owned = snapshot[0].bags[0]
        .items
        .iter()
        .find(|entry| entry.item.item_guid == fist)
        .unwrap();
    assert!(owned.item.soulbound);

    request(&mut app, &client, EquipItem { from: bag(3) });

    let errors = received::<InventoryError>(&mut app, &client);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, InventoryErrorReason::CantEquipEver);
    assert!(
        persistence::load_character(db(&app), client.character_id)
            .unwrap()
            .equipment_appearance
            .entries
            .is_empty()
    );
}

#[test]
fn equipping_real_weapon_updates_character_stats_and_main_hand() {
    use crate::item_stats::{WeaponDamage, load_item_scaling_tables};
    use crate::stats::{CharacterStats, StatsPlugin};
    use shared::components::{CombatRatings, Health, Level, UnitStats, WeaponType};

    const NOTCHED_SHORTSWORD: u32 = 727;
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return;
    }
    let mut app = setup_app_with(persistence::temp_database(), |app| {
        app.add_plugins(StatsPlugin);
    });
    app.insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    app.insert_resource(load_item_scaling_tables(&path).unwrap());
    let client = spawn_client(&mut app, 1, "Bagger");
    let mut data = persistence::load_character(db(&app), client.character_id).unwrap();
    data.level = 5;
    persistence::save_character(db(&app), client.character_id, &data);
    app.world_mut().entity_mut(client.player).insert((
        Level(5),
        UnitStats::default(),
        CombatRatings::default(),
        shared::components::DerivedStats::default(),
        Health {
            current: 1.0,
            max: 1.0,
        },
    ));
    grant_bag_item(
        &app,
        client.character_id,
        NOTCHED_SHORTSWORD,
        "Notched Shortsword",
    );
    enter_world(&mut app, &client);
    app.update();
    let before = *app.world().get::<CharacterStats>(client.player).unwrap();
    assert_eq!(before.main_hand, WeaponDamage::UNARMED);

    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: ItemLocation::Equipment(EquipmentSlot::MainHand),
        },
    );
    app.update();

    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let after = *app.world().get::<CharacterStats>(client.player).unwrap();
    // Notched Shortsword (ilvl 5): +1 Agility, +1 Stamina, 1-2 damage at 2.60.
    // A warrior with no spec counts every gear primary stat.
    assert_eq!(after.primary.agility, before.primary.agility + 1.0);
    assert_eq!(after.primary.stamina, before.primary.stamina + 1.0);
    assert_eq!(after.max_health, before.max_health + 1.0);
    assert_eq!(
        after.main_hand,
        WeaponDamage {
            min: 1.0,
            max: 2.0,
            speed: 2.6,
            kind: WeaponType::OneHand,
        }
    );
}

#[test]
fn a_sword_dropped_on_the_head_slot_is_refused_with_retail_text() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    grant_bag_weapon(&app, client.character_id);
    enter_world(&mut app, &client);

    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: ItemLocation::Equipment(EquipmentSlot::Head),
        },
    );

    let errors = received::<InventoryError>(&mut app, &client);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].reason.message(),
        "That item does not go in that slot."
    );
    assert!(received::<InventoryDelta>(&mut app, &client).is_empty());
}

#[test]
fn an_equipped_sword_dropped_on_an_empty_bag_slot_is_unequipped_there() {
    let mut app = setup_app(persistence::temp_database());
    let client = spawn_client(&mut app, 1, "Bagger");
    let sword = grant_bag_weapon(&app, client.character_id);
    enter_world(&mut app, &client);
    let main_hand = ItemLocation::Equipment(EquipmentSlot::MainHand);
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: main_hand,
        },
    );
    received::<InventoryDelta>(&mut app, &client);

    request(
        &mut app,
        &client,
        SwapItem {
            from: main_hand,
            to: bag(9),
        },
    );

    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let deltas = received::<InventoryDelta>(&mut app, &client);
    let changes: Vec<_> = deltas[0]
        .changes
        .iter()
        .map(|change| (change.location, change.item.as_ref().map(|i| i.item_guid)))
        .collect();
    assert_eq!(changes, vec![(bag(9), Some(sword)), (main_hand, None)]);
    let appearance = app
        .world()
        .get::<EquipmentAppearance>(client.player)
        .unwrap();
    assert!(appearance.entries.is_empty());
}
