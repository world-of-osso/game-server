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
    GameDatabase::new(db)
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
fn toyfx20_login_initializes_source_racial_pvp_faction() {
    for (race, expected) in [
        (1, 2),
        (2, 1),
        (52, 2),
        (70, 1),
        (84, 1),
        (85, 2),
        (86, 2),
        (91, 1),
    ] {
        let db = test_db();
        let (id, _) = persistence::create_character(&db.db, 1, "Pvpidentity", race, 1).unwrap();
        let (app, player) = select_character_in_world(db, 1, id);
        assert_eq!(
            app.world()
                .get::<shared::components::CurrentPvpFaction>(player),
            Some(&shared::components::CurrentPvpFaction(expected)),
            "race {race}"
        );
    }
}

#[test]
fn fresh_login_initializes_noncombat_state_for_pvp_context() {
    let db = test_db();
    let (id, _) = persistence::create_character(&db.db, 1, "Pvpfresh", 1, 2).unwrap();
    let (app, player) = select_character_in_world(db, 1, id);
    assert_eq!(
        app.world().get::<shared::components::CombatStatus>(player),
        Some(&shared::components::CombatStatus(false))
    );
}

#[test]
fn source_start_facing_is_used_on_login_without_repositioning_saved_characters() {
    for moved in [false, true] {
        let db = test_db();
        let (id, mut created) = persistence::create_character(&db.db, 1, "Mulgore", 6, 1).unwrap();
        if moved {
            created.x = -2322.0;
            created.y = -377.0;
            created.z = 63.0;
            persistence::save_character(&db.db, id, &created);
        }
        let (app, player) = select_character_in_world(db, 1, id);
        let entity = app.world().entity(player);
        let position = entity.get::<shared::components::Position>().unwrap();
        assert_eq!(
            (position.x, position.y, position.z),
            (created.x, created.z, -created.y)
        );
        let rotation = entity.get::<shared::components::Rotation>().unwrap();
        let expected_yaw = if moved {
            0.0
        } else {
            0.302378 + std::f32::consts::FRAC_PI_2
        };
        assert!(
            (rotation.y - expected_yaw).abs() < 0.001,
            "yaw {}",
            rotation.y
        );
    }
}

#[test]
fn cold_character_selection_returns_while_writer_is_held_and_reopens_initialized_state() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let raw = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&raw);
    let (character, _) = persistence::create_character(&raw, 1, "Coldwriter", 1, 1).unwrap();
    // Never call hydration/loaders here: secondary character rows remain absent.
    let db = GameDatabase::new(raw);
    let held = db.db.begin_write().unwrap();
    let (release, blocked) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let explicitly_released = blocked.recv_timeout(Duration::from_secs(5)).is_ok();
        held.abort().unwrap();
        explicitly_released
    });
    let (mut app, connection) =
        queue_character_selection(db, 1, character, crate::maps::MapStore::continents());
    app.update();
    app.update();
    assert!(
        !holder.is_finished(),
        "cold selection blocked on the durable writer"
    );
    assert!(
        app.world()
            .get::<crate::auth::InWorld>(connection)
            .is_none()
    );
    assert_eq!(
        app.world_mut()
            .query::<&ClientId>()
            .iter(app.world())
            .count(),
        0
    );
    release.send(()).unwrap();
    assert!(
        holder.join().unwrap(),
        "writer watchdog is not a success condition"
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while app
        .world()
        .get::<crate::auth::InWorld>(connection)
        .is_none()
    {
        assert!(
            std::time::Instant::now() < deadline,
            "cold selection never completed"
        );
        app.update();
        std::thread::yield_now();
    }
    let gold = app
        .world_mut()
        .query::<&Gold>()
        .single(app.world())
        .unwrap()
        .0;
    assert_eq!(gold, persistence::InventoryData::default().gold);
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    use redb::ReadableDatabase;
    let read = reopened.begin_read().unwrap();
    for name in [
        "character_inventories",
        "pvp_states",
        "world_maps",
        "friends",
        "ignore_lists",
        "collections",
        "reputations_retail_v2",
    ] {
        assert!(
            read.open_table(redb::TableDefinition::<u64, &[u8]>::new(name))
                .unwrap()
                .get(character)
                .unwrap()
                .is_some(),
            "missing durable {name}"
        );
    }
    assert_eq!(persistence::load_inventory(&reopened, character).gold, gold);
    assert_eq!(
        persistence::load_collections(&reopened, character),
        crate::collection::default_collection_state()
    );
}

