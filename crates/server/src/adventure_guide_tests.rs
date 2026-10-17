use super::*;
use chrono::{TimeZone, Utc};
use lightyear::prelude::{MessageReceiver, MessageSender};
use rusqlite::{Connection, OpenFlags};
use shared::protocol::{
    AdventureGuideRequest, AdventureGuideSnapshot, InstanceChannel, PerksActivityProgress,
};

fn attach_requests(h: &mut crate::bank_test_harness::Harness) {
    AdventureGuidePlugin.build(&mut h.app);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<AdventureGuideRequest>::default(),
        MessageReceiver::<AdventureGuideRequest>::default(),
        MessageSender::<AdventureGuideSnapshot>::default(),
        MessageReceiver::<AdventureGuideSnapshot>::default(),
    ));
}
fn request(
    h: &mut crate::bank_test_harness::Harness,
    request: AdventureGuideRequest,
) -> AdventureGuideSnapshot {
    h.app
        .world_mut()
        .get_mut::<MessageSender<AdventureGuideRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(request);
    let mut replies = Vec::new();
    for _ in 0..10 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    replies.pop().expect("authoritative journal reply")
}
fn prepare_real_catalog(h: &mut crate::bank_test_harness::Harness, now: i64) {
    let path = std::env::var_os("ADVENTURE_GUIDE_WORLD_COPY")
        .expect("explicit private prepared world copy");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let criteria = crate::achievement::load_achievement_catalog(&conn).unwrap();
    let journal_csv = crate::test_data::world_db_path()
        .parent()
        .unwrap()
        .join("db2/active/AdventureJournal.csv");
    let catalog = load_catalog(&conn, &criteria, &journal_csv);
    assert!(matches!(catalog, AdventureCatalog::Ready(_)), "{catalog:?}");
    h.app.insert_resource(criteria);
    h.app.insert_resource(catalog);
    h.app.insert_resource(crate::maps::ServerClock {
        fixed_unix: Some(now),
        offset_secs: 0,
    });
    h.app
        .insert_resource(crate::guild_bank::DailyReset { hour_utc: 15 });
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(shared::components::Level(80));
}

#[test]
fn adventure_guide_month_refresh_never_waits_for_a_busy_durable_writer() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let now = Utc
        .with_ymd_and_hms(2026, 10, 15, 16, 0, 0)
        .single()
        .unwrap()
        .timestamp();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Guidesave").unwrap();
    prepare_real_catalog(&mut h, now);
    let connection = h.connection;
    let baseline = settle_month_for_connection(h.app.world_mut(), connection).unwrap();
    let held = h
        .app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        held.abort().unwrap();
    });
    let started = std::time::Instant::now();
    let refreshed = settle_month_for_connection(h.app.world_mut(), connection).unwrap();
    let elapsed = started.elapsed();
    release.join().unwrap();
    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "Guide month settlement tick waited for writer: {elapsed:?}"
    );
    assert_eq!(refreshed, baseline);
}

#[test]
fn suggested_provider_uses_existing_earned_achievement_authority_for_bonus() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let now = Utc
        .with_ymd_and_hms(2026, 10, 15, 16, 0, 0)
        .single()
        .unwrap()
        .timestamp();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Journalbonus").unwrap();
    attach_requests(&mut h);
    prepare_real_catalog(&mut h, now);
    let unearned = request(&mut h, AdventureGuideRequest::Query);
    let position = |snapshot: &AdventureGuideSnapshot, id| {
        snapshot
            .suggestions
            .iter()
            .position(|row| row.id == id)
            .unwrap_or_else(|| panic!("Eligible authored suggestion {id} missing"))
    };
    // Actual Random Battleground27 receives bonus4 from PlayerCondition33559 / achievement39935.
    assert!(position(&unearned, 27) > position(&unearned, 852));
    h.app
        .world_mut()
        .get_mut::<PlayerAchievementState>(h.player)
        .unwrap()
        .earned_ids
        .insert(39935);
    let earned = request(&mut h, AdventureGuideRequest::Query);
    assert!(position(&earned, 27) < position(&earned, 852));
    assert!(
        earned
            .suggestions
            .iter()
            .all(|row| row.button_text.is_empty())
    );
}

