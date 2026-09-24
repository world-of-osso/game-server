//! Rules against real world.db rows (Northshire); each test skips when the DB is absent.
use shared::protocol::{QuestFailedReason, QuestGiverQuestState, QuestGiverStatus};

use super::*;
use crate::quest_data::test_quest_db;

const DEPUTY_WILLEM: u32 = 823;
const MARSHAL_MCBRIDE: u32 = 197;
const A_THREAT_WITHIN: u32 = 783;
const KOBOLD_CAMP_CLEANUP: u32 = 7;
const EAGAN_PELTSKINNER: u32 = 5261;
const KOBOLD_VERMIN: u32 = 6;
const KOBOLD_WORKER: u32 = 257;

const HUMAN_WARRIOR: QuestPlayer = QuestPlayer {
    level: 1,
    race: 1,
    class: 1,
};

fn active(quest_id: u32) -> ActiveQuestData {
    ActiveQuestData {
        quest_id,
        counts: Vec::new(),
    }
}

fn log_with(active_quests: &[u32], rewarded: &[u32]) -> QuestLogData {
    let mut rewarded = rewarded.to_vec();
    rewarded.sort_unstable();
    QuestLogData {
        active: active_quests.iter().copied().map(active).collect(),
        rewarded,
        watched: Vec::new(),
    }
}

fn no_items(_: u32) -> u32 {
    0
}

#[test]
fn willem_offers_a_threat_within_but_not_its_follow_ups() {
    let Some(db) = test_quest_db() else { return };
    let log = QuestLogData::default();

    let list = giver_quest_list(db, &log, HUMAN_WARRIOR, DEPUTY_WILLEM, no_items);

    assert_eq!(
        list.iter()
            .map(|entry| (entry.quest_id, entry.title.as_str(), entry.state))
            .collect::<Vec<_>>(),
        vec![(
            A_THREAT_WITHIN,
            "A Threat Within",
            QuestGiverQuestState::Available
        )]
    );
    assert_eq!(
        giver_status(db, &log, HUMAN_WARRIOR, DEPUTY_WILLEM, no_items),
        QuestGiverStatus::Available
    );
}

#[test]
fn kobold_camp_cleanup_needs_a_threat_within_turned_in() {
    let Some(db) = test_quest_db() else { return };
    let quest = &db.quests[&KOBOLD_CAMP_CLEANUP];

    let fresh = can_take(db, &QuestLogData::default(), HUMAN_WARRIOR, quest);
    let in_log = can_take(db, &log_with(&[A_THREAT_WITHIN], &[]), HUMAN_WARRIOR, quest);
    let rewarded = can_take(db, &log_with(&[], &[A_THREAT_WITHIN]), HUMAN_WARRIOR, quest);

    assert_eq!(fresh, Err(QuestFailedReason::DontHaveRequirement));
    assert_eq!(in_log, Err(QuestFailedReason::DontHaveRequirement));
    assert_eq!(rewarded, Ok(()));
}

#[test]
fn mcbride_marks_reward_for_a_threat_within_and_then_offers_kobold_camp_cleanup() {
    let Some(db) = test_quest_db() else { return };
    let accepted = log_with(&[A_THREAT_WITHIN], &[]);
    let turned_in = log_with(&[], &[A_THREAT_WITHIN]);

    assert_eq!(
        giver_status(db, &accepted, HUMAN_WARRIOR, MARSHAL_MCBRIDE, no_items),
        QuestGiverStatus::Reward
    );
    assert_eq!(
        giver_status(db, &turned_in, HUMAN_WARRIOR, MARSHAL_MCBRIDE, no_items),
        QuestGiverStatus::Available
    );
    let offered: Vec<u32> =
        giver_quest_list(db, &turned_in, HUMAN_WARRIOR, DEPUTY_WILLEM, no_items)
            .iter()
            .map(|entry| entry.quest_id)
            .collect();
    assert!(offered.contains(&EAGAN_PELTSKINNER));
    assert!(!offered.contains(&A_THREAT_WITHIN));
}

#[test]
fn eight_kobold_vermin_kills_complete_kobold_camp_cleanup() {
    let Some(db) = test_quest_db() else { return };
    let mut log = log_with(&[KOBOLD_CAMP_CLEANUP], &[A_THREAT_WITHIN]);

    assert!(!credit_kill(db, &mut log, KOBOLD_WORKER));
    for _ in 0..7 {
        assert!(credit_kill(db, &mut log, KOBOLD_VERMIN));
    }
    assert!(!is_complete_in_log(db, &log, KOBOLD_CAMP_CLEANUP, no_items));
    assert!(credit_kill(db, &mut log, KOBOLD_VERMIN));
    assert!(!credit_kill(db, &mut log, KOBOLD_VERMIN));

    assert_eq!(log.active(KOBOLD_CAMP_CLEANUP).unwrap().counts, vec![8]);
    assert!(is_complete_in_log(db, &log, KOBOLD_CAMP_CLEANUP, no_items));
    assert_eq!(
        giver_status(db, &log, HUMAN_WARRIOR, MARSHAL_MCBRIDE, no_items),
        QuestGiverStatus::Reward
    );
}

#[test]
fn level_ten_sees_a_threat_within_as_trivial() {
    let Some(db) = test_quest_db() else { return };
    let player = QuestPlayer {
        level: 10,
        ..HUMAN_WARRIOR
    };

    assert_eq!(
        giver_status(
            db,
            &QuestLogData::default(),
            player,
            DEPUTY_WILLEM,
            no_items
        ),
        QuestGiverStatus::LowLevelAvailable
    );
}

#[test]
fn orc_cannot_take_alliance_quest_and_log_caps_at_35() {
    let Some(db) = test_quest_db() else { return };
    let quest = &db.quests[&A_THREAT_WITHIN];
    let orc = QuestPlayer {
        race: 2,
        ..HUMAN_WARRIOR
    };
    let full_ids: Vec<u32> = (1_000_000..1_000_035).collect();

    assert_eq!(
        can_take(db, &QuestLogData::default(), orc, quest),
        Err(QuestFailedReason::WrongRace)
    );
    assert_eq!(
        can_take(db, &log_with(&full_ids, &[]), HUMAN_WARRIOR, quest),
        Err(QuestFailedReason::QuestLogFull)
    );
}
