use std::time::Duration;

use bevy::prelude::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{
    AppMessageExt, Connected, MessageReceiver, MessageSender, PeerId, RemoteId,
};
use shared::components::{
    CharacterAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Gold, Health, Player,
    UnitLevel,
};
use shared::currency::currency_by_id;
use shared::death::DeathState;
use shared::protocol::{
    AuthChannel, ChatMessage, ChatType, DeathStateUpdate, EnterWorldResponse, LoadTerrain,
    SelectCharacter,
};

use crate::auth::Authenticated;
use crate::auth_character::{load_selected_character_state, process_select_character};
use crate::death::{DeathPlugin, PlayerDeathState};
use crate::durability::{EquippedDurability, ItemDurability, ItemDurabilityInfo, apply_death_loss};
use crate::gold::{GoldPlugin, GoldUpdateQueue};
use crate::persistence::{self, ClientId, GameDatabase};

fn test_db() -> GameDatabase {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("auth_character.redb");
    let db = redb::Database::create(&path).expect("db");
    persistence::create_tables(&db);
    std::mem::forget(dir);
    GameDatabase { db }
}

#[test]
fn queued_gold_updates_coalesce_and_skip_equal_or_offline_players() {
    let mut app = App::new();
    app.add_plugins(GoldPlugin);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Thrall".into(),
                race: 2,
                class: 7,
                appearance: CharacterAppearance::default(),
            },
            ClientId(42),
            Gold(100),
        ))
        .id();

    {
        let mut updates = app.world_mut().resource_mut::<GoldUpdateQueue>();
        updates.record(42, 125);
        updates.record(42, 150);
    }
    app.update();

    assert_eq!(app.world().entity(player).get::<Gold>().unwrap().0, 150);
    let changed_after_update = app
        .world()
        .entity(player)
        .get_ref::<Gold>()
        .unwrap()
        .last_changed();

    {
        let mut updates = app.world_mut().resource_mut::<GoldUpdateQueue>();
        updates.record(42, 150);
        updates.record(99, 900);
    }
    app.update();

    assert_eq!(app.world().entity(player).get::<Gold>().unwrap().0, 150);
    assert_eq!(
        app.world()
            .entity(player)
            .get_ref::<Gold>()
            .unwrap()
            .last_changed(),
        changed_after_update
    );
    assert!(app.world().resource::<GoldUpdateQueue>().is_empty());
}

#[test]
fn load_selected_character_state_syncs_pvp_currencies_from_wallet() {
    let db = test_db();
    let character_id = 55;
    let mut wallet = shared::currency::CurrencyWallet::default();
    wallet.add(1, 420, currency_by_id(1).unwrap()).unwrap();
    wallet.add(2, 80, currency_by_id(2).unwrap()).unwrap();
    persistence::save_currencies(&db.db, character_id, &wallet);

    let pvp = crate::pvp::PlayerPvpProgress {
        honor: 12,
        conquest: 7,
        ..Default::default()
    };
    persistence::save_pvp(&db.db, character_id, &pvp);

    let state = load_selected_character_state(&db, character_id, 1);

    assert_eq!(state.currencies.get(1), 420);
    assert_eq!(state.currencies.get(2), 80);
    assert_eq!(state.pvp.progress.honor, 420);
    assert_eq!(state.pvp.progress.conquest, 80);
}

/// Runs `SelectCharacter` through `process_select_character` and returns the spawned player.
/// Item 100 (the test helm) has 70 durability points.
fn helm_durability() -> ItemDurability {
    ItemDurability(
        [(
            100,
            ItemDurabilityInfo {
                max: 70,
                cost_per_point: Some(1.0),
            },
        )]
        .into_iter()
        .collect(),
    )
}

fn select_character_in_world(
    db: GameDatabase,
    account_id: u64,
    character_id: u64,
) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<SelectCharacter>();
    app.register_message::<EnterWorldResponse>();
    app.register_message::<LoadTerrain>();
    app.register_message::<DeathStateUpdate>();
    app.insert_resource(db);
    app.insert_resource(helm_durability());
    app.add_plugins(DeathPlugin);
    app.add_systems(Update, process_select_character);
    app.finish();
    app.cleanup();

    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id },
            MessageSender::<SelectCharacter>::default(),
            MessageReceiver::<SelectCharacter>::default(),
            MessageSender::<EnterWorldResponse>::default(),
            MessageReceiver::<EnterWorldResponse>::default(),
            MessageSender::<LoadTerrain>::default(),
            MessageReceiver::<LoadTerrain>::default(),
            MessageSender::<DeathStateUpdate>::default(),
            MessageReceiver::<DeathStateUpdate>::default(),
        ))
        .id();
    app.world_mut()
        .entity_mut(connection)
        .get_mut::<MessageSender<SelectCharacter>>()
        .expect("connection should have a character-select sender")
        .send::<AuthChannel>(SelectCharacter { character_id });

    app.update();
    app.update();
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);

    let player = app
        .world_mut()
        .query::<(Entity, &ClientId)>()
        .iter(app.world())
        .find_map(|(entity, client_id)| (client_id.0 == character_id).then_some(entity))
        .expect("selecting a character should spawn its player entity");
    (app, player)
}

