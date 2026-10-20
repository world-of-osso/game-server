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
/// Retail inventory type 13 (one-hand) dagger.
const WORN_DAGGER: u32 = 2092;
/// Retail inventory type 22 (off-hand weapon).
const TRIBAL_GUARDIAN: u32 = 12939;
/// Native SPELL_EFFECT_DUAL_WIELD 40.
const DUAL_WIELD: u32 = 674;
const VERIGANS_FIST: u32 = 6953;
const SMALL_BROWN_POUCH: u32 = 4496;
const BLUE_LEATHER_BAG: u32 = 856;
const REAGENT_BAG: u32 = 194019;
/// `ITEM_FLAG2_USED_IN_A_TRADESKILL` in `Flags_1` of linen, peacebloom and copper ore.
const TRADESKILL_FLAGS2: i64 = -2_147_475_456;

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
    let reagent = |mut template: ItemTemplate| {
        template.scaling.flags2 = TRADESKILL_FLAGS2;
        template
    };
    // Class 1 (container), `ContainerSlots`; subclass 11 is a reagent bag.
    let container = |container_slots, subclass_id| ItemTemplate {
        container_slots,
        scaling: crate::item_stats::ItemScaling {
            class_id: 1,
            subclass_id,
            ..Default::default()
        },
        ..template(18, 1, 0, 0, -1)
    };
    ItemTemplates {
        retail: [
            (WORN_SHORTSWORD, template(21, 1, 2, 1, -1)),
            (WORN_DAGGER, template(13, 1, 2, 1, -1)),
            (TRIBAL_GUARDIAN, template(22, 1, 1, 0, -1)),
            (LINEN, reagent(template(0, 1000, 0, 0, -1))),
            (PEACEBLOOM, reagent(template(0, 1000, 0, 0, -1))),
            (COPPER_ORE, reagent(template(0, 1000, 0, 0, -1))),
            (VERIGANS_FIST, template(17, 1, 1, 0, 2)),
            (SMALL_BROWN_POUCH, container(6, 0)),
            (BLUE_LEATHER_BAG, container(8, 0)),
            (REAGENT_BAG, container(32, 11)),
        ]
        .into_iter()
        .collect(),
        forever: Default::default(),
        loot_need: Default::default(),
        retail_learning_spells: Default::default(),
    }
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
    app.insert_resource(GameDatabase::new(db));
    app.init_resource::<GoldUpdateQueue>();
    app.insert_resource(test_templates());
    // This inventory fixture contains no toy definitions; item use still needs
    // the catalog owned by ToysPlugin in the production app.
    app.init_resource::<crate::toy_catalog::ToyCatalog>();
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
        definition_source: shared::item_data::ItemDefinitionSource::Retail,
        count,
        durability: None,
        soulbound: false,
        crafting_origin: None,
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

/// Real ItemSparse identities; the production loader supplies all flags and stats.
fn add_nonvisual_templates(app: &mut App) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    let catalog = crate::item_templates::load_item_templates(&path).expect("Retail catalog");
    let mut templates = app.world_mut().resource_mut::<ItemTemplates>();
    for id in [
        1116, 1319, 1490, 2820, 4112, 12344, 35748, 35749, 18472, 2504, 4549,
    ] {
        templates
            .retail
            .insert(id, *catalog.retail.get(&id).expect("authentic item"));
    }
}

fn equipped_location(slot: EquipmentSlot) -> ItemLocation {
    ItemLocation::Equipment(slot)
}

