//! Real-DB tests: item templates from data/world.db.

use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Channel, Connected, Message, PeerId, RemoteId};
use shared::components::{CharacterAppearance, EquipmentAppearance, Player, Position};
use shared::protocol::{
    EquipmentSlot, EquipmentSnapshot, InventoryChannel, InventoryDelta, InventoryError,
    InventorySnapshot, ItemLocation, SwapItem,
};

use super::*;
use crate::gold::GoldUpdateQueue;
use crate::inventory::InventoryPlugin;
use crate::item_templates::load_item_templates;
use crate::networking::PlayerLink;
use crate::persistence::appearance_store::load_account_appearances;
use crate::persistence::{self, ClientId};

/// Worn Shortsword (25): bind on equip, `ItemAppearance` 154.
const WORN_SHORTSWORD: u32 = 25;
const SHORTSWORD_APPEARANCE: u32 = 154;
/// Verigan's Fist (6953): bind on pickup, `ItemAppearance` 2288.
const VERIGANS_FIST: u32 = 6953;
const FIST_APPEARANCE: u32 = 2288;
const ACCOUNT: u64 = 7;

fn setup_app() -> Option<App> {
    let path = crate::test_data::world_db_path();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    install_database(&mut app);
    app.init_resource::<GoldUpdateQueue>();
    app.insert_resource(load_item_templates(&path).unwrap());
    app.add_plugins((InventoryPlugin, AppearanceCollectionPlugin));
    app.init_resource::<Inbox>().add_systems(First, collect);
    app.finish();
    app.cleanup();
    Some(app)
}

#[derive(Resource)]
struct TestDatabasePath(tempfile::TempDir);

pub(crate) fn install_database(app: &mut App) {
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("game.redb")).unwrap();
    persistence::create_tables(&db);
    app.insert_resource(GameDatabase::new(db));
    app.insert_resource(TestDatabasePath(directory));
}

pub(crate) fn assert_restart(mut app: App, account: u64, expected: &[(u32, bool)]) {
    let directory = app
        .world_mut()
        .remove_resource::<TestDatabasePath>()
        .unwrap();
    drop(app);
    let db = redb::Database::open(directory.0.path().join("game.redb")).unwrap();
    persistence::create_tables(&db);
    let collection = load_account_appearances(&db, account);
    for &(appearance, collected) in expected {
        assert_eq!(
            collection.has(appearance),
            collected,
            "appearance {appearance} after database/server reopen"
        );
    }
}

struct Client {
    character_id: u64,
    connection: Entity,
}

fn db(app: &App) -> &redb::Database {
    &app.world().resource::<GameDatabase>().db
}

/// A new Human Paladin on `ACCOUNT` holding `item_id` in its backpack.
fn spawn_client(app: &mut App, peer: u64, name: &str, item_id: u32) -> Client {
    let (character_id, _) =
        persistence::create_character(db(app), ACCOUNT, name, 1, 2).expect("create character");
    let mut inventory = persistence::load_inventory(db(app), character_id);
    inventory.items.push(persistence::InventoryItemData {
        item_guid: persistence::next_id(db(app), "item_instance"),
        item_id,
        name: String::new(),
        quality: 1,
        required_level: 1,
        stack_count: 1,
        vendor_sell_price: 1,
        unique_auction: false,
    });
    persistence::save_inventory(db(app), character_id, &inventory);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 2,
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
            RemoteId(PeerId::Local(peer)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: ACCOUNT,
            },
            PlayerLink(player),
        ))
        .id();
    add_pair::<InventorySnapshot>(app, connection);
    add_pair::<EquipmentSnapshot>(app, connection);
    add_pair::<InventoryDelta>(app, connection);
    add_pair::<InventoryError>(app, connection);
    add_pair::<SwapItem>(app, connection);
    add_pair::<AppearanceCollectionUpdate>(app, connection);
    Client {
        character_id,
        connection,
    }
}

fn add_pair<M: Message>(app: &mut App, connection: Entity) {
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
    settle(app);
}

fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

fn send<M: Message, C: Channel>(app: &mut App, client: &Client, message: M) {
    app.world_mut()
        .get_mut::<MessageSender<M>>(client.connection)
        .unwrap()
        .send::<C>(message);
    settle(app);
}

/// Collection updates received per connection, drained every frame.
#[derive(Resource, Default)]
struct Inbox(Vec<(Entity, Vec<u32>)>);

fn collect(
    mut receivers: Query<(Entity, &mut MessageReceiver<AppearanceCollectionUpdate>)>,
    mut inbox: ResMut<Inbox>,
) {
    for (connection, mut receiver) in &mut receivers {
        inbox.0.extend(
            receiver
                .receive()
                .map(|update| (connection, update.appearances)),
        );
    }
}

fn updates(app: &mut App, client: &Client) -> Vec<Vec<u32>> {
    let mut inbox = app.world_mut().resource_mut::<Inbox>();
    let (mine, rest) = std::mem::take(&mut inbox.0)
        .into_iter()
        .partition(|(connection, _)| *connection == client.connection);
    inbox.0 = rest;
    mine.into_iter()
        .map(|(_, appearances)| appearances)
        .collect()
}