#[test]
fn character_relogin_reads_owned_achievements_while_default_or_migration_write_is_pending() {
    for legacy_row in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let raw = redb::Database::create(file.path()).unwrap();
        persistence::create_tables(&raw);
        let db = GameDatabase::new(raw);
        let (character, _) =
            persistence::create_character(&db.db, 1, "Relogpending", 1, 1).unwrap();
        // Initialize unrelated login tables before isolating the achievement reader.
        load_selected_character_state(&db, 1, character, 1, 1);
        let txn = db.db.begin_write().unwrap();
        {
            let mut rows = txn.open_table(persistence::ACHIEVEMENTS).unwrap();
            if legacy_row {
                rows.insert(
                    character,
                    br#"{"earned_ids":[10],"earned_at":{"10":90}}"#.as_slice(),
                )
                .unwrap();
            } else {
                rows.remove(character).unwrap();
            }
        }
        txn.commit().unwrap();
        let mut pending = crate::achievement::PlayerAchievementState::default();
        pending.earned_ids.extend([10, 14782]);
        pending.earned_at.insert(14782, 100);
        pending.criteria_progress.insert(162, 41);
        let held = db.db.begin_write().unwrap();
        let (release, blocked) = std::sync::mpsc::channel();
        let holder = std::thread::spawn(move || {
            let signal = blocked.recv_timeout(Duration::from_secs(5));
            held.abort().unwrap();
            signal.expect("fixture explicitly releases the held writer");
        });
        crate::achievement::save_monthly_contribution(&db, 1, character, &pending);
        let (mut app, connection) =
            queue_character_selection(db, 1, character, crate::maps::MapStore::continents());
        app.update();
        app.update();
        assert!(
            !holder.is_finished(),
            "selection must return while the writer is held; legacy={legacy_row}"
        );
        let loaded = crate::achievement::load_character_progress(
            app.world().resource::<GameDatabase>(),
            1,
            character,
        );
        assert!(
            loaded.earned_ids.contains(&14782),
            "login lost queued achievement; legacy={legacy_row}"
        );
        assert_eq!(loaded.earned_at[&14782], 100);
        assert_eq!(loaded.criteria_progress[&162], 41);
        release.send(()).unwrap();
        holder.join().unwrap();
        let player = finish_character_selection(&mut app, connection);
        let hydrated = app
            .world()
            .get::<crate::achievement::PlayerAchievementState>(player)
            .unwrap();
        assert!(hydrated.earned_ids.contains(&14782));
        assert_eq!(hydrated.earned_at[&14782], 100);
        assert_eq!(hydrated.criteria_progress[&162], 41);
        drop(app);
        let reopened = redb::Database::open(file.path()).unwrap();
        let saved = persistence::load_achievements(&reopened, character);
        assert!(
            saved.earned_ids.contains(&14782),
            "stale default/migration must not overwrite pending progress"
        );
        assert_eq!(saved.criteria_progress[&162], 41);
    }
}

