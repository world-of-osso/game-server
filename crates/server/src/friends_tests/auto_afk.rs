use std::sync::atomic::Ordering;
use std::time::Duration;

use bevy::time::TimeUpdateStrategy;

use super::*;

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
    app.insert_resource(GameDatabase::new(db));
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
