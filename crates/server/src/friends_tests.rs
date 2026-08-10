use std::sync::atomic::Ordering;
use std::time::Duration;

use bevy::time::TimeUpdateStrategy;

use super::*;

fn test_player(name: &str) -> Player {
    Player {
        name: name.into(),
        race: 1,
        class: 8,
        appearance: Default::default(),
    }
}

fn friend(name: &str) -> FriendRecord {
    FriendRecord {
        name: name.into(),
        note: String::new(),
    }
}

fn auto_afk_deadline_app() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(1));
    app.init_resource::<AutoAfkScheduler>();
    app.add_systems(Update, process_due_auto_afk);
    app.update();
    app
}

fn schedule_test_deadline(app: &mut App, player: Entity, now: Duration) -> u64 {
    let mut tracker = PresenceTracker::default();
    {
        let mut scheduler = app.world_mut().resource_mut::<AutoAfkScheduler>();
        schedule_auto_afk(player, &mut tracker, now, &mut scheduler);
    }
    let generation = tracker.generation;
    app.world_mut().entity_mut(player).insert(tracker);
    generation
}

fn run_activity(app: &mut App, player: Entity, now: Duration) {
    let mut system = IntoSystem::into_system(
        move |mut players: Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>,
              mut scheduler: ResMut<AutoAfkScheduler>| {
            mark_player_activity(player, &mut players, &mut scheduler, now, None);
        },
    );
    system.initialize(app.world_mut());
    system.run((), app.world_mut()).unwrap();
}

fn run_presence_request(
    app: &mut App,
    player: Entity,
    requested: PresenceStatus,
    now: Duration,
) -> String {
    let mut system = IntoSystem::into_system(
        move |mut players: Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>,
              mut scheduler: ResMut<AutoAfkScheduler>| {
            set_presence(player, requested, &mut players, &mut scheduler, now, None)
        },
    );
    system.initialize(app.world_mut());
    system.run((), app.world_mut()).unwrap().unwrap()
}

#[test]
fn add_friend_rejects_self_and_duplicates() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    crate::persistence::create_character(&db, 1, "Theron", 1, 2).unwrap();
    crate::persistence::create_character(&db, 2, "Alice", 1, 8).unwrap();

    let mut state = PlayerFriendsState::default();
    assert_eq!(
        add_friend(&db, &mut state, "Theron", "Theron").unwrap_err(),
        "cannot add yourself as a friend"
    );
    assert_eq!(
        add_friend(&db, &mut state, "Theron", "Nobody").unwrap_err(),
        "character 'Nobody' not found"
    );
    assert_eq!(
        add_friend(&db, &mut state, "Theron", "Alice").unwrap(),
        "friend added: Alice"
    );
    assert_eq!(
        add_friend(&db, &mut state, "Theron", "Alice").unwrap_err(),
        "'Alice' is already on your friends list"
    );
}

#[test]
fn remove_friend_requires_existing_entry() {
    let mut state = PlayerFriendsState {
        entries: vec![friend("Alice")],
    };

    assert_eq!(
        remove_friend(&mut state, "Bob").unwrap_err(),
        "'Bob' is not on your friends list"
    );
    assert_eq!(
        remove_friend(&mut state, "Alice").unwrap(),
        "friend removed: Alice"
    );
    assert!(state.entries.is_empty());
}

#[test]
fn friends_state_round_trips_through_persistence() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);

    let state = PlayerFriendsState {
        entries: vec![friend("Alice"), friend("Bob")],
    };
    crate::persistence::save_friends(&db, 42, &state);

    let loaded = crate::persistence::load_friends(&db, 42);
    assert_eq!(loaded, state);
}

#[test]
fn set_presence_tracks_manual_afk_and_invalidates_deadline() {
    let mut app = auto_afk_deadline_app();
    let player = app
        .world_mut()
        .spawn((test_player("Theron"), PresenceStatus::Online))
        .id();
    let initial_generation = schedule_test_deadline(&mut app, player, Duration::ZERO);

    let message = run_presence_request(&mut app, player, PresenceStatus::Afk, Duration::ZERO);
    assert_eq!(message, "presence set to afk");

    let entity = app.world().entity(player);
    let tracker = entity.get::<PresenceTracker>().unwrap();
    assert_eq!(
        *entity.get::<PresenceStatus>().unwrap(),
        PresenceStatus::Afk
    );
    assert_eq!(tracker.manual, Some(ManualPresence::Afk));
    assert!(tracker.generation > initial_generation);
}