#[test]
fn load_selected_character_state_syncs_pvp_currencies_from_wallet() {
    let db = test_db();
    let character_id = 55;
    let mut wallet = shared::currency::CurrencyWallet::default();
    wallet
        .add(1792, 420, currency_by_id(1792).unwrap())
        .unwrap();
    wallet.add(1602, 80, currency_by_id(1602).unwrap()).unwrap();
    persistence::save_currencies(&db.db, character_id, &wallet);

    let pvp = crate::pvp::PlayerPvpProgress {
        honor: 12,
        conquest: 7,
        ..Default::default()
    };
    persistence::save_pvp(&db.db, character_id, &pvp);

    let state = load_selected_character_state(&db, 1, character_id, 1, 1);

    assert_eq!(state.currencies.get(1792), 420);
    assert_eq!(state.currencies.get(1602), 80);
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

#[derive(Resource, Default)]
struct DeathReplies(Vec<DeathStateUpdate>);

fn collect_death_replies(
    mut receivers: Query<&mut MessageReceiver<DeathStateUpdate>>,
    mut replies: ResMut<DeathReplies>,
) {
    for mut receiver in &mut receivers {
        replies.0.extend(receiver.receive());
    }
}

fn select_character_in_world(
    db: GameDatabase,
    account_id: u64,
    character_id: u64,
) -> (App, Entity) {
    select_character_with_maps(
        db,
        account_id,
        character_id,
        crate::maps::MapStore::continents(),
    )
}

fn queue_character_selection(
    db: GameDatabase,
    account_id: u64,
    character_id: u64,
    maps: crate::maps::MapStore,
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
    app.init_resource::<DeathReplies>();
    app.add_systems(Last, collect_death_replies);
    app.add_plugins(DeathPlugin);
    app.insert_resource(maps);
    app.init_resource::<crate::maps::InstanceManager>();
    // Reproduce the legal production ordering that used to send Alive before
    // the next frame restored the newly selected ghost.
    app.add_systems(
        Update,
        process_select_character
            .after(crate::death::restore_persisted_ghosts)
            .before(crate::death::send_initial_death_snapshot),
    );
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
            MessageSender::<DeathStateUpdate>::default(),
            MessageReceiver::<DeathStateUpdate>::default(),
        ))
        .id();
    app.world_mut()
        .entity_mut(connection)
        .get_mut::<MessageSender<SelectCharacter>>()
        .expect("connection should have a character-select sender")
        .send::<AuthChannel>(SelectCharacter { character_id });

    (app, connection)
}

fn select_character_with_maps(
    db: GameDatabase,
    account_id: u64,
    character_id: u64,
    maps: crate::maps::MapStore,
) -> (App, Entity) {
    let (mut app, connection) = queue_character_selection(db, account_id, character_id, maps);
    let player = finish_character_selection(&mut app, connection);
    (app, player)
}

fn finish_character_selection(app: &mut App, connection: Entity) -> Entity {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while app
        .world()
        .get::<crate::auth::InWorld>(connection)
        .is_none()
    {
        assert!(
            std::time::Instant::now() < deadline,
            "selection never completed"
        );
        app.update();
        std::thread::yield_now();
    }
    app.update();
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(Last);
    app.world()
        .get::<crate::networking::PlayerLink>(connection)
        .unwrap()
        .0
}