#[test]
fn selected_player_replicates_its_persisted_level_as_unit_level() {
    let db = test_db();
    let account_id = 9;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Leveled", 1, 2)
        .expect("character should be created");
    let mut data = persistence::load_character(&db.db, character_id).unwrap();
    data.level = 14;
    persistence::save_character(&db.db, character_id, &data);

    let (app, player) = select_character_in_world(db, account_id, character_id);

    assert_eq!(
        app.world().entity(player).get::<UnitLevel>(),
        Some(&UnitLevel(14))
    );
}

#[test]
fn selecting_zero_health_character_queues_one_death_transition() {
    let db = test_db();
    let account_id = 7;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "DeadChar", 1, 1)
        .expect("character should be created");
    let mut data = persistence::load_character(&db.db, character_id)
        .expect("created character should have persisted data");
    data.health = 0.0;
    data.equipment_appearance.entries = vec![EquippedAppearanceEntry {
        slot: EquipmentVisualSlot::Head,
        item_id: Some(100),
        display_info_id: Some(200),
        inventory_type: 1,
        hidden: false,
    }];
    persistence::save_character(&db.db, character_id, &data);

    let expected_initial = crate::durability::load_equipped_durability(&data, &helm_durability());
    assert_eq!(expected_initial.slots.len(), 1);
    let mut expected_after_death = expected_initial.clone();
    assert!(apply_death_loss(&mut expected_after_death));

    let (mut app, player) = select_character_in_world(db, account_id, character_id);
    let player_entity = app.world().entity(player);
    assert_eq!(player_entity.get::<Health>().unwrap().current, 0.0);
    let death_state = player_entity
        .get::<PlayerDeathState>()
        .expect("loaded zero health must queue a death transition");
    assert_eq!(death_state.state, DeathState::Dead);
    assert!(death_state.corpse.is_some());
    assert_eq!(
        player_entity
            .get::<EquippedDurability>()
            .expect("selected player should have durability")
            .slots,
        expected_after_death.slots
    );

    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);
    assert_eq!(
        app.world()
            .entity(player)
            .get::<EquippedDurability>()
            .unwrap()
            .slots,
        expected_after_death.slots
    );
}

#[derive(Resource, Default)]
struct EnterWorldReplies(Vec<EnterWorldResponse>);

/// Unread receiver buffers are dropped on the next receive, so drain every frame.
fn collect_enter_world_replies(
    mut receivers: Query<&mut MessageReceiver<EnterWorldResponse>>,
    mut replies: ResMut<EnterWorldReplies>,
) {
    for mut receiver in &mut receivers {
        replies.0.extend(receiver.receive());
    }
}

#[test]
fn selecting_character_with_invalid_race_class_is_rejected() {
    let db = test_db();
    let account_id = 8;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Stonebark", 3, 1)
        .expect("character should be created");
    let mut data = persistence::load_character(&db.db, character_id).unwrap();
    data.class = 11;
    persistence::save_character(&db.db, character_id, &data);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<SelectCharacter>();
    app.register_message::<EnterWorldResponse>();
    app.register_message::<LoadTerrain>();
    app.insert_resource(db);
    app.add_plugins(DeathPlugin);
    app.add_systems(Update, process_select_character);
    app.init_resource::<EnterWorldReplies>();
    app.add_systems(Last, collect_enter_world_replies);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id },
            MessageSender::<SelectCharacter>::default(),
            MessageReceiver::<SelectCharacter>::default(),
            MessageSender::<EnterWorldResponse>::default(),
            MessageReceiver::<EnterWorldResponse>::default(),
            MessageSender::<LoadTerrain>::default(),
            MessageReceiver::<LoadTerrain>::default(),
        ))
        .id();
    app.world_mut()
        .get_mut::<MessageSender<SelectCharacter>>(connection)
        .unwrap()
        .send::<AuthChannel>(SelectCharacter { character_id });
    for _ in 0..3 {
        app.update();
    }

    let responses = &app.world().resource::<EnterWorldReplies>().0;
    assert_eq!(responses.len(), 1);
    assert!(!responses[0].success);
    assert_eq!(
        responses[0].error.as_deref(),
        Some("Class 11 is not available for race 3")
    );
    let spawned = app
        .world_mut()
        .query::<&ClientId>()
        .iter(app.world())
        .count();
    assert_eq!(spawned, 0);
}