#[test]
fn player_activity_clears_manual_afk_but_preserves_dnd() {
    let mut app = auto_afk_deadline_app();
    let afk_entity = app
        .world_mut()
        .spawn((test_player("Theron"), PresenceStatus::Afk))
        .id();
    let dnd_entity = app
        .world_mut()
        .spawn((test_player("Jaina"), PresenceStatus::Dnd))
        .id();
    schedule_test_deadline(&mut app, afk_entity, Duration::ZERO);
    schedule_test_deadline(&mut app, dnd_entity, Duration::ZERO);
    run_presence_request(&mut app, afk_entity, PresenceStatus::Afk, Duration::ZERO);
    run_presence_request(&mut app, dnd_entity, PresenceStatus::Dnd, Duration::ZERO);

    let afk_generation = app
        .world()
        .entity(afk_entity)
        .get::<PresenceTracker>()
        .unwrap()
        .generation;
    let dnd_generation = app
        .world()
        .entity(dnd_entity)
        .get::<PresenceTracker>()
        .unwrap()
        .generation;
    run_activity(&mut app, afk_entity, Duration::from_secs(1));
    run_activity(&mut app, dnd_entity, Duration::from_secs(1));

    let afk = app.world().entity(afk_entity);
    assert_eq!(
        *afk.get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    assert_eq!(afk.get::<PresenceTracker>().unwrap().manual, None);
    assert!(afk.get::<PresenceTracker>().unwrap().generation > afk_generation);

    let dnd = app.world().entity(dnd_entity);
    assert_eq!(*dnd.get::<PresenceStatus>().unwrap(), PresenceStatus::Dnd);
    assert_eq!(
        dnd.get::<PresenceTracker>().unwrap().manual,
        Some(ManualPresence::Dnd)
    );
    assert!(dnd.get::<PresenceTracker>().unwrap().generation > dnd_generation);
}

#[test]
fn non_due_auto_afk_players_are_not_visited() {
    let _test_lock = AUTO_AFK_TEST_LOCK.lock().unwrap();
    let mut app = auto_afk_deadline_app();
    let players = (0..3)
        .map(|index| {
            let player = app
                .world_mut()
                .spawn((
                    test_player(&format!("Player{index}")),
                    PresenceStatus::Online,
                ))
                .id();
            schedule_test_deadline(&mut app, player, Duration::ZERO);
            player
        })
        .collect::<Vec<_>>();

    app.update();
    let status_ticks = players
        .iter()
        .map(|&player| {
            app.world()
                .entity(player)
                .get_ref::<PresenceStatus>()
                .unwrap()
                .last_changed()
        })
        .collect::<Vec<_>>();
    AUTO_AFK_PLAYERS_VISITED.store(0, Ordering::SeqCst);

    for _ in 0..3 {
        app.update();
    }

    assert_eq!(
        AUTO_AFK_PLAYERS_VISITED.load(Ordering::SeqCst),
        0,
        "non-due automatic AFK players must not be visited"
    );
    for (player, status_tick) in players.iter().zip(status_ticks) {
        let entity = app.world().entity(*player);
        assert_eq!(
            *entity.get::<PresenceStatus>().unwrap(),
            PresenceStatus::Online
        );
        assert_eq!(
            entity.get_ref::<PresenceStatus>().unwrap().last_changed(),
            status_tick,
            "non-due players must not change presence"
        );
    }
}

#[test]
fn auto_afk_transitions_exactly_at_deadline() {
    let _test_lock = AUTO_AFK_TEST_LOCK.lock().unwrap();
    AUTO_AFK_TRANSITIONS.store(0, Ordering::SeqCst);

    let mut app = auto_afk_deadline_app();
    let player = app
        .world_mut()
        .spawn((test_player("Theron"), PresenceStatus::Online))
        .id();
    let generation = schedule_test_deadline(&mut app, player, Duration::ZERO);

    app.update();
    let online_tick = app
        .world()
        .entity(player)
        .get_ref::<PresenceStatus>()
        .unwrap()
        .last_changed();
    for _ in 0..298 {
        app.update();
    }
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 0);
    assert_eq!(
        app.world()
            .entity(player)
            .get::<PresenceTracker>()
            .unwrap()
            .generation,
        generation
    );
    assert_eq!(
        app.world()
            .entity(player)
            .get_ref::<PresenceStatus>()
            .unwrap()
            .last_changed(),
        online_tick
    );

    app.update();
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Afk
    );
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 1);
    let afk_tick = app
        .world()
        .entity(player)
        .get_ref::<PresenceStatus>()
        .unwrap()
        .last_changed();

    app.update();
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 1);
    assert_eq!(
        app.world()
            .entity(player)
            .get_ref::<PresenceStatus>()
            .unwrap()
            .last_changed(),
        afk_tick
    );
}