#[test]
fn adventure_guide_month_utc_boundary_and_leap_year_follow_reset_hour() {
    let at = |y, m, d, h| {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0)
            .single()
            .unwrap()
            .timestamp()
    };
    assert_eq!(month::month_at(at(2026, 11, 1, 14), 15, 0).month, 10);
    let november = month::month_at(at(2026, 11, 1, 15), 15, 0);
    assert_eq!(
        (november.month, november.start_unix, november.end_unix),
        (11, at(2026, 11, 1, 15), at(2026, 12, 1, 15))
    );
    let february = month::month_at(at(2024, 2, 29, 16), 15, 0);
    assert_eq!(february.end_unix - february.start_unix, 29 * 86400);
    assert_eq!(month::month_at(at(2027, 1, 1, 14), 15, 0).year, 2026);
}

#[test]
fn adventure_guide_real_rows_unavailable_census_tracking_and_monthly_persistence() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let now = Utc
        .with_ymd_and_hms(2026, 10, 11, 16, 0, 0)
        .single()
        .unwrap()
        .timestamp();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Journalrealm").unwrap();
    attach_requests(&mut h);
    prepare_real_catalog(&mut h, now);
    let snapshot = request(&mut h, AdventureGuideRequest::Query);
    assert!(snapshot.source_error.is_none());
    assert_eq!(snapshot.activities.len(), 945);
    assert_eq!(snapshot.month.month, 10);
    let quests = snapshot.activities.iter().find(|a| a.id == 3).unwrap();
    assert_eq!((&*quests.name, quests.points), ("Complete 30 Quests", 200));
    assert!(matches!(
        quests.progress,
        PerksActivityProgress::Unavailable { .. }
    ));
    let census = &snapshot.suggestion_census;
    println!(
        "suggestion census {census:?}; visible {}",
        snapshot.suggestions.len()
    );
    let journal_csv = crate::test_data::world_db_path()
        .parent()
        .unwrap()
        .join("db2/active/AdventureJournal.csv");
    let source_rows = csv::Reader::from_path(journal_csv)
        .unwrap()
        .records()
        .map(Result::unwrap)
        .count();
    assert_eq!(census.source_rows as usize, source_rows);
    assert_eq!(
        census.source_rows,
        census.unsupported_conditions
            + census.failed_conditions
            + snapshot.suggestions.len() as u32
    );
    assert!(census.unsupported_conditions > 0);
    assert!(census.unsupported_actions <= snapshot.suggestions.len() as u32);
    assert!(
        snapshot
            .suggestions
            .iter()
            .all(|row| row.button_text.is_empty())
    );
    let tracked = request(
        &mut h,
        AdventureGuideRequest::TrackActivity {
            activity_id: 3,
            tracked: true,
        },
    );
    assert!(
        tracked
            .activities
            .iter()
            .find(|a| a.id == 3)
            .unwrap()
            .tracked
    );
    let character = h.character_id;
    drop(h);
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Journalalt").unwrap();
    h.play_as(character);
    assert_eq!(h.character_id, character);
    attach_requests(&mut h);
    prepare_real_catalog(&mut h, now);
    assert!(
        request(&mut h, AdventureGuideRequest::Query)
            .activities
            .iter()
            .find(|a| a.id == 3)
            .unwrap()
            .tracked
    );
    h.app
        .world_mut()
        .resource_mut::<crate::maps::ServerClock>()
        .fixed_unix = Some(
        Utc.with_ymd_and_hms(2026, 11, 1, 15, 0, 0)
            .single()
            .unwrap()
            .timestamp(),
    );
    let reset = request(&mut h, AdventureGuideRequest::Query);
    assert_eq!(reset.month.month, 11);
    assert!(reset.activities.iter().find(|a| a.id == 3).unwrap().tracked);
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .monthly
            .as_ref()
            .unwrap()
            .counters
            .is_empty()
    );
}

