use super::*;
use lightyear::connection::host::HostClient;
use shared::protocol::AdventureGuideSnapshot;

#[test]
fn achievement_monthly_snapshot_never_waits_for_a_busy_durable_writer() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 41, "Asyncproof", 1, 1).unwrap();
    let db = GameDatabase::new(db);
    let held = db.db.begin_write().unwrap();
    let blocker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = std::time::Instant::now();
    save_monthly_contribution(&db, 41, character, &PlayerAchievementState::default());
    let elapsed = started.elapsed();
    blocker.join().unwrap();
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "tick waited for writer: {elapsed:?}"
    );
}

#[test]
fn achievement_account_monthly_writer_snapshots_preserve_cross_owner_order() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (first, _) = persistence::create_character(&db, 41, "Firstqueue", 1, 1).unwrap();
    let (second, _) = persistence::create_character(&db, 42, "Otherqueue", 1, 1).unwrap();
    let db = GameDatabase::new(db);
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let (release, blocked) = std::sync::mpsc::channel();
    let (entered, ready) = std::sync::mpsc::channel();
    db.writer.enqueue("blocked earlier commit", move |_| {
        entered.send(()).unwrap();
        blocked.recv().map_err(|error| error.to_string())?;
        Ok(())
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap();
    let mut first_state = PlayerAchievementState::default();
    first_state.criteria_progress.insert(1832, 2);
    let mut other_state = PlayerAchievementState::default();
    other_state.criteria_progress.insert(1832, 9);
    for (account, character, state) in [(41, first, &first_state), (42, second, &other_state)] {
        account_store::save_account_progress(
            &db,
            state,
            &catalog,
            account_store::AccountUpdate {
                account,
                character,
                completed_map: None,
                race: 1,
                now: 999,
            },
        );
    }
    first_state.criteria_progress.insert(1832, 3);
    first_state.monthly = Some(crate::adventure_guide::MonthlyCriteriaState {
        interval_start: 0,
        tracking_since: 0,
        counters: BTreeMap::from([(3209, 7)]),
        tracked: BTreeSet::new(),
        ignored_login_achievements: BTreeSet::new(),
    });
    save_monthly_contribution(&db, 41, first, &first_state);
    assert_eq!(monthly_contributions(&db, 41), vec![first_state.clone()]);
    assert_eq!(monthly_contributions(&db, 42), vec![other_state.clone()]);
    assert!(
        persistence::load_achievements(&db.db, first)
            .criteria_progress
            .is_empty()
    );
    release.send(()).unwrap();
    let drained = db.into_database();
    drop(drained);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, first),
        first_state
    );
    assert_eq!(
        persistence::load_achievements(&reopened, second),
        other_state
    );
}

#[test]
fn achievement_login_known_spell_facts_fit_one_tick_budget() {
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let mut query = conn.prepare("SELECT DISTINCT Asset FROM criteria WHERE Type=34 AND Asset>0 ORDER BY Asset LIMIT 100").unwrap();
    let spells: Vec<u32> = query
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .chain(std::iter::once(32298))
        .collect();
    let events: Vec<_> = catalog
        .known_spell_events(spells.into_iter())
        .into_iter()
        .filter(|event| catalog.tracks(*event))
        .collect();
    assert!(
        events.len() >= 50,
        "exercise a real second-page-sized known-spell population"
    );
    let mut reference = PlayerAchievementState::default();
    let reference_toasts: BTreeSet<_> = events
        .iter()
        .flat_map(|event| {
            catalog.update_at(
                &mut reference,
                *event,
                None,
                1,
                &PlayerConditions::default(),
            )
        })
        .map(|toast| toast.achievement_id)
        .collect();
    let mut world = World::new();
    world.insert_resource(catalog);
    world.init_resource::<crate::maps::ServerClock>();
    let player = world
        .spawn((
            Player {
                name: "Perfknown".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(80),
            crate::maps::InMap::continent(0),
            PlayerAchievementState::default(),
        ))
        .id();
    let mut actual_toasts = BTreeSet::new();
    let fastest = (0..3)
        .map(|_| {
            world
                .entity_mut(player)
                .insert(PlayerAchievementState::default());
            let start = std::time::Instant::now();
            let (_, toasts) = update_player_events(&mut world, player, &events, None).unwrap();
            actual_toasts = toasts
                .into_iter()
                .map(|toast| toast.achievement_id)
                .collect();
            start.elapsed()
        })
        .min()
        .unwrap();
    let actual = world.get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(actual.criteria_progress, reference.criteria_progress);
    assert_eq!(actual.earned_ids, reference.earned_ids);
    assert_eq!(actual_toasts, reference_toasts);
    println!("{} real known-spell facts: {fastest:?}", events.len());
    assert_eq!(
        world
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&6155),
        Some(&1)
    );
    assert!(
        fastest < std::time::Duration::from_millis(50),
        "login achievement reconstruction alone exhausts the tick: {fastest:?}"
    );
}