fn backpack_slot_of(app: &App, character_id: u64, item_id: u32) -> ItemLocation {
    let record =
        persistence::inventory_store::load_inventory_record(db(app), character_id).unwrap();
    let guid = record
        .inventory
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .unwrap()
        .item_guid;
    let placement = record
        .placements
        .iter()
        .find(|placement| placement.item_guid == guid)
        .unwrap();
    ItemLocation::Bag {
        bag: placement.bag,
        slot: placement.slot,
    }
}

#[test]
fn equipping_a_bind_on_equip_sword_teaches_its_appearance_to_the_account() {
    let Some(mut app) = setup_app() else { return };
    let paladin = spawn_client(&mut app, 1, "Uther", WORN_SHORTSWORD);
    enter_world(&mut app, &paladin);
    let entered = updates(&mut app, &paladin);
    assert_eq!(
        entered.len(),
        1,
        "the collection is sent on entering the world"
    );
    assert!(
        !entered[0].contains(&SHORTSWORD_APPEARANCE),
        "a sword in the bags is not learned"
    );

    let from = backpack_slot_of(&app, paladin.character_id, WORN_SHORTSWORD);
    send::<_, InventoryChannel>(
        &mut app,
        &paladin,
        SwapItem {
            from,
            to: ItemLocation::Equipment(EquipmentSlot::MainHand),
        },
    );

    let learned = updates(&mut app, &paladin);
    assert_eq!(learned.len(), 1);
    assert!(learned[0].contains(&SHORTSWORD_APPEARANCE));
    assert!(load_account_appearances(db(&app), ACCOUNT).has(SHORTSWORD_APPEARANCE));

    // Another character on the account starts with it.
    let alt = spawn_client(&mut app, 2, "Turalyon", VERIGANS_FIST);
    enter_world(&mut app, &alt);
    let alt_updates = updates(&mut app, &alt);
    assert!(alt_updates[0].contains(&SHORTSWORD_APPEARANCE));
    assert_restart(app, ACCOUNT, &[(SHORTSWORD_APPEARANCE, true)]);
}

#[test]
fn transmogacq_destroying_an_unbound_boe_collects_and_sends_its_appearance() {
    let mut app = setup_app().unwrap();
    let paladin = spawn_client(&mut app, 1, "Uther", WORN_SHORTSWORD);
    add_pair::<shared::protocol::DestroyItem>(&mut app, paladin.connection);
    enter_world(&mut app, &paladin);
    updates(&mut app, &paladin);
    assert!(!load_account_appearances(db(&app), ACCOUNT).has(SHORTSWORD_APPEARANCE));
    let location = backpack_slot_of(&app, paladin.character_id, WORN_SHORTSWORD);
    send::<_, InventoryChannel>(
        &mut app,
        &paladin,
        shared::protocol::DestroyItem { location, count: 0 },
    );
    assert!(
        persistence::load_inventory(db(&app), paladin.character_id)
            .items
            .iter()
            .all(|item| item.item_id != WORN_SHORTSWORD)
    );
    assert!(load_account_appearances(db(&app), ACCOUNT).has(SHORTSWORD_APPEARANCE));
    assert!(
        updates(&mut app, &paladin)
            .iter()
            .any(|update| update.contains(&SHORTSWORD_APPEARANCE))
    );
    assert_restart(app, ACCOUNT, &[(SHORTSWORD_APPEARANCE, true)]);
}

#[test]
fn transmogacq_failed_destroy_does_not_collect_an_unbound_boe() {
    let mut app = setup_app().unwrap();
    let paladin = spawn_client(&mut app, 1, "Uther", WORN_SHORTSWORD);
    add_pair::<shared::protocol::DestroyItem>(&mut app, paladin.connection);
    enter_world(&mut app, &paladin);
    updates(&mut app, &paladin);
    send::<_, InventoryChannel>(
        &mut app,
        &paladin,
        shared::protocol::DestroyItem {
            location: ItemLocation::Bag { bag: 0, slot: 15 },
            count: 0,
        },
    );
    assert!(
        persistence::load_inventory(db(&app), paladin.character_id)
            .items
            .iter()
            .any(|item| item.item_id == WORN_SHORTSWORD)
    );
    assert!(!load_account_appearances(db(&app), ACCOUNT).has(SHORTSWORD_APPEARANCE));
    assert!(updates(&mut app, &paladin).is_empty());
    assert_restart(app, ACCOUNT, &[(SHORTSWORD_APPEARANCE, false)]);
}

#[test]
fn owning_a_bind_on_pickup_item_teaches_its_appearance() {
    let Some(mut app) = setup_app() else { return };
    let paladin = spawn_client(&mut app, 1, "Uther", VERIGANS_FIST);

    enter_world(&mut app, &paladin);

    let entered = updates(&mut app, &paladin);
    assert!(entered[0].contains(&FIST_APPEARANCE), "{entered:?}");
    assert!(load_account_appearances(db(&app), ACCOUNT).has(FIST_APPEARANCE));
    assert_restart(app, ACCOUNT, &[(FIST_APPEARANCE, true)]);
}