#[test]
fn skyborne_real_kits_create_persist_and_select_all_authored_classes() {
    let kit = crate::player_create_info::OwnedKit::from_world_db().expect("real world.db kit");
    let pairs = [(95, &[1, 3, 4, 8, 11][..]), (96, &[1, 3, 4, 7, 11][..])];
    for (race, classes) in pairs {
        for &class in classes {
            let directory = tempfile::tempdir().expect("tempdir");
            let path = directory.path().join("skyborne.redb");
            let db = redb::Database::create(&path).expect("create database");
            persistence::create_tables(&db);
            let (character_id, data) = persistence::create_character_with_appearance(
                &db,
                1,
                "Skyborne",
                race,
                class,
                CharacterAppearance::default(),
                &kit.kit(),
            )
            .expect("authored Skyborne pair should create with real kit");
            assert_eq!(data.level, 1);
            let expected = if race == 95 {
                (4088.501, 1847.5, 976.169)
            } else {
                (4051.417, 1858.625, 977.620)
            };
            assert_eq!((data.x, data.y, data.z), expected);
            drop(db);

            let db = redb::Database::open(&path).expect("reopen character storage");
            let saved = persistence::load_character(&db, character_id).expect("saved character");
            assert_eq!((saved.race, saved.class, saved.level), (race, class, 1));
            assert_eq!(
                crate::maps::persistence::load_character_map(&db, character_id),
                2991
            );
            let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
            let maps = crate::maps::store::load_map_store(&conn).expect("scratch map overlay");
            let (app, player) =
                select_character_with_maps(GameDatabase::new(db), 1, character_id, maps);
            let entity = app.world().entity(player);
            assert_eq!(entity.get::<UnitLevel>(), Some(&UnitLevel(1)));
            assert_eq!(entity.get::<Player>().unwrap().race, race);
            assert_eq!(entity.get::<Player>().unwrap().class, class);
            let template = if race == 95 { 3629 } else { 3628 };
            assert_eq!(
                entity.get::<shared::components::UnitFactionTemplate>(),
                Some(&shared::components::UnitFactionTemplate(template))
            );
            assert_eq!(entity.get::<crate::maps::InMap>().unwrap().map_id, 2991);
            let terrain = app.world().resource::<TerrainReplies>().0.first().unwrap();
            assert_eq!(terrain.map_name, "2991");
            assert_eq!((terrain.initial_tile_y, terrain.initial_tile_x), (28, 24));
            let rotation = entity.get::<shared::components::Rotation>().unwrap();
            assert!((rotation.y - std::f32::consts::PI).abs() < 0.001);
        }
    }
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

/// The drowning system runs for players with `MirrorTimers`; a player enters the world
/// with every timer off.
#[test]
fn selected_player_enters_the_world_with_mirror_timers_off() {
    let db = test_db();
    let account_id = 10;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Diver", 1, 1)
        .expect("character should be created");

    let (app, player) = select_character_in_world(db, account_id, character_id);

    assert_eq!(
        app.world()
            .entity(player)
            .get::<crate::mirror_timers::MirrorTimers>(),
        Some(&crate::mirror_timers::MirrorTimers::default())
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
        definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
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
    app.insert_resource(crate::maps::MapStore::continents());
    app.init_resource::<crate::maps::InstanceManager>();
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
    app.insert_resource(crate::maps::MapStore::continents());
    app.init_resource::<crate::maps::InstanceManager>();
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
    initial_terrain_on_enter_map(0, wow_position)
}

/// First `LoadTerrain` a character saved on map `map_id` receives on entering the world.
fn initial_terrain_on_enter_map(map_id: u32, wow_position: (f32, f32, f32)) -> LoadTerrain {
    let db = test_db();
    let account_id = 21;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Tilewalker", 1, 1)
        .expect("character should be created");
    let mut data = persistence::load_character(&db.db, character_id).expect("character data");
    // CharacterData stores WoW coordinates.
    (data.x, data.y, data.z) = wow_position;
    persistence::save_character(&db.db, character_id, &data);
    crate::maps::persistence::save_character_map(&db.db, character_id, map_id);

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
    app.insert_resource(crate::maps::MapStore::continents());
    app.init_resource::<crate::maps::InstanceManager>();
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
fn character_saved_in_wailing_caverns_logs_in_after_server_restart_with_gathering_nodes() {
    let directory = tempfile::tempdir().unwrap();
    let db_path = directory.path().join("wc-login.redb");
    let account_id = 21;
    let character_id = {
        let db = redb::Database::create(&db_path).unwrap();
        persistence::create_tables(&db);
        let (id, _) = persistence::create_character(&db, account_id, "Cavewalker", 1, 1).unwrap();
        let mut data = persistence::load_character(&db, id).unwrap();
        (data.x, data.y, data.z) = (-151.139, 414.367, -72.6294);
        data.level = 80;
        persistence::save_character(&db, id, &data);
        crate::maps::persistence::save_character_map(&db, id, 43);
        id
    };
    // Each app reopens the real persisted character with a new map manager/pool state.
    // The second login also has no NPC population index: GO pools still need state.
    for with_creatures in [true, false] {
        let path = crate::test_data::world_db_path();
        let conn = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut world_data = crate::world_data::load_world_data(&path);
        if !with_creatures {
            world_data.spawns_by_map.remove(&43);
        }
        let objects = crate::game_objects::load_game_object_spawns(&conn).unwrap();
        let pooled_guids: Vec<_> = objects
            .0
            .iter()
            .filter(|spawn| {
                spawn.map_id == 43
                    && spawn.info.go_type == shared::protocol::GAMEOBJECT_TYPE_GATHERING_NODE
            })
            .filter(|spawn| world_data.pools.gameobject_pool(spawn.guid).is_some())
            .map(|spawn| spawn.guid)
            .collect();
        assert!(
            !pooled_guids.is_empty(),
            "authentic saved map has gathering candidates"
        );
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(shared::protocol::ProtocolPlugin);
        app.add_plugins(DeathPlugin);
        app.insert_resource(crate::maps::store::load_map_store(&conn).unwrap());
        app.init_resource::<crate::maps::InstanceManager>();
        app.init_resource::<crate::spawn_pools::PoolSpawns>();
        app.insert_resource(world_data);
        app.insert_resource(objects);
        app.insert_resource(crate::gathering::load_data(&conn).unwrap());
        app.insert_resource(GameDatabase::new(redb::Database::open(&db_path).unwrap()));
        app.init_resource::<TerrainReplies>();
        app.add_systems(Update, process_select_character);
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
        let map = app
            .world_mut()
            .query::<(&ClientId, &crate::maps::InMap)>()
            .iter(app.world())
            .find(|(id, _)| id.0 == character_id)
            .map(|(_, map)| *map)
            .expect("saved character entered copy");
        assert_eq!(map.map_id, 43);
        assert_ne!(map.instance_id, 0);
        assert_eq!(
            app.world().resource::<TerrainReplies>().0[0].map_name,
            "wailingcaverns"
        );
        let selected: std::collections::BTreeSet<_> = pooled_guids
            .iter()
            .copied()
            .filter(|guid| {
                app.world().resource::<crate::spawn_pools::PoolSpawns>().0[&map]
                    .spawns
                    .gameobject_spawned(*guid)
            })
            .collect();
        let world = app.world_mut();
        let spawned: std::collections::BTreeSet<_> = world
            .query::<(&crate::maps::InMap, &crate::gathering::NodeSource)>()
            .iter(world)
            .filter(|(copy, node)| **copy == map && pooled_guids.contains(&node.0.guid))
            .map(|(_, node)| node.0.guid)
            .collect();
        assert_eq!(
            spawned, selected,
            "login spawns only this copy's selected authentic nodes"
        );
    }
}

#[test]
fn a_character_saved_on_kalimdor_enters_kalimdor() {
    // Ratchet, Northern Barrens: tile row 39 (WoW Y), column 33 (WoW X).
    let ratchet = initial_terrain_on_enter_map(1, (-956.0, -3754.0, 5.0));
    assert_eq!(
        (
            ratchet.map_name.as_str(),
            ratchet.initial_tile_y,
            ratchet.initial_tile_x
        ),
        ("kalimdor", 39, 33)
    );
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

fn ghost_at_graveyard(character_id: u64) -> PlayerDeathState {
    PlayerDeathState {
        state: DeathState::Ghost,
        corpse: Some(shared::death::Corpse {
            player: character_id,
            map_id: 0,
            x: -8913.5,
            y: 82.0,
            z: 140.25,
            created_at: 1234,
        }),
        graveyard: Some(shared::death::Graveyard {
            id: 7,
            zone_id: 12,
            faction: 0,
            map_id: 0,
            x: -9050.0,
            y: 90.0,
            z: 200.0,
        }),
    }
}

/// Save the character the way a logout does (`save_live_character`), at the
/// graveyard with 0 health.
fn log_out(db: &GameDatabase, character_id: u64, health: f32, death: Option<&PlayerDeathState>) {
    let data = persistence::load_character(&db.db, character_id).unwrap();
    let player = Player {
        name: data.name.clone(),
        race: data.race,
        class: data.class,
        appearance: data.appearance.clone(),
    };
    let position = shared::components::Position {
        x: -9050.0,
        y: 90.0,
        z: 200.0,
    };
    let health = Health {
        current: health,
        max: 100.0,
    };
    persistence::save_live_character(&db.db, character_id, &player, &position, &health, death, 0);
}

/// Player::LoadFromDB (PLAYER_FLAGS_GHOST → still dead, a ghost) and
/// Player::LoadCorpse (the corpse stays where it died): a released ghost
/// that logs out comes back a ghost, not freshly Dead at the graveyard.
#[test]
fn relogged_ghost_comes_back_a_ghost_with_its_corpse() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("death-login.redb");
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);
    let account_id = 11;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Spirit", 1, 1)
        .expect("character should be created");
    let ghost = ghost_at_graveyard(character_id);
    log_out(&db, character_id, 0.0, Some(&ghost));
    drop(db);
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);

    let (mut app, player) = select_character_in_world(db, account_id, character_id);
    for _ in 0..3 {
        app.update();
    }
    let updates = &app.world().resource::<DeathReplies>().0;
    let states: Vec<_> = updates
        .iter()
        .filter_map(|update| {
            update
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.state.clone())
        })
        .collect();
    assert_eq!(states, vec![shared::protocol::DeathStateSnapshot::Ghost]);
    let player = app.world().entity(player);
    let death_state = player.get::<PlayerDeathState>().expect("ghost restored");
    assert_eq!(death_state.state, DeathState::Ghost);
    assert_eq!(death_state.corpse, ghost.corpse);
    assert_eq!(death_state.graveyard, ghost.graveyard);
    let position = player.get::<shared::components::Position>().unwrap();
    assert_eq!((position.x, position.z), (-9050.0, 200.0));
    assert_eq!(player.get::<Health>().unwrap().current, 0.0);
}