#[test]
fn activity_invalidates_old_auto_afk_deadline() {
    let _test_lock = AUTO_AFK_TEST_LOCK.lock().unwrap();
    AUTO_AFK_TRANSITIONS.store(0, Ordering::SeqCst);

    let mut app = auto_afk_deadline_app();
    let player = app
        .world_mut()
        .spawn((test_player("Theron"), PresenceStatus::Online))
        .id();
    let initial_generation = schedule_test_deadline(&mut app, player, Duration::ZERO);

    for _ in 0..299 {
        app.update();
    }
    run_activity(&mut app, player, Duration::from_secs(299));
    let rescheduled_generation = app
        .world()
        .entity(player)
        .get::<PresenceTracker>()
        .unwrap()
        .generation;
    assert!(rescheduled_generation > initial_generation);

    app.update();
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 0);
    for _ in 0..298 {
        app.update();
    }
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    app.update();
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Afk
    );
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 1);
}

#[test]
fn manual_dnd_survives_activity_and_inactivity() {
    let _test_lock = AUTO_AFK_TEST_LOCK.lock().unwrap();
    AUTO_AFK_TRANSITIONS.store(0, Ordering::SeqCst);

    let mut app = auto_afk_deadline_app();
    let player = app
        .world_mut()
        .spawn((test_player("Jaina"), PresenceStatus::Online))
        .id();
    schedule_test_deadline(&mut app, player, Duration::ZERO);
    let message = run_presence_request(&mut app, player, PresenceStatus::Dnd, Duration::ZERO);
    assert_eq!(message, "presence set to dnd");
    run_activity(&mut app, player, Duration::from_secs(1));

    for _ in 0..301 {
        app.update();
    }
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Dnd
    );
    assert_eq!(
        app.world()
            .entity(player)
            .get::<PresenceTracker>()
            .unwrap()
            .manual,
        Some(ManualPresence::Dnd)
    );
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn online_and_offline_presence_schedule_deadlines() {
    let _test_lock = AUTO_AFK_TEST_LOCK.lock().unwrap();
    AUTO_AFK_TRANSITIONS.store(0, Ordering::SeqCst);

    let mut app = auto_afk_deadline_app();
    let player = app
        .world_mut()
        .spawn((test_player("Theron"), PresenceStatus::Afk))
        .id();
    schedule_test_deadline(&mut app, player, Duration::ZERO);
    assert_eq!(
        run_presence_request(&mut app, player, PresenceStatus::Online, Duration::ZERO),
        "presence set to online"
    );
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    assert_eq!(
        app.world()
            .entity(player)
            .get::<PresenceTracker>()
            .unwrap()
            .manual,
        None
    );

    app.update();
    assert_eq!(
        run_presence_request(
            &mut app,
            player,
            PresenceStatus::Offline,
            Duration::from_secs(1)
        ),
        "presence set to online"
    );
    for _ in 0..299 {
        app.update();
    }
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    app.update();
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Afk
    );
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 1);
}

#[test]
fn initial_friends_presence_schedules_auto_afk_deadline() {
    let _test_lock = AUTO_AFK_TEST_LOCK.lock().unwrap();
    AUTO_AFK_TRANSITIONS.store(0, Ordering::SeqCst);

    let db = persistence::temp_database();
    let (character_id, _) = persistence::create_character(&db, 1, "Theron", 1, 8).unwrap();
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)));
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(1));
    app.insert_resource(GameDatabase { db });
    app.add_plugins(FriendsPlugin);
    app.update();
    let player = app
        .world_mut()
        .spawn((
            test_player("Theron"),
            Level(10),
            Zone { id: 1 },
            PlayerFriendsState::default(),
        ))
        .id();
    app.world_mut().spawn((
        Authenticated { account_id: 1 },
        InWorld { character_id },
        PlayerLink(player),
    ));

    app.update();
    app.update();
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    assert!(
        app.world()
            .entity(player)
            .get::<PresenceTracker>()
            .unwrap()
            .generation
            > 0
    );
    AUTO_AFK_TRANSITIONS.store(0, Ordering::SeqCst);

    for _ in 0..298 {
        app.update();
    }
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Online
    );
    app.update();
    assert_eq!(
        *app.world().entity(player).get::<PresenceStatus>().unwrap(),
        PresenceStatus::Afk
    );
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 1);
}

