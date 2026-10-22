use crate::persistence::housing_store::BUILD_KEY;
use crate::persistence::quest_store::{ActiveQuestData, QuestLogData};
use crate::quest_data::{ObjectiveKind, QuestDb, QuestTemplate};

#[test]
fn db2_follow_install_housing_quests_read_the_declared_next_build_key() {
    let (source, mut quests) = fixture();
    let key = "334a36cf5f3c32a5f7a167405487fa14";
    source
        .execute(
            "UPDATE housing_catalog_build SET build_key=?, build='12.1.5.70391'",
            [key],
        )
        .unwrap();
    source
        .execute("UPDATE housing_catalog_record SET build_key=?", [key])
        .unwrap();
    crate::housing_quests::load(&source, &mut quests).unwrap();
    let quest = &quests.quests[&92572];
    assert!(!quest.completion_blocked);
    assert_eq!(quest.objectives[0].object_id, 216882);
}

fn fixture() -> (rusqlite::Connection, QuestDb) {
    let source = crate::housing_catalog::tests::catalog_fixture();
    source
        .execute(
            "INSERT INTO housing_catalog_record VALUES (?,'HouseDecor',9247,?)",
            rusqlite::params![BUILD_KEY, r#"{"ItemID":"253173"}"#],
        )
        .unwrap();
    // Exact pinned TDB1210 + local Retail Criteria/CriteriaTree rows.
    source.execute_batch("CREATE TABLE tdb_quest_objectives (ID INTEGER,QuestID INTEGER,Type INTEGER,ObjectID INTEGER,Amount INTEGER,Flags INTEGER,Flags2 INTEGER,Description TEXT,\"Order\" INTEGER);
        INSERT INTO tdb_quest_objectives VALUES (465162,92572,14,216882,1,0,0,'Purchase Earthen Storage Crate decor',0);
        CREATE TABLE criteria_tree (ID INTEGER,Parent INTEGER,Amount INTEGER,Operator INTEGER,CriteriaID INTEGER,Flags INTEGER);
        INSERT INTO criteria_tree VALUES (216882,0,0,4,0,0),(216883,216882,0,0,109153,0);
        CREATE TABLE criteria (ID INTEGER,Type INTEGER,Asset INTEGER,ModifierTreeId INTEGER,StartEvent INTEGER,StartTimer INTEGER,FailEvent INTEGER,Flags INTEGER,EligibilityWorldStateID INTEGER,EligibilityWorldStateValue INTEGER);
        INSERT INTO criteria VALUES (109153,36,253168,0,0,0,0,32,0,0);").unwrap();
    let mut quests = QuestDb::default();
    quests.quests.insert(
        92572,
        QuestTemplate {
            id: 92572,
            reward_items: vec![(253173, 1)],
            completion_blocked: true,
            ..Default::default()
        },
    );
    (source, quests)
}

#[test]
fn housing_quest_uses_authored_acquisition_criteria_and_never_auto_completes_empty_legacy_objectives()
 {
    let (source, mut quests) = fixture();
    crate::housing_quests::load(&source, &mut quests).unwrap();
    let quest = &quests.quests[&92572];
    assert!(!quest.completion_blocked);
    assert_eq!(quest.objectives.len(), 1);
    assert_eq!(quest.objectives[0].kind, ObjectiveKind::CriteriaTree);
    assert_eq!(quest.objectives[0].object_id, 216882);
    let mut log = QuestLogData {
        active: vec![ActiveQuestData {
            quest_id: 92572,
            counts: vec![0],
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(!crate::quest_rules::is_complete_in_log(
        &quests,
        &log,
        92572,
        |_| 0
    ));
    assert!(!crate::housing_quests::credit_acquired_item(
        &quests, &mut log, 245259, 1
    ));
    assert!(crate::housing_quests::credit_acquired_item(
        &quests, &mut log, 253168, 1
    ));
    assert_eq!(log.active(92572).unwrap().counts, vec![1]);
    assert!(crate::quest_rules::is_complete_in_log(
        &quests,
        &log,
        92572,
        |_| 0
    ));
    // AcquireItem progress accumulates; normal later bag consumption is not a revoke.
    assert!(!crate::housing_quests::credit_acquired_item(
        &quests, &mut log, 253168, 1
    ));
    assert_eq!(quests.quests[&92572].reward_items, vec![(253173, 1)]);
}

#[test]
fn housing_quest_installed_quest_root_annotation_allows_only_matching_paid_item_credit() {
    let (source, mut quests) = fixture();
    // Local70448 CriteriaTree216882: IsForQuest0x1000; TC a352b1fa DBCEnums.h.
    source
        .execute("UPDATE criteria_tree SET Flags=4096 WHERE ID=216882", [])
        .unwrap();
    crate::housing_quests::load(&source, &mut quests).unwrap();
    assert!(
        !quests.quests[&92572].completion_blocked,
        "authored IsForQuest root metadata must not block Furniture Favor"
    );
    let mut log = QuestLogData {
        active: vec![ActiveQuestData {
            quest_id: 92572,
            counts: vec![0],
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(!crate::housing_quests::credit_acquired_item(
        &quests, &mut log, 245259, 1
    ));
    assert!(!crate::quest_rules::is_complete_in_log(
        &quests,
        &log,
        92572,
        |_| 0
    ));
    assert!(crate::housing_quests::credit_acquired_item(
        &quests, &mut log, 253168, 1
    ));
    assert_eq!(log.active(92572).unwrap().counts, vec![1]);
    assert!(crate::quest_rules::is_complete_in_log(
        &quests,
        &log,
        92572,
        |_| 0
    ));
    assert_eq!(quests.quests[&92572].reward_items, vec![(253173, 1)]);
}

#[test]
fn housing_quest_root_annotation_does_not_allow_faction_or_unknown_flags() {
    for flags in [0x1200, 0x1400, 0x9000] {
        let (source, mut quests) = fixture();
        source
            .execute("UPDATE criteria_tree SET Flags=? WHERE ID=216882", [flags])
            .unwrap();
        crate::housing_quests::load(&source, &mut quests).unwrap();
        assert!(quests.quests[&92572].completion_blocked, "flags{flags}");
        assert!(quests.quests[&92572].objectives.is_empty());
    }
}

#[test]
fn housing_quest_unknown_criteria_or_conditions_remain_blocked_without_a_replacement_objective() {
    let (source, mut quests) = fixture();
    source
        .execute("UPDATE criteria SET ModifierTreeId=700", [])
        .unwrap();
    crate::housing_quests::load(&source, &mut quests).unwrap();
    assert!(quests.quests[&92572].completion_blocked);
    source
        .execute("UPDATE criteria SET ModifierTreeId=0,Type=69", [])
        .unwrap();
    crate::housing_quests::load(&source, &mut quests).unwrap();
    assert!(quests.quests[&92572].completion_blocked);
}