#[test]
fn nonvisual_equipment_ring_pair_snapshot_and_relog() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let database = Database::create(file.path()).unwrap();
    persistence::create_tables(&database);
    let mut app = setup_app(database);
    add_nonvisual_templates(&mut app);
    let client = spawn_client(&mut app, 91, "Ringwearer");
    let first = grant_bag_item(&app, client.character_id, 1116, "Ring of Pure Silver");
    let second = grant_bag_item(&app, client.character_id, 1319, "Ring of Iron Will");
    enter_world(&mut app, &client);
    for slot in [3, 4] {
        request(&mut app, &client, EquipItem { from: bag(slot) });
    }
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let model = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap();
    let snapshot = model.equipment_snapshot();
    assert_eq!(
        snapshot
            .items
            .iter()
            .map(|entry| (entry.slot, entry.item.item_guid))
            .collect::<Vec<_>>(),
        vec![
            (EquipmentSlot::Finger1, first),
            (EquipmentSlot::Finger2, second)
        ]
    );
    assert_eq!(
        app.world()
            .entity(client.player)
            .get::<EquippedItems>()
            .unwrap()
            .items
            .len(),
        2
    );
    app.world_mut()
        .entity_mut(client.connection)
        .remove::<InWorld>();
    app.update();
    let _ = received::<EquipmentSnapshot>(&mut app, &client);
    enter_world(&mut app, &client);
    assert_eq!(
        received::<EquipmentSnapshot>(&mut app, &client),
        vec![snapshot.clone()]
    );
    let character_id = client.character_id;
    drop(app);
    let database = Database::open(file.path()).unwrap();
    persistence::create_tables(&database);
    let mut reopened = setup_app(database);
    add_nonvisual_templates(&mut reopened);
    let model = load_model(
        db(&reopened),
        reopened.world().resource::<ItemTemplates>(),
        character_id,
    )
    .unwrap();
    assert_eq!(model.equipment_snapshot(), snapshot);
}

#[test]
fn nonvisual_equipment_trinket_swap_and_unequip_preserve_guids() {
    let mut app = setup_app(persistence::temp_database());
    add_nonvisual_templates(&mut app);
    let client = spawn_client(&mut app, 92, "Trinketer");
    let first = grant_bag_item(&app, client.character_id, 1490, "Guardian Talisman");
    let second = grant_bag_item(&app, client.character_id, 2820, "Nifty Stopwatch");
    enter_world(&mut app, &client);
    for (from, to) in [
        (bag(3), equipped_location(EquipmentSlot::Trinket1)),
        (bag(4), equipped_location(EquipmentSlot::Trinket2)),
        (
            equipped_location(EquipmentSlot::Trinket1),
            equipped_location(EquipmentSlot::Trinket2),
        ),
    ] {
        request(&mut app, &client, SwapItem { from, to });
    }
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let model = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap();
    let view = model.view();
    assert_eq!(
        view[&equipped_location(EquipmentSlot::Trinket1)].item_guid,
        second
    );
    assert_eq!(
        view[&equipped_location(EquipmentSlot::Trinket2)].item_guid,
        first
    );
    request(
        &mut app,
        &client,
        EquipItem {
            from: equipped_location(EquipmentSlot::Trinket2),
        },
    );
    let model = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap();
    assert!(
        !model
            .view()
            .contains_key(&equipped_location(EquipmentSlot::Trinket2))
    );
    assert!(
        model
            .inventory_snapshot()
            .bags
            .iter()
            .flat_map(|bag| &bag.items)
            .any(|entry| entry.item.item_guid == first)
    );
}

#[test]
fn nonvisual_equipment_unique_ring_rejects_second_without_mutation() {
    let mut app = setup_app(persistence::temp_database());
    add_nonvisual_templates(&mut app);
    let client = spawn_client(&mut app, 93, "Uniquewearer");
    grant_bag_item(&app, client.character_id, 12344, "Seal of Ascension");
    grant_bag_item(&app, client.character_id, 12344, "Seal of Ascension");
    enter_world(&mut app, &client);
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: equipped_location(EquipmentSlot::Finger1),
        },
    );
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let before = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap()
    .view();
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(4),
            to: equipped_location(EquipmentSlot::Finger2),
        },
    );
    let errors = received::<InventoryError>(&mut app, &client);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, InventoryErrorReason::ItemUniqueEquippable);
    assert_eq!(
        load_model(
            db(&app),
            app.world().resource::<ItemTemplates>(),
            client.character_id
        )
        .unwrap()
        .view(),
        before
    );
}