#[derive(Resource, Default)]
struct ChatReplies(Vec<ChatMessage>);

fn collect_chat_replies(
    mut receivers: Query<&mut MessageReceiver<ChatMessage>>,
    mut replies: ResMut<ChatReplies>,
) {
    for mut receiver in &mut receivers {
        replies.0.extend(receiver.receive());
    }
}

#[test]
fn entering_world_sends_one_system_motd_message() {
    let db = test_db();
    let account_id = 12;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Motdreader", 1, 1)
        .expect("character should be created");
    let mut motd = shared::motd::Motd::default();
    motd.set("Raid reset is Tuesday.", 1_000).unwrap();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<SelectCharacter>();
    app.register_message::<EnterWorldResponse>();
    app.register_message::<LoadTerrain>();
    app.register_message::<ChatMessage>();
    app.insert_resource(db);
    app.insert_resource(motd);
    app.add_plugins(DeathPlugin);
    app.add_systems(Update, process_select_character);
    app.add_systems(
        Update,
        crate::messaging::send_motd_on_enter_world.after(process_select_character),
    );
    app.init_resource::<ChatReplies>();
    app.add_systems(Last, collect_chat_replies);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id },
            MessageSender::<SelectCharacter>::default(),
            MessageReceiver::<SelectCharacter>::default(),
            MessageSender::<EnterWorldResponse>::default(),
            MessageReceiver::<EnterWorldResponse>::default(),
            MessageSender::<LoadTerrain>::default(),
            MessageReceiver::<LoadTerrain>::default(),
            MessageSender::<ChatMessage>::default(),
            MessageReceiver::<ChatMessage>::default(),
        ))
        .id();
    app.world_mut()
        .get_mut::<MessageSender<SelectCharacter>>(connection)
        .unwrap()
        .send::<AuthChannel>(SelectCharacter { character_id });
    for _ in 0..4 {
        app.update();
    }

    let messages = &app.world().resource::<ChatReplies>().0;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].channel, ChatType::System);
    assert_eq!(messages[0].content, "Raid reset is Tuesday.");
}

#[derive(Resource, Default)]
struct TerrainReplies(Vec<LoadTerrain>);

fn collect_terrain_replies(
    mut receivers: Query<&mut MessageReceiver<LoadTerrain>>,
    mut replies: ResMut<TerrainReplies>,
) {
    for mut receiver in &mut receivers {
        replies.0.extend(receiver.receive());
    }
}

/// First `LoadTerrain` a character saved at WoW `(x, y, z)` receives on entering the world.
fn initial_terrain_on_enter(wow_position: (f32, f32, f32)) -> LoadTerrain {
    let db = test_db();
    let account_id = 21;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Tilewalker", 1, 1)
        .expect("character should be created");
    let mut data = persistence::load_character(&db.db, character_id).expect("character data");
    // CharacterData stores WoW coordinates.
    (data.x, data.y, data.z) = wow_position;
    persistence::save_character(&db.db, character_id, &data);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<SelectCharacter>();
    app.register_message::<EnterWorldResponse>();
    app.register_message::<LoadTerrain>();
    app.insert_resource(db);
    app.add_plugins(DeathPlugin);
    app.add_systems(Update, process_select_character);
    app.init_resource::<TerrainReplies>();
    app.add_systems(Last, collect_terrain_replies);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id },
            MessageSender::<SelectCharacter>::default(),
            MessageReceiver::<SelectCharacter>::default(),
            MessageSender::<EnterWorldResponse>::default(),
            MessageReceiver::<EnterWorldResponse>::default(),
            MessageSender::<LoadTerrain>::default(),
            MessageReceiver::<LoadTerrain>::default(),
        ))
        .id();
    app.world_mut()
        .get_mut::<MessageSender<SelectCharacter>>(connection)
        .unwrap()
        .send::<AuthChannel>(SelectCharacter { character_id });
    for _ in 0..4 {
        app.update();
    }
    app.world()
        .resource::<TerrainReplies>()
        .0
        .first()
        .cloned()
        .expect("entering the world sends LoadTerrain")
}

#[test]
fn entering_world_loads_the_tile_under_the_character() {
    // Auctioneer Fitch, Stormwind Trade District: ADT azeroth_30_48 (row 30, column 48).
    let stormwind = initial_terrain_on_enter((-8821.0, 660.8, 97.5));
    assert_eq!(
        (
            stormwind.map_name.as_str(),
            stormwind.initial_tile_y,
            stormwind.initial_tile_x
        ),
        ("azeroth", 30, 48)
    );
    // Northshire Abbey: ADT azeroth_32_48.
    let northshire = initial_terrain_on_enter((-8949.0, -132.0, 83.0));
    assert_eq!(
        (northshire.initial_tile_y, northshire.initial_tile_x),
        (32, 48)
    );
}