#[test]
fn relogged_dead_player_keeps_death_position_without_another_durability_loss() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dead-login.redb");
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);
    let account_id = 12;
    let (character_id, _) =
        persistence::create_character(&db.db, account_id, "Unreleased", 1, 1).unwrap();
    let mut data = persistence::load_character(&db.db, character_id).unwrap();
    data.equipment_appearance.entries = vec![EquippedAppearanceEntry {
        slot: EquipmentVisualSlot::Head,
        item_id: Some(100),
        definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
        display_info_id: Some(200),
        inventory_type: 1,
        hidden: false,
    }];
    let expected_durability =
        crate::durability::load_equipped_durability(&data, &helm_durability());
    persistence::save_character(&db.db, character_id, &data);
    let mut dead = ghost_at_graveyard(character_id);
    dead.state = DeathState::Dead;
    log_out(&db, character_id, 0.0, Some(&dead));
    drop(db);
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);

    let (app, player) = select_character_in_world(db, account_id, character_id);
    let player = app.world().entity(player);
    assert_eq!(player.get::<PlayerDeathState>(), Some(&dead));
    assert_eq!(player.get::<Health>().unwrap().current, 0.0);
    assert_eq!(
        player.get::<EquippedDurability>().unwrap().slots,
        expected_durability.slots
    );
}