#[test]
fn nonvisual_equipment_distinct_alchemist_stones_share_db2_equip_limit() {
    let mut app = setup_app(persistence::temp_database());
    add_nonvisual_templates(&mut app);
    let client = spawn_client(&mut app, 96, "Alchemist");
    grant_bag_item(
        &app,
        client.character_id,
        35748,
        "Guardian's Alchemist Stone",
    );
    grant_bag_item(
        &app,
        client.character_id,
        35749,
        "Sorcerer's Alchemist Stone",
    );
    enter_world(&mut app, &client);
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: equipped_location(EquipmentSlot::Trinket1),
        },
    );
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let before = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap()
    .view();
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(4),
            to: equipped_location(EquipmentSlot::Trinket2),
        },
    );
    assert_eq!(received::<InventoryError>(&mut app, &client).len(), 1);
    assert_eq!(
        load_model(
            db(&app),
            app.world().resource::<ItemTemplates>(),
            client.character_id
        )
        .unwrap()
        .view(),
        before
    );
    // Replacing the first stone does not count the displaced slot.
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(4),
            to: equipped_location(EquipmentSlot::Trinket1),
        },
    );
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let view = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap()
    .view();
    assert_eq!(
        view[&equipped_location(EquipmentSlot::Trinket1)].item_id,
        35749
    );
    assert_eq!(view[&bag(4)].item_id, 35748);
}

#[test]
fn nonvisual_equipment_keeps_class_checks_and_mainline_ranged_mapping() {
    let mut app = setup_app(persistence::temp_database());
    add_nonvisual_templates(&mut app);
    let client = spawn_client(&mut app, 97, "Classwearer");
    grant_bag_item(
        &app,
        client.character_id,
        18472,
        "Royal Seal of Eldre'Thalas",
    );
    grant_bag_item(&app, client.character_id, 2504, "Worn Shortbow");
    enter_world(&mut app, &client);
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: equipped_location(EquipmentSlot::Trinket1),
        },
    );
    let errors = received::<InventoryError>(&mut app, &client);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].reason, InventoryErrorReason::CantEquipEver);
    request(&mut app, &client, EquipItem { from: bag(4) });
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let model = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap();
    assert_eq!(
        model.view()[&equipped_location(EquipmentSlot::MainHand)].item_id,
        2504
    );
    assert!(
        !model
            .view()
            .contains_key(&equipped_location(EquipmentSlot::Ranged))
    );
    assert_eq!(model.view()[&bag(3)].item_id, 18472);
}

#[test]
fn nonvisual_equipment_auto_replaces_lower_level_paired_ring() {
    let mut app = setup_app(persistence::temp_database());
    add_nonvisual_templates(&mut app);
    let client = spawn_client(&mut app, 98, "Pairwearer");
    grant_bag_item(&app, client.character_id, 1116, "Ring of Pure Silver");
    grant_bag_item(&app, client.character_id, 1319, "Ring of Iron Will");
    grant_bag_item(&app, client.character_id, 4549, "Seafire Band");
    enter_world(&mut app, &client);
    for from in [bag(3), bag(4), bag(5)] {
        request(&mut app, &client, EquipItem { from });
    }
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    let view = load_model(
        db(&app),
        app.world().resource::<ItemTemplates>(),
        client.character_id,
    )
    .unwrap()
    .view();
    assert_eq!(
        view[&equipped_location(EquipmentSlot::Finger1)].item_id,
        1116
    );
    assert_eq!(
        view[&equipped_location(EquipmentSlot::Finger2)].item_id,
        4549
    );
    assert_eq!(view[&bag(5)].item_id, 1319);
}

#[test]
fn nonvisual_equipment_migration_preserves_old_character_and_visual_items() {
    let database = persistence::temp_database();
    let mut app = setup_app(database);
    let client = spawn_client(&mut app, 94, "Oldwearer");
    grant_bag_weapon(&app, client.character_id);
    enter_world(&mut app, &client);
    request(&mut app, &client, EquipItem { from: bag(3) });
    let before =
        persistence::inventory_store::load_inventory_record(db(&app), client.character_id).unwrap();
    let transaction = db(&app).begin_write().unwrap();
    transaction
        .delete_table(persistence::inventory_store::NONVISUAL_EQUIPMENT)
        .unwrap();
    transaction.commit().unwrap();
    persistence::create_tables(db(&app));
    let after =
        persistence::inventory_store::load_inventory_record(db(&app), client.character_id).unwrap();
    assert_eq!(after, before);
    assert_eq!(after.equipped.len(), 1);
    assert!(after.nonvisual_equipped.is_empty());
}

