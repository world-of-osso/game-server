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
                weight: 0.,
            },
            ObjectiveRule {
                flags: 2,
                weight: 0.,
            },
            ObjectiveRule {
                flags: 92,
                weight: 9.,
            },
            ObjectiveRule {
                flags: 92,
                weight: 14.,
            },
            ObjectiveRule::default(),
            ObjectiveRule {
                flags: 4,
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
            weight: 0.,
        }]),
        ..Default::default()
    };
    assert!(!objectives_done(&quest, &[0]));
    assert!(objectives_done(&quest, &[1]));
    assert!(crate::quest_view::objective_snapshots(&quest, &[1]).is_empty());
}
