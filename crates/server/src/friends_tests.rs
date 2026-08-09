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
fn set_presence_tracks_manual_afk_and_resets_idle() {
    let mut app = App::new();
    let player_entity = app.world_mut().spawn((test_player("Theron"),)).id();
    app.world_mut().entity_mut(player_entity).insert((
        PresenceStatus::Online,
        PresenceTracker {
            idle_secs: 42.0,
            manual: None,
        },
    ));

    let mut apply_presence = IntoSystem::into_system(
        move |mut players: Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>| {
            set_presence(player_entity, PresenceStatus::Afk, &mut players)
        },
    );
    apply_presence.initialize(app.world_mut());
    let message = apply_presence.run((), app.world_mut()).unwrap().unwrap();
    assert_eq!(message, "presence set to afk");

    let mut query = app
        .world_mut()
        .query::<(&Player, &PresenceStatus, &PresenceTracker)>();
    let (_, status, tracker) = query
        .iter(app.world())
        .find(|(player, _, _)| player.name == "Theron")
        .unwrap();
    assert_eq!(*status, PresenceStatus::Afk);
    assert_eq!(tracker.idle_secs, 0.0);
    assert_eq!(tracker.manual, Some(ManualPresence::Afk));
}

#[test]
fn player_activity_clears_manual_afk_but_preserves_dnd() {
    let mut app = App::new();
    let afk_entity = app.world_mut().spawn((test_player("Theron"),)).id();
    let dnd_entity = app.world_mut().spawn((test_player("Jaina"),)).id();

    app.world_mut().entity_mut(afk_entity).insert((
        PresenceStatus::Afk,
        PresenceTracker {
            idle_secs: 180.0,
            manual: Some(ManualPresence::Afk),
        },
    ));
    app.world_mut().entity_mut(dnd_entity).insert((
        PresenceStatus::Dnd,
        PresenceTracker {
            idle_secs: 240.0,
            manual: Some(ManualPresence::Dnd),
        },
    ));

    let mut mark_activity = IntoSystem::into_system(
        move |mut players: Query<(&mut PresenceStatus, &mut PresenceTracker), With<Player>>| {
            mark_player_activity(afk_entity, &mut players);
            mark_player_activity(dnd_entity, &mut players);
        },
    );
    mark_activity.initialize(app.world_mut());
    let _ = mark_activity.run((), app.world_mut());

    let mut query = app
        .world_mut()
        .query::<(&Player, &PresenceStatus, &PresenceTracker)>();
    let mut states = query
        .iter(app.world())
        .map(|(player, status, tracker)| (player.name.clone(), *status, *tracker))
        .collect::<Vec<_>>();
    states.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(states[0].0, "Jaina");
    assert_eq!(states[0].1, PresenceStatus::Dnd);
    assert_eq!(states[0].2.idle_secs, 0.0);
    assert_eq!(states[0].2.manual, Some(ManualPresence::Dnd));

    assert_eq!(states[1].0, "Theron");
    assert_eq!(states[1].1, PresenceStatus::Online);
    assert_eq!(states[1].2.idle_secs, 0.0);
    assert_eq!(states[1].2.manual, None);
}

#[test]
fn auto_afk_promotes_idle_players_without_manual_presence() {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.world_mut().spawn((
        test_player("Theron"),
        PresenceStatus::Online,
        PresenceTracker::default(),
    ));
    app.world_mut().spawn((
        test_player("Jaina"),
        PresenceStatus::Dnd,
        PresenceTracker {
            idle_secs: AUTO_AFK_SECS,
            manual: Some(ManualPresence::Dnd),
        },
    ));

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(AUTO_AFK_SECS + 1.0));
    app.add_systems(Update, tick_auto_afk);
    app.update();

    let mut query = app
        .world_mut()
        .query::<(&Player, &PresenceStatus, &PresenceTracker)>();
    let mut states = query
        .iter(app.world())
        .map(|(player, status, tracker)| (player.name.clone(), *status, *tracker))
        .collect::<Vec<_>>();
    states.sort_by(|a, b| a.0.cmp(&b.0));

    assert_eq!(states[0].0, "Jaina");
    assert_eq!(states[0].1, PresenceStatus::Dnd);
    assert_eq!(states[0].2.manual, Some(ManualPresence::Dnd));
    assert_eq!(states[1].0, "Theron");
    assert_eq!(states[1].1, PresenceStatus::Afk);
    assert_eq!(states[1].2.manual, None);
}

#[test]
fn auto_afk_does_not_dirty_unchanged_online_presence() {
    let mut app = App::new();
    app.init_resource::<Time>();
    let player = app
        .world_mut()
        .spawn((
            test_player("Theron"),
            PresenceStatus::Online,
            PresenceTracker::default(),
        ))
        .id();
    app.add_systems(Update, tick_auto_afk);

    app.update();
    let first_tick = app
        .world()
        .entity(player)
        .get_ref::<PresenceStatus>()
        .unwrap()
        .last_changed();

    app.update();

    assert_eq!(
        app.world()
            .entity(player)
            .get_ref::<PresenceStatus>()
            .unwrap()
            .last_changed(),
        first_tick,
        "unchanged Online presence must not advance its change tick"
    );
}

#[test]
fn process_presence_requests_system_initializes_without_conflict() {
    let mut app = App::new();
    app.init_resource::<FriendSnapshotIndex>();
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