#[test]
fn nonvisual_equipment_neck_and_ring_apply_stats_and_existing_aura_modifiers() {
    use bevy::ecs::system::RunSystemOnce;
    use shared::components::{CombatRatings, DerivedStats, Health, Level, UnitStats};
    let mut app = setup_app(persistence::temp_database());
    add_nonvisual_templates(&mut app);
    let client = spawn_client(&mut app, 95, "Statwearer");
    let path = crate::test_data::world_db_path();
    app.insert_resource(crate::item_stats::load_item_scaling_tables(&path).unwrap());
    app.world_mut().entity_mut(client.player).insert((
        Level(10),
        UnitStats::default(),
        CombatRatings::default(),
        DerivedStats::default(),
        Health {
            current: 100.0,
            max: 100.0,
        },
        crate::stats::StatModifiers {
            primary_flat: UnitStats {
                stamina: 10.0,
                ..Default::default()
            },
            ..Default::default()
        },
    ));
    grant_bag_item(&app, client.character_id, 1116, "Ring of Pure Silver");
    grant_bag_item(&app, client.character_id, 4112, "Choker of the High Shaman");
    enter_world(&mut app, &client);
    app.world_mut()
        .run_system_once(crate::stats::update_character_stats)
        .unwrap();
    let before = *app.world().get::<UnitStats>(client.player).unwrap();
    let health_before = app.world().get::<Health>(client.player).unwrap().max;
    for from in [bag(3), bag(4)] {
        request(&mut app, &client, EquipItem { from });
    }
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    app.world_mut()
        .run_system_once(crate::stats::update_character_stats)
        .unwrap();
    let after = *app.world().get::<UnitStats>(client.player).unwrap();
    assert!(after.stamina > before.stamina);
    assert!(after.intellect > before.intellect);
    assert!(app.world().get::<Health>(client.player).unwrap().max > health_before);
    request(
        &mut app,
        &client,
        EquipItem {
            from: equipped_location(EquipmentSlot::Finger1),
        },
    );
    request(
        &mut app,
        &client,
        EquipItem {
            from: equipped_location(EquipmentSlot::Neck),
        },
    );
    app.world_mut()
        .run_system_once(crate::stats::update_character_stats)
        .unwrap();
    assert_eq!(
        *app.world().get::<UnitStats>(client.player).unwrap(),
        before
    );
    assert_eq!(
        app.world().get::<Health>(client.player).unwrap().max,
        health_before
    );
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
        vec![(0, 16), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]
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
fn achievements_criteria_equipped_retail_item_is_actual_wear_not_bag_ownership_or_rejected_move() {
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut app = setup_app_with(persistence::temp_database(), |app| {
        app.insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
        app.insert_resource(
            crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap(),
        );
        app.add_plugins(crate::achievement::AchievementPlugin);
    });
    let client = spawn_client(&mut app, 1, "Tabardproof");
    app.world_mut()
        .entity_mut(client.player)
        .insert(crate::achievement::PlayerAchievementState::default());
    grant_bag_item(&app, client.character_id, 20132, "Arathor Battle Tabard");
    enter_world(&mut app, &client);
    assert!(
        !persistence::load_achievements(db(&app), client.character_id)
            .criteria_progress
            .contains_key(&2335)
    );
    request(
        &mut app,
        &client,
        SwapItem {
            from: bag(3),
            to: ItemLocation::Equipment(EquipmentSlot::Head),
        },
    );
    assert!(!received::<InventoryError>(&mut app, &client).is_empty());
    app.update();
    assert!(
        !persistence::load_achievements(db(&app), client.character_id)
            .criteria_progress
            .contains_key(&2335)
    );
    request(&mut app, &client, EquipItem { from: bag(3) });
    assert!(received::<InventoryError>(&mut app, &client).is_empty());
    app.update();
    assert_eq!(
        persistence::load_achievements(db(&app), client.character_id)
            .criteria_progress
            .get(&2335),
        Some(&1)
    );
    request(&mut app, &client, EquipItem { from: bag(3) });
    app.update();
    assert_eq!(
        persistence::load_achievements(db(&app), client.character_id)
            .criteria_progress
            .get(&2335),
        Some(&1)
    );
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
    let path = crate::test_data::world_db_path();
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

#[path = "inventory_bag_tests.rs"]
mod bags;
#[path = "inventory_titans_grip_tests.rs"]
mod titans_grip;

/// The character knows `spells`, resolved through native DB2 rows.
fn learn_spells(app: &mut App, client: &Client, spells: &[u32]) {
    let data = crate::spell_cast::test_harness::world_db().expect("real world.db required");
    let rows = spells
        .iter()
        .map(|id| data.store.get(*id).expect("native spell").clone())
        .collect();
    app.insert_resource(crate::spell_info::SpellStore::fixture(rows));
    app.world_mut().entity_mut(client.player).insert(
        crate::class_progression::KnownSpells::fixture(spells.to_vec()),
    );
}

fn off_hand_item(app: &App, character_id: u64) -> Option<u32> {
    persistence::load_character(db(app), character_id)
        .unwrap()
        .equipment_appearance
        .entries
        .iter()
        .find(|entry| entry.slot == EquipmentVisualSlot::OffHand)
        .and_then(|entry| entry.item_id)
}

fn equip_off_hand(app: &mut App, client: &Client, from: ItemLocation) -> Vec<InventoryError> {
    request(
        app,
        client,
        SwapItem {
            from,
            to: ItemLocation::Equipment(EquipmentSlot::OffHand),
        },
    );
    received::<InventoryError>(app, client)
}

#[test]
fn dual_wield_spell_permits_off_hand_weapons() {
    for (item, name) in [
        (WORN_DAGGER, "Worn Dagger"),
        (TRIBAL_GUARDIAN, "Tribal Guardian"),
    ] {
        let mut app = setup_app(persistence::temp_database());
        let client = spawn_client(&mut app, 1, "Rogue");
        learn_spells(&mut app, &client, &[DUAL_WIELD]);
        grant_bag_item(&app, client.character_id, item, name);
        enter_world(&mut app, &client);
        assert_eq!(equip_off_hand(&mut app, &client, bag(3)), Vec::new());
        assert_eq!(off_hand_item(&app, client.character_id), Some(item));
    }
}

#[test]
fn off_hand_weapons_require_dual_wield() {
    for (item, name) in [
        (WORN_DAGGER, "Worn Dagger"),
        (TRIBAL_GUARDIAN, "Tribal Guardian"),
    ] {
        let mut app = setup_app(persistence::temp_database());
        let client = spawn_client(&mut app, 1, "Paladin");
        learn_spells(&mut app, &client, &[]);
        grant_bag_item(&app, client.character_id, item, name);
        enter_world(&mut app, &client);
        let errors = equip_off_hand(&mut app, &client, bag(3));
        assert_eq!(errors.len(), 1, "{item}");
        assert_eq!(errors[0].reason, InventoryErrorReason::CantDualWield);
        assert_eq!(off_hand_item(&app, client.character_id), None);
    }
}

#[test]
fn auto_equip_offers_the_off_hand_only_to_dual_wielders() {
    for (spells, off_hand) in [(&[DUAL_WIELD][..], Some(WORN_DAGGER)), (&[][..], None)] {
        let mut app = setup_app(persistence::temp_database());
        let client = spawn_client(&mut app, 1, "Wielder");
        learn_spells(&mut app, &client, spells);
        grant_bag_item(
            &app,
            client.character_id,
            WORN_SHORTSWORD,
            "Worn Shortsword",
        );
        grant_bag_item(&app, client.character_id, WORN_DAGGER, "Worn Dagger");
        enter_world(&mut app, &client);
        request(
            &mut app,
            &client,
            SwapItem {
                from: bag(3),
                to: ItemLocation::Equipment(EquipmentSlot::MainHand),
            },
        );
        request(&mut app, &client, EquipItem { from: bag(4) });
        assert!(received::<InventoryError>(&mut app, &client).is_empty());
        assert_eq!(off_hand_item(&app, client.character_id), off_hand);
    }
}