#[test]
fn despawned_auto_afk_deadline_is_ignored() {
    let _test_lock = AUTO_AFK_TEST_LOCK.lock().unwrap();
    AUTO_AFK_TRANSITIONS.store(0, Ordering::SeqCst);

    let mut app = auto_afk_deadline_app();
    let player = app
        .world_mut()
        .spawn((test_player("Theron"), PresenceStatus::Online))
        .id();
    schedule_test_deadline(&mut app, player, Duration::ZERO);
    app.world_mut().despawn(player);

    for _ in 0..300 {
        app.update();
    }
    assert_eq!(AUTO_AFK_TRANSITIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn process_presence_requests_system_initializes_without_conflict() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.init_resource::<FriendSnapshotIndex>();
    app.init_resource::<AutoAfkScheduler>();
    app.add_systems(Update, process_presence_requests);
    app.update();
}

fn spawn_friend_owner(
    app: &mut App,
    account_id: u64,
    character_id: u64,
    name: &str,
    friend_name: &str,
) -> Entity {
    let player = app
        .world_mut()
        .spawn((
            test_player(name),
            Level(10),
            Zone { id: 1 },
            PlayerFriendsState {
                entries: vec![friend(friend_name)],
            },
        ))
        .id();
    app.world_mut().spawn((
        Authenticated { account_id },
        InWorld { character_id },
        PlayerLink(player),
    ));
    player
}

fn tracker_tick(app: &App, player: Entity) -> bevy::ecs::change_detection::Tick {
    app.world()
        .entity(player)
        .get_ref::<FriendsSnapshotTracker>()
        .expect("initial friends snapshot should install a tracker")
        .last_changed()
}

fn tracker_snapshot(app: &App, player: Entity) -> FriendsSnapshot {
    app.world()
        .entity(player)
        .get::<FriendsSnapshotTracker>()
        .and_then(|tracker| tracker.last_sent.clone())
        .expect("initial friends snapshot should be tracked")
}

fn snapshot_entry<'a>(snapshot: &'a FriendsSnapshot, name: &str) -> &'a FriendCharacterSnapshot {
    snapshot
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .expect("friend should be present in snapshot")
}

fn save_character_level(app: &App, name: &str, level: u16) {
    let db = &app.world().resource::<GameDatabase>().db;
    let character_id = persistence::find_character_id_by_name(db, name).expect("character id");
    let mut character = persistence::load_character(db, character_id).expect("character data");
    character.level = level;
    persistence::save_character(db, character_id, &character);
}

#[test]
fn unchanged_frame_does_not_rebuild_offline_friend_snapshot() {
    let db = persistence::temp_database();
    let (theron_id, _) = persistence::create_character(&db, 1, "Theron", 1, 8).unwrap();
    persistence::create_character(&db, 2, "Alice", 1, 8).unwrap();

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(GameDatabase { db });
    app.add_plugins(FriendsPlugin);
    let theron = spawn_friend_owner(&mut app, 1, theron_id, "Theron", "Alice");
    app.update();
    app.update();

    let original_tick = tracker_tick(&app, theron);
    let original_snapshot = tracker_snapshot(&app, theron);
    save_character_level(&app, "Alice", 42);
    app.update();

    assert_eq!(tracker_snapshot(&app, theron), original_snapshot);
    assert_eq!(
        tracker_tick(&app, theron),
        original_tick,
        "an unchanged frame must not rebuild an offline friend snapshot from the database"
    );
}

#[test]
fn presence_change_updates_only_owners_listing_that_player() {
    let db = persistence::temp_database();
    let (theron_id, _) = persistence::create_character(&db, 1, "Theron", 1, 8).unwrap();
    let (jaina_id, _) = persistence::create_character(&db, 2, "Jaina", 1, 8).unwrap();
    persistence::create_character(&db, 3, "Alice", 1, 8).unwrap();
    persistence::create_character(&db, 4, "Bob", 1, 8).unwrap();

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(GameDatabase { db });
    app.add_plugins(FriendsPlugin);
    let theron = spawn_friend_owner(&mut app, 1, theron_id, "Theron", "Alice");
    let jaina = spawn_friend_owner(&mut app, 2, jaina_id, "Jaina", "Bob");
    let alice = app
        .world_mut()
        .spawn((
            test_player("Alice"),
            Level(10),
            Zone { id: 1 },
            PresenceStatus::Online,
        ))
        .id();
    app.update();
    app.update();

    let theron_tick = tracker_tick(&app, theron);
    let jaina_tick = tracker_tick(&app, jaina);
    app.world_mut()
        .entity_mut(alice)
        .insert(PresenceStatus::Afk);
    save_character_level(&app, "Bob", 42);
    app.update();

    assert_ne!(tracker_tick(&app, theron), theron_tick);
    assert_eq!(
        tracker_tick(&app, jaina),
        jaina_tick,
        "an unrelated offline database change must not rebuild another owner's snapshot"
    );
}

