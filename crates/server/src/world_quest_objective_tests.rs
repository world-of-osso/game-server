//! Authored optional/hidden/weighted requirements, independent of legacy template slots.
use crate::persistence::quest_store::ActiveQuestData;
use crate::quest_data::{ObjectiveDef, ObjectiveKind, QuestTemplate};
use crate::quest_rules::{objective_counts, objectives_done};
use crate::world_quest_objectives::ObjectiveRule;

#[test]
fn world_quest_weighted_progress_uses_hidden_counts_and_optional_rows_never_block() {
    // TDB76587:207974 weight9,207975 weight14, optional hidden marker207971,
    // progress-bar type15 and mandatory completion credit214539.
    let quest = QuestTemplate {
        id: 76587,
        quest_info_modifiers: 0x100,
        objectives: vec![
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 207971,
                required: 1,
                text: "Hidden start".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::ProgressBar,
                object_id: 0,
                required: 100,
                text: "Close Time Rifts".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 207974,
                required: 12,
                text: "Contributor".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 207975,
                required: 8,
                text: "Other contributor".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 214539,
                required: 1,
                text: "Complete the event".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 42,
                required: 1,
                text: "Optional visible".into(),
            },
        ],
        world_objective_rules: Some(vec![
            ObjectiveRule {
                flags: 28,
                flags2: 0,
                weight: 0.,
            },
            ObjectiveRule {
                flags: 2,
                flags2: 0,
                weight: 0.,
            },
            ObjectiveRule {
                flags: 92,
                flags2: 0,
                weight: 9.,
            },
            ObjectiveRule {
                flags: 92,
                flags2: 0,
                weight: 14.,
            },
            ObjectiveRule::default(),
            ObjectiveRule {
                flags: 4,
                flags2: 0,
                weight: 0.,
            },
        ]),
        ..Default::default()
    };
    let mut active = ActiveQuestData {
        quest_id: 76587,
        counts: vec![0, 0, 10, 0, 0, 0],
        ..Default::default()
    };
    let counts = objective_counts(&quest, &active, |_| 0);
    assert_eq!(counts[1], 90);
    assert!(!objectives_done(&quest, &counts));
    active.counts[2] = 12;
    let counts = objective_counts(&quest, &active, |_| 0);
    assert_eq!(counts[1], 100);
    assert!(
        !objectives_done(&quest, &counts),
        "mandatory completion credit still required"
    );
    active.counts[4] = 1;
    let counts = objective_counts(&quest, &active, |_| 0);
    assert!(
        objectives_done(&quest, &counts),
        "optional/contributor requirements cannot block completed bar"
    );
    let view = crate::quest_view::objective_snapshots(&quest, &counts);
    assert_eq!(view.len(), 3);
    assert_eq!((view[0].current, view[0].required), (100, 100));
    assert!(view[2].optional);
}