#[test]
fn achievement_frame_publishes_one_final_snapshot_and_preserves_every_toast() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 41, "Perfproof", 1, 1).unwrap();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&conn).unwrap();
    let journal_csv = crate::test_data::world_db_path()
        .parent()
        .unwrap()
        .join("db2/active/AdventureJournal.csv");
    let adventure = crate::adventure_guide::load_catalog(&conn, &catalog, &journal_csv);
    assert!(matches!(
        adventure,
        crate::adventure_guide::AdventureCatalog::Ready(_)
    ));
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.add_plugins((
        AchievementPlugin,
        crate::adventure_guide::AdventureGuidePlugin,
    ));
    app.insert_resource(catalog);
    app.insert_resource(adventure);
    app.insert_resource(GameDatabase::new(db));
    app.finish();
    app.cleanup();
    app.update();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Perfproof".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Level(1),
            PlayerAchievementState::default(),
            crate::maps::InMap::continent(0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            PlayerLink(player),
            Authenticated { account_id: 41 },
            InWorld {
                character_id: character,
            },
            RemoteId(PeerId::Netcode(41)),
            Connected,
            HostClient { buffer: Vec::new() },
            MessageSender::<AchievementStateUpdate>::default(),
            MessageReceiver::<AchievementStateUpdate>::default(),
            MessageSender::<AdventureGuideSnapshot>::default(),
            MessageReceiver::<AdventureGuideSnapshot>::default(),
        ))
        .id();
    app.update();
    app.world_mut()
        .get_mut::<MessageReceiver<AchievementStateUpdate>>(connection)
        .unwrap()
        .receive()
        .for_each(drop);
    app.world_mut()
        .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(connection)
        .unwrap()
        .receive()
        .for_each(drop);
    record_criteria_event(app.world_mut(), player, CriteriaEvent::ReachLevel(10));
    record_criteria_event(app.world_mut(), player, CriteriaEvent::ReachLevel(20));
    app.update();
    let updates: Vec<_> = app
        .world_mut()
        .get_mut::<MessageReceiver<AchievementStateUpdate>>(connection)
        .unwrap()
        .receive()
        .collect();
    let snapshots: Vec<_> = updates
        .iter()
        .filter_map(|update| update.snapshot.as_ref())
        .collect();
    assert_eq!(
        snapshots.len(),
        1,
        "one final state, not one whole catalog per credit/toast"
    );
    for id in [6, 7] {
        assert!(snapshots[0].earned_ids.contains(&id));
        assert_eq!(
            updates
                .iter()
                .filter(|update| update
                    .completed
                    .as_ref()
                    .is_some_and(|toast| toast.achievement_id == id))
                .count(),
            1
        );
    }
    let guides: Vec<_> = app
        .world_mut()
        .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(connection)
        .unwrap()
        .receive()
        .collect();
    assert_eq!(
        guides.len(),
        1,
        "publish the guide once after all achievement credits"
    );
    app.update();
    assert_eq!(
        app.world_mut()
            .get_mut::<MessageReceiver<AchievementStateUpdate>>(connection)
            .unwrap()
            .receive()
            .count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(connection)
            .unwrap()
            .receive()
            .count(),
        0
    );
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    assert!(saved.earned_ids.contains(&6) && saved.earned_ids.contains(&7));
}