#[test]
fn friend_login_and_logout_update_the_listed_owner() {
    let db = persistence::temp_database();
    let (theron_id, _) = persistence::create_character(&db, 1, "Theron", 1, 8).unwrap();
    persistence::create_character(&db, 2, "Alice", 1, 8).unwrap();

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(GameDatabase { db });
    app.add_plugins(FriendsPlugin);
    let theron = spawn_friend_owner(&mut app, 1, theron_id, "Theron", "Alice");
    app.update();
    app.update();
    assert_eq!(
        snapshot_entry(&tracker_snapshot(&app, theron), "Alice").presence,
        PresenceStatus::Offline
    );

    let alice = app
        .world_mut()
        .spawn((
            test_player("Alice"),
            Level(10),
            Zone { id: 2 },
            PresenceStatus::Online,
        ))
        .id();
    app.update();
    assert_eq!(
        snapshot_entry(&tracker_snapshot(&app, theron), "Alice").presence,
        PresenceStatus::Online
    );

    app.world_mut().despawn(alice);
    app.update();
    assert_eq!(
        snapshot_entry(&tracker_snapshot(&app, theron), "Alice").presence,
        PresenceStatus::Offline
    );
}

#[test]
fn friend_zone_and_level_changes_refresh_the_listed_owner() {
    let db = persistence::temp_database();
    let (theron_id, _) = persistence::create_character(&db, 1, "Theron", 1, 8).unwrap();
    persistence::create_character(&db, 2, "Alice", 1, 8).unwrap();

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(GameDatabase { db });
    app.add_plugins(FriendsPlugin);
    let theron = spawn_friend_owner(&mut app, 1, theron_id, "Theron", "Alice");
    let alice = app
        .world_mut()
        .spawn((
            test_player("Alice"),
            Level(10),
            Zone { id: 1 },
            PresenceStatus::Online,
        ))
        .id();
    app.update();
    app.update();

    app.world_mut().entity_mut(alice).insert(Zone { id: 2 });
    app.update();
    assert_eq!(
        snapshot_entry(&tracker_snapshot(&app, theron), "Alice").area,
        "Zone 2"
    );

    app.world_mut().entity_mut(alice).insert(Level(20));
    app.update();
    assert_eq!(
        snapshot_entry(&tracker_snapshot(&app, theron), "Alice").level,
        20
    );
}

#[test]
fn friend_list_changes_replace_reverse_owner_links() {
    let mut app = App::new();
    let owner = app.world_mut().spawn_empty().id();
    let connection = app.world_mut().spawn_empty().id();
    let alice = app.world_mut().spawn_empty().id();
    let bob = app.world_mut().spawn_empty().id();
    let mut index = FriendSnapshotIndex::default();

    index.register_owner(
        owner,
        connection,
        &PlayerFriendsState {
            entries: vec![friend("Alice")],
        },
    );
    index.set_online(
        alice,
        &test_player("Alice"),
        &Level(10),
        Some(&Zone { id: 1 }),
        Some(&PresenceStatus::Online),
    );
    assert_eq!(index.take_dirty_owners(), vec![owner]);

    index.register_owner(
        owner,
        connection,
        &PlayerFriendsState {
            entries: vec![friend("Bob")],
        },
    );
    index.set_online(
        alice,
        &test_player("Alice"),
        &Level(10),
        Some(&Zone { id: 1 }),
        Some(&PresenceStatus::Afk),
    );
    assert!(index.take_dirty_owners().is_empty());

    index.set_online(
        bob,
        &test_player("Bob"),
        &Level(10),
        Some(&Zone { id: 1 }),
        Some(&PresenceStatus::Online),
    );
    assert_eq!(index.take_dirty_owners(), vec![owner]);
}