#[test]
fn world_quest_sequence_and_contributor_bar_prerequisites_block_premature_credit() {
    let quest = QuestTemplate {
        id: 82531,
        objectives: vec![
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 226416,
                required: 1,
                text: "Appropriate a Skiff".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::ProgressBar,
                object_id: 0,
                required: 100,
                text: "Progress".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 227724,
                required: 10,
                text: "Contributor".into(),
            },
        ],
        world_objective_rules: Some(vec![
            ObjectiveRule::default(),
            ObjectiveRule {
                flags: 2,
                flags2: 0,
                weight: 0.,
            },
            ObjectiveRule {
                flags: 92,
                flags2: 0,
                weight: 10.,
            },
        ]),
        ..Default::default()
    };
    let mut db = crate::quest_data::QuestDb::default();
    db.quests.insert(82531, quest);
    let mut log = crate::persistence::quest_store::QuestLogData {
        active: vec![ActiveQuestData {
            quest_id: 82531,
            counts: vec![0; 3],
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(!crate::quest_rules::credit_kill(&db, &mut log, 227724));
    assert_eq!(log.active(82531).unwrap().counts, vec![0, 0, 0]);
    assert!(crate::quest_rules::credit_kill(&db, &mut log, 226416));
    for _ in 0..10 {
        assert!(crate::quest_rules::credit_kill(&db, &mut log, 227724));
    }
    assert!(
        !crate::quest_rules::credit_kill(&db, &mut log, 227724),
        "completed bar rejects extra contributor credit"
    );
    let counts = objective_counts(&db.quests[&82531], log.active(82531).unwrap(), |_| 0);
    assert_eq!(counts[1], 100);
    assert!(objectives_done(&db.quests[&82531], &counts));
}

#[test]
fn world_quest_sequence_advances_after_a_derived_bar_completes() {
    let quest = QuestTemplate {
        id: 1,
        objectives: vec![
            ObjectiveDef {
                kind: ObjectiveKind::ProgressBar,
                object_id: 0,
                required: 100,
                text: "Bar".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 42,
                required: 10,
                text: "Contributor".into(),
            },
            ObjectiveDef {
                kind: ObjectiveKind::Monster,
                object_id: 43,
                required: 1,
                text: "Next phase".into(),
            },
        ],
        world_objective_rules: Some(vec![
            ObjectiveRule {
                flags: 2,
                ..Default::default()
            },
            ObjectiveRule {
                flags: 64,
                weight: 10.,
                ..Default::default()
            },
            ObjectiveRule {
                flags: 2,
                ..Default::default()
            },
        ]),
        ..Default::default()
    };
    let mut db = crate::quest_data::QuestDb::default();
    db.quests.insert(1, quest);
    let mut log = crate::persistence::quest_store::QuestLogData {
        active: vec![ActiveQuestData {
            quest_id: 1,
            counts: vec![0; 3],
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(!crate::quest_rules::credit_kill(&db, &mut log, 43));
    for _ in 0..10 {
        assert!(crate::quest_rules::credit_kill(&db, &mut log, 42));
    }
    assert!(crate::quest_rules::credit_kill(&db, &mut log, 43));
    assert!(objectives_done(
        &db.quests[&1],
        &objective_counts(&db.quests[&1], log.active(1).unwrap(), |_| 0)
    ));
}

#[test]
fn world_quest_dynamic_objective_load_preserves_order_flags_names_and_weights() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_world_quest (quest_id INTEGER);INSERT INTO content_world_quest VALUES(76587);
        CREATE TABLE content_world_quest_objective (quest_id INTEGER,objective_id INTEGER,ordinal INTEGER,kind INTEGER,object_id INTEGER,amount INTEGER,flags INTEGER,weight REAL,description TEXT,flags2 INTEGER DEFAULT 0);
        INSERT INTO content_world_quest_objective VALUES (76587,1,1,15,0,1,2,0,'Close Time Rifts',0),(76587,2,0,0,207974,12,92,9,'',0);
        CREATE TABLE tdb_creature_template(entry INTEGER,name TEXT);INSERT INTO tdb_creature_template VALUES(207974,'Time Rift Invader');
        CREATE TABLE content_item(ID INTEGER,name TEXT);").unwrap();
    let mut db = crate::quest_data::QuestDb::default();
    db.quests.insert(
        76587,
        QuestTemplate {
            id: 76587,
            ..Default::default()
        },
    );
    crate::world_quest_objectives::load(&conn, &mut db).unwrap();
    let quest = &db.quests[&76587];
    assert_eq!(quest.objectives.len(), 2);
    assert_eq!(quest.objectives[0].text, "Time Rift Invader slain");
    assert_eq!(quest.objectives[1].kind, ObjectiveKind::ProgressBar);
    assert_eq!(quest.objectives[1].required, 100);
    assert_eq!(
        quest.world_objective_rules.as_ref().unwrap()[0],
        ObjectiveRule {
            flags: 92,
            flags2: 0,
            weight: 9.
        }
    );
}

#[test]
fn world_quest_bound_item_uses_persisted_credit_not_bag_count() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_world_quest (quest_id INTEGER); INSERT INTO content_world_quest VALUES(80295);
        CREATE TABLE content_world_quest_objective (quest_id INTEGER,kind INTEGER,object_id INTEGER,amount INTEGER,flags INTEGER,flags2 INTEGER,weight REAL,description TEXT,ordinal INTEGER);
        INSERT INTO content_world_quest_objective VALUES(80295,1,217343,20,0,1,0,'Collect',0);
        CREATE TABLE tdb_creature_template(entry INTEGER,name TEXT); CREATE TABLE content_item(ID INTEGER,name TEXT);").unwrap();
    let mut db = crate::quest_data::QuestDb::default();
    db.quests.insert(
        80295,
        QuestTemplate {
            id: 80295,
            ..Default::default()
        },
    );
    crate::world_quest_objectives::load(&conn, &mut db).unwrap();
    let active = ActiveQuestData {
        quest_id: 80295,
        counts: vec![4],
        ..Default::default()
    };
    let quest = &db.quests[&80295];
    assert_eq!(objective_counts(quest, &active, |_| 20), vec![4]);
    assert!(!objectives_done(
        quest,
        &objective_counts(quest, &active, |_| 20)
    ));
    assert_eq!(objective_counts(quest, &active, |_| 0), vec![4]);
}

#[test]
fn world_quest_area_trigger_enter_zero_amount_is_an_unset_boolean_not_free_completion() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_world_quest(quest_id INTEGER); INSERT INTO content_world_quest VALUES(86610);
        CREATE TABLE content_world_quest_objective(quest_id INTEGER,kind INTEGER,object_id INTEGER,amount INTEGER,flags INTEGER,flags2 INTEGER,weight REAL,description TEXT,ordinal INTEGER);
        INSERT INTO content_world_quest_objective VALUES(86610,19,35987,0,0,0,0,'The Oasis mail delivered',0);
        CREATE TABLE tdb_creature_template(entry INTEGER,name TEXT); CREATE TABLE content_item(ID INTEGER,name TEXT);").unwrap();
    let mut db = crate::quest_data::QuestDb::default();
    db.quests.insert(
        86610,
        QuestTemplate {
            id: 86610,
            ..Default::default()
        },
    );
    crate::world_quest_objectives::load(&conn, &mut db).unwrap();
    let mut log = crate::persistence::quest_store::QuestLogData {
        active: vec![ActiveQuestData {
            quest_id: 86610,
            counts: vec![0],
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(
        !objectives_done(&db.quests[&86610], &[0]),
        "Amount0 is not a satisfied task trigger"
    );
    assert!(crate::quest_rules::credit_objective(
        &db,
        &mut log,
        ObjectiveKind::AreaTriggerEnter,
        35987
    ));
    assert_eq!(log.active(86610).unwrap().counts, vec![1]);
    assert!(objectives_done(&db.quests[&86610], &[1]));
    assert!(!crate::quest_rules::credit_objective(
        &db,
        &mut log,
        ObjectiveKind::AreaTriggerEnter,
        35987
    ));
    assert!(!crate::quest_rules::credit_objective(
        &db,
        &mut log,
        ObjectiveKind::Monster,
        35987
    ));
}

#[test]
fn world_quest_hidden_mandatory_requirement_still_blocks_completion() {
    let quest = QuestTemplate {
        id: 1,
        objectives: vec![ObjectiveDef {
            kind: ObjectiveKind::Monster,
            object_id: 42,
            required: 1,
            text: "Hidden mandatory".into(),
        }],
        world_objective_rules: Some(vec![ObjectiveRule {
            flags: 8,
            flags2: 0,
            weight: 0.,
        }]),
        ..Default::default()
    };
    assert!(!objectives_done(&quest, &[0]));
    assert!(objectives_done(&quest, &[1]));
    assert!(crate::quest_view::objective_snapshots(&quest, &[1]).is_empty());
}