/// Frozen pre-change corpse encoding; no death-state row existed in this layout.
#[derive(bitcode::Encode)]
struct PreDeathStateCorpse {
    map_id: u16,
    x: f32,
    y: f32,
    z: f32,
    created_at: u64,
}

#[test]
fn pre_death_state_record_loads_ghost_and_corpse_on_first_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-death.redb");
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);
    let account_id = 13;
    let (character_id, mut data) =
        persistence::create_character(&db.db, account_id, "LegacySpirit", 1, 1).unwrap();
    data.health = 0.0;
    persistence::save_character(&db.db, character_id, &data);
    let ghost = ghost_at_graveyard(character_id);
    let corpse = ghost.corpse.as_ref().unwrap();
    let bytes = bitcode::encode(&PreDeathStateCorpse {
        map_id: corpse.map_id,
        x: corpse.x,
        y: corpse.y,
        z: corpse.z,
        created_at: corpse.created_at,
    });
    let txn = db.db.begin_write().unwrap();
    txn.delete_table(redb::TableDefinition::<u64, &[u8]>::new(
        "character_death_states",
    ))
    .unwrap();
    txn.open_table(persistence::CHARACTER_CORPSES)
        .unwrap()
        .insert(character_id, bytes.as_slice())
        .unwrap();
    txn.commit().unwrap();
    drop(db);
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);
    let (mut app, player) = select_character_in_world(db, account_id, character_id);
    for _ in 0..3 {
        app.update();
    }
    let updates = &app.world().resource::<DeathReplies>().0;
    let states: Vec<_> = updates
        .iter()
        .filter_map(|update| {
            update
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.state.clone())
        })
        .collect();
    assert_eq!(states, vec![shared::protocol::DeathStateSnapshot::Ghost]);
    let player = app.world().entity(player);
    let state = player.get::<PlayerDeathState>().unwrap();
    assert_eq!(state.state, DeathState::Ghost);
    assert_eq!(state.corpse, ghost.corpse);
    assert_eq!(player.get::<Health>().unwrap().current, 0.0);
}

