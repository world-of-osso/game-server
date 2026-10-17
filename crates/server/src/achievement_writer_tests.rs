use super::*;

#[test]
fn guild_news_query_waits_for_the_queued_achievement_commit() {
    assert_guild_news_reader_order(false);
}

#[test]
fn guild_news_subscription_waits_for_the_queued_achievement_commit() {
    assert_guild_news_reader_order(true);
}

fn assert_guild_news_reader_order(subscribe_first: bool) {
    use lightyear::prelude::{MessageReceiver, MessageSender};
    use shared::protocol::{GuildChannel, GuildNewsKind, GuildNewsRequest, GuildNewsState};
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Newsordered").unwrap();
    persistence::set_character_guild_by_id(h.db(), h.character_id, 7, "Ordered News").unwrap();
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&source).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert((Level(50), PlayerAchievementState::default()));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<GuildNewsRequest>::default(),
        MessageReceiver::<GuildNewsRequest>::default(),
        MessageSender::<GuildNewsState>::default(),
        MessageReceiver::<GuildNewsState>::default(),
    ));
    if subscribe_first {
        h.app
            .world_mut()
            .get_mut::<MessageSender<GuildNewsRequest>>(h.connection)
            .unwrap()
            .send::<GuildChannel>(GuildNewsRequest::Query);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut baseline = Vec::new();
        while baseline.is_empty() && std::time::Instant::now() < deadline {
            h.app.update();
            baseline.extend(
                h.app
                    .world_mut()
                    .get_mut::<MessageReceiver<GuildNewsState>>(h.connection)
                    .unwrap()
                    .receive(),
            );
            std::thread::yield_now();
        }
        assert_eq!(baseline.len(), 1);
        assert!(baseline[0].entries.is_empty());
    }
    let held = h.db().begin_write().unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    update_player_events(
        h.app.world_mut(),
        h.player,
        &[CriteriaEvent::ReachLevel(50)],
        None,
    )
    .unwrap();
    if !subscribe_first {
        h.app
            .world_mut()
            .get_mut::<MessageSender<GuildNewsRequest>>(h.connection)
            .unwrap()
            .send::<GuildChannel>(GuildNewsRequest::Query);
    }
    let mut replies = Vec::new();
    let started = std::time::Instant::now();
    for _ in 0..4 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<GuildNewsState>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    let elapsed = started.elapsed();
    let before_commit = replies.len();
    release.join().unwrap();
    crate::test_data::drain_persistence(h.app.world().resource::<GameDatabase>());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while replies.is_empty() && std::time::Instant::now() < deadline {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<GuildNewsState>>(h.connection)
                .unwrap()
                .receive(),
        );
        std::thread::yield_now();
    }
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "news tick blocked: {elapsed:?}"
    );
    assert_eq!(
        before_commit, 0,
        "no stale reply before commit; subscribed={subscribe_first}"
    );
    assert_eq!(
        replies.len(),
        1,
        "must publish committed news; subscribed={subscribe_first}"
    );
    assert_eq!(replies[0].entries.len(), 1);
    assert!(matches!(
        replies[0].entries[0].kind,
        GuildNewsKind::PlayerAchievement {
            achievement_id: 14782,
            ..
        }
    ));
    let character = h.character_id;
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        crate::guild_news::query(&db, character).entries,
        replies[0].entries
    );
}

// Original busy-writer RED from 0ff7a921a, retained before FIFO integration.
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
fn achievement_account_snapshot_never_waits_for_a_busy_durable_writer() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 41, "Accountproof", 1, 1).unwrap();
    let db = GameDatabase::new(db);
    let mut state = PlayerAchievementState::default();
    state.criteria_progress.insert(162, 41);
    let held = db.db.begin_write().unwrap();
    let blocker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = std::time::Instant::now();
    account_store::save_account_progress(
        &db,
        &state,
        &AchievementCatalog::default(),
        account_store::AccountUpdate {
            account: 41,
            character,
            completed_map: None,
            race: 1,
            now: 100,
        },
    );
    let elapsed = started.elapsed();
    blocker.join().unwrap();
    let db = db.into_database();
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "account tick waited for writer: {elapsed:?}"
    );
    assert_eq!(
        persistence::load_achievements(&db, character).criteria_progress[&162],
        41
    );
}