#[test]
fn adventure_guide_actual_creature_credit_is_once_persistent_and_reset_by_month() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let now = Utc
        .with_ymd_and_hms(2026, 10, 11, 16, 0, 0)
        .single()
        .unwrap()
        .timestamp();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Guidehunter").unwrap();
    attach_requests(&mut h);
    prepare_real_catalog(&mut h, now);
    request(&mut h, AdventureGuideRequest::Query);
    let source = Connection::open_with_flags(
        std::env::var_os("ADVENTURE_GUIDE_WORLD_COPY").unwrap(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let trees = h
        .app
        .world()
        .resource::<AchievementCatalog>()
        .criterion_tree_ids(151404);
    let (criterion,asset)=trees.iter().find_map(|id| source.query_row("SELECT c.ID,c.Asset FROM criteria_tree t JOIN criteria c ON c.ID=t.CriteriaID WHERE t.ID=?1 AND c.Type=0",[id],|r|Ok((r.get::<_,u32>(0)?,r.get::<_,u32>(1)?))).ok()).expect("authentic Colossus kill criterion");
    // The realm's Wrath creature catalog does not contain these Retail event NPCs.
    // Death credit matches the authentic criterion Asset, not the display name.
    let name = "Colossus".to_string();
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(crate::maps::InMap::continent(1));
    let creature = h
        .app
        .world_mut()
        .spawn((
            shared::components::Npc {
                template_id: asset,
                name,
            },
            crate::maps::InMap::continent(1),
        ))
        .id();
    crate::achievement::record_creature_death(h.app.world_mut(), creature, Some(h.player));
    let snapshot = request(&mut h, AdventureGuideRequest::Query);
    let colossus = snapshot.activities.iter().find(|a| a.id == 498).unwrap();
    assert!(matches!(
        colossus.progress,
        PerksActivityProgress::Available {
            current: 1,
            completed: true,
            ..
        }
    ));
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.player)
        .unwrap();
    assert_eq!(
        state.monthly.as_ref().unwrap().counters[&criterion],
        1,
        "shared criteria across activities must not double credit one death"
    );
    let credited_character = h.character_id;
    drop(h);
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Guidehuntalt").unwrap();
    assert_ne!(h.character_id, credited_character);
    attach_requests(&mut h);
    prepare_real_catalog(&mut h, now);
    let restored = request(&mut h, AdventureGuideRequest::Query);
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .monthly
            .as_ref()
            .unwrap()
            .counters
            .is_empty(),
        "new character has no kill contribution of its own"
    );
    assert!(matches!(
        restored
            .activities
            .iter()
            .find(|a| a.id == 498)
            .unwrap()
            .progress,
        PerksActivityProgress::Available {
            current: 1,
            completed: true,
            ..
        }
    ));
    h.app
        .world_mut()
        .resource_mut::<crate::maps::ServerClock>()
        .fixed_unix = Some(
        Utc.with_ymd_and_hms(2026, 11, 1, 15, 0, 0)
            .single()
            .unwrap()
            .timestamp(),
    );
    let reset = request(&mut h, AdventureGuideRequest::Query);
    assert!(matches!(
        reset
            .activities
            .iter()
            .find(|a| a.id == 498)
            .unwrap()
            .progress,
        PerksActivityProgress::Available {
            current: 0,
            completed: false,
            ..
        }
    ));
}

#[test]
fn adventure_guide_query_replies_instead_of_inventing_missing_catalog_content() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Journalrealm").unwrap();
    AdventureGuidePlugin.build(&mut h.app);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<AdventureGuideRequest>::default(),
        MessageReceiver::<AdventureGuideRequest>::default(),
        MessageSender::<AdventureGuideSnapshot>::default(),
        MessageReceiver::<AdventureGuideSnapshot>::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<MessageSender<AdventureGuideRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(AdventureGuideRequest::Query);
    let mut replies = Vec::new();
    for _ in 0..10 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    assert_eq!(
        replies.len(),
        1,
        "realm-backed journal request must actually receive a response"
    );
    assert!(replies[0].source_error.is_some());
    assert!(replies[0].activities.is_empty());
    assert!(replies[0].suggestions.is_empty());
}
