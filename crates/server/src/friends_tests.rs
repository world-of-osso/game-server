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
    app.add_systems(Update, process_presence_requests);
    app.update();
}