#[test]
fn character_saved_after_resurrection_logs_in_alive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("alive-login.redb");
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);
    let account_id = 12;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Risen", 1, 1)
        .expect("character should be created");
    log_out(
        &db,
        character_id,
        0.0,
        Some(&ghost_at_graveyard(character_id)),
    );
    log_out(&db, character_id, 50.0, Some(&PlayerDeathState::default()));
    drop(db);
    let db = GameDatabase::new(redb::Database::create(&path).unwrap());
    persistence::create_tables(&db.db);

    let (mut app, player) = select_character_in_world(db, account_id, character_id);
    app.update();
    let player = app.world().entity(player);
    assert!(
        player
            .get::<PlayerDeathState>()
            .is_none_or(|state| state.state == DeathState::Alive)
    );
    assert_eq!(player.get::<Health>().unwrap().current, 50.0);
}

/// A second session of the same account (re-auth) selects the character again while
/// the first session's player is still in the world.
fn select_again(app: &mut App, account_id: u64, character_id: u64) {
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(account_id + 100)),
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
        .get_mut::<MessageSender<SelectCharacter>>(connection)
        .unwrap()
        .send::<AuthChannel>(SelectCharacter { character_id });
    finish_character_selection(app, connection);
}

#[test]
fn re_entering_the_world_keeps_where_the_old_session_moved_to() {
    let db = test_db();
    let account_id = 11;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Relogger", 1, 1)
        .expect("character should be created");
    let (mut app, stale) = select_character_in_world(db, account_id, character_id);
    // The old session flew to Sentinel Hill (Bevy space) and was never saved.
    let landed = shared::components::Position {
        x: -10550.8,
        y: 59.07,
        z: -1034.49,
    };
    *app.world_mut()
        .get_mut::<shared::components::Position>(stale)
        .unwrap() = landed;

    select_again(&mut app, account_id, character_id);

    let fresh = app
        .world_mut()
        .query::<(Entity, &ClientId, &shared::components::Position)>()
        .iter(app.world())
        .find(|(entity, client_id, _)| client_id.0 == character_id && *entity != stale)
        .map(|(_, _, position)| *position)
        .expect("the new session spawns a player");
    assert_eq!(fresh, landed, "the new player starts where the old one was");
    let stored =
        persistence::load_character(&app.world().resource::<GameDatabase>().db, character_id)
            .unwrap();
    assert_eq!((stored.x, stored.y, stored.z), (-10550.8, 1034.49, 59.07));
}

/// Fbloading (292 health) entered through character select onto the Northshire lake bed
/// (azeroth 32,48, WoW -8558, -490, seabed 140.04): with the real `GroundMap` and
/// `update_mirror_timers`, breath runs out 180 s of server time after the dive and drowning
/// hits 58-59 health (20% + urand(0, 1)) every second.
#[test]
fn a_selected_player_on_the_lake_bed_drowns_every_second_after_180_s() {
    let db = test_db();
    let account_id = 11;
    let (character_id, _) = persistence::create_character(&db.db, account_id, "Fbloading", 1, 1)
        .expect("character should be created");
    let mut data = persistence::load_character(&db.db, character_id).unwrap();
    (data.x, data.y, data.z) = (-8558.0, -490.0, 140.04);
    (data.health, data.max_health) = (292.0, 292.0);
    persistence::save_character(&db.db, character_id, &data);

    let (mut app, player) = select_character_in_world(db, account_id, character_id);
    app.insert_resource(crate::ground::GroundMap::new(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        std::collections::HashMap::from([(0, 775_971)]),
    ));
    app.add_systems(FixedUpdate, crate::mirror_timers::update_mirror_timers);
    let mut hits = Vec::new();
    let mut last = 292.0;
    for tick in 0..20 * 184 {
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_millis(50));
        app.insert_resource(time);
        app.world_mut().run_schedule(FixedUpdate);
        let health = app.world().get::<Health>(player).unwrap().current;
        if health < last {
            hits.push((tick, last - health));
        }
        last = health;
    }

    let ticks: Vec<usize> = hits.iter().map(|(tick, _)| *tick).collect();
    assert_eq!(ticks, vec![3601, 3621, 3641, 3661]);
    assert!(
        hits.iter().all(|(_, hit)| (58.0..=59.0).contains(hit)),
        "{hits:?}"
    );
}
