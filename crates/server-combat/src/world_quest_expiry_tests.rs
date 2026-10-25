//! Persistence/clock boundary, independent of NPC-given quest acceptance.
use crate::auth::InWorld;
use crate::maps::difficulty::ServerClock;
use crate::persistence::{
    GameDatabase,
    quest_store::{ActiveQuestData, QuestLogData, load_quest_log, save_quest_log},
};
use crate::quest::QuestDirty;
use crate::quest_data::{QuestDb, QuestTemplate};
use crate::world_quests::{Definition, Rotation};
use bevy::prelude::*;

fn expiry_app(path: &std::path::Path, now: i64) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(GameDatabase::new(redb::Database::create(path).unwrap()));
    app.insert_resource(ServerClock {
        fixed_unix: Some(now),
        ..Default::default()
    });
    app.insert_resource(Rotation {
        epoch: 1000,
        ..Default::default()
    });
    app.init_resource::<QuestDirty>();
    let mut quests = QuestDb {
        world_quests: vec![Definition {
            quest_id: 52847,
            zone_id: 9734,
            duration: 604800,
        }],
        ..Default::default()
    };
    quests.quests.insert(
        52847,
        QuestTemplate {
            id: 52847,
            title: "Doom's Howl".into(),
            quest_info_modifiers: 0x100,
            ..Default::default()
        },
    );
    quests.quests.insert(
        62,
        QuestTemplate {
            id: 62,
            time_allowed: 600,
            ..Default::default()
        },
    );
    app.insert_resource(quests);
    app.world_mut().spawn(InWorld { character_id: 11 });
    app.add_systems(Update, crate::quest_time::refresh);
    app
}

#[test]
fn world_quest_expiry_removes_persisted_task_and_watch_at_exact_window_end_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("world-quest.redb");
    let mut app = expiry_app(&path, 605799);
    let log = QuestLogData {
        active: vec![
            ActiveQuestData {
                quest_id: 52847,
                counts: vec![1],
                expires_at: Some(605800),
                failed: false,
            },
            ActiveQuestData {
                quest_id: 62,
                counts: vec![0],
                expires_at: Some(606000),
                failed: false,
            },
        ],
        watched: vec![52847, 62],
        ..Default::default()
    };
    save_quest_log(&app.world().resource::<GameDatabase>().db, 11, &log);
    app.update();
    assert_eq!(
        load_quest_log(&app.world().resource::<GameDatabase>().db, 11),
        log
    );
    assert_eq!(
        app.world().resource::<QuestDb>().world_quest_windows[&52847].expires_at,
        605800
    );
    drop(app);

    let mut restarted = expiry_app(&path, 605800);
    restarted.update();
    let expired = load_quest_log(&restarted.world().resource::<GameDatabase>().db, 11);
    assert!(expired.active(52847).is_none());
    assert_eq!(expired.watched, vec![62]);
    assert!(!expired.active(62).unwrap().failed);
    assert!(expired.completed_at.is_empty());
    assert!(expired.rewarded.is_empty());
    assert!(restarted.world().resource::<QuestDirty>().0.contains(&11));
    assert!(
        restarted
            .world()
            .resource::<QuestDb>()
            .world_quest_windows
            .is_empty()
    );
    restarted.update();
    assert_eq!(
        load_quest_log(&restarted.world().resource::<GameDatabase>().db, 11),
        expired
    );
    drop(restarted);

    let mut final_restart = expiry_app(&path, 605801);
    final_restart.update();
    assert_eq!(
        load_quest_log(&final_restart.world().resource::<GameDatabase>().db, 11),
        expired
    );
}

#[test]
fn world_quest_area_loading_reports_missing_source_z_and_reimport_command() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE content_world_quest_area (quest_id INTEGER, blob_id INTEGER,
         map_id INTEGER, ui_map_id INTEGER, point_index INTEGER, x REAL, y REAL);
         INSERT INTO content_world_quest_area VALUES (82521,514681,2601,2256,0,-2268,-1960);",
    )
    .unwrap();
    let error = crate::world_quests::load_areas(&conn)
        .unwrap_err()
        .to_string();
    assert!(error.contains("content_world_quest_area.z"), "{error}");
    assert!(error.contains("scripts/import_world_quests.py"), "{error}");
    assert!(error.contains("--poi-db2-dir"), "{error}");
}

#[test]
fn world_quest_definition_loading_never_supplies_a_guessed_duration() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    assert!(
        crate::world_quests::load_definitions(&conn)
            .unwrap()
            .is_empty()
    );
    conn.execute_batch("CREATE TABLE content_world_quest (quest_id INTEGER,zone_id INTEGER,duration_seconds INTEGER);
        INSERT INTO content_world_quest VALUES (52847,9734,604800),(54895,-656,604800);").unwrap();
    assert_eq!(
        crate::world_quests::load_definitions(&conn).unwrap(),
        vec![
            Definition {
                quest_id: 52847,
                zone_id: 9734,
                duration: 604800
            },
            Definition {
                quest_id: 54895,
                zone_id: -656,
                duration: 604800
            },
        ]
    );
    conn.execute(
        "UPDATE content_world_quest SET duration_seconds=0 WHERE quest_id=52847",
        [],
    )
    .unwrap();
    assert!(crate::world_quests::load_definitions(&conn).is_err());
}
