//! Rules against real world.db rows (Northshire); each test skips when the DB is absent.
use shared::protocol::{QuestFailedReason, QuestGiverQuestState, QuestGiverStatus};

use super::*;
use crate::quest_data::test_quest_db;

const DEPUTY_WILLEM: u32 = 823;
const MARSHAL_MCBRIDE: u32 = 197;
// Retail Northshire's warrior chain (TrinityCore; Cataclysm removed A Threat Within and
// Kobold Camp Cleanup): Beating Them Back! → Lions for Lambs → Join the Battle! →
// They Sent Assassins. ContentTuning 1476 (levels 1-10), Human-only race mask.
const BEATING_THEM_BACK: u32 = 28766;
const LIONS_FOR_LAMBS: u32 = 28774;
const JOIN_THE_BATTLE: u32 = 28789;
const THEY_SENT_ASSASSINS: u32 = 28797;
/// Beating Them Back!'s hunter variant (AllowableClasses 128).
const BEATING_THEM_BACK_HUNTER: u32 = 28757;
const BLACKROCK_WORG: u32 = 49871;
const BLACKROCK_SPY: u32 = 49874;

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
fn mcbride_offers_a_warrior_beating_them_back_but_not_its_follow_ups() {
    let Some(db) = test_quest_db() else { return };
    let log = QuestLogData::default();

    let list = giver_quest_list(db, &log, HUMAN_WARRIOR, MARSHAL_MCBRIDE, no_items);
    let offered: Vec<(u32, &str, QuestGiverQuestState)> = list
        .iter()
        .map(|entry| (entry.quest_id, entry.title.as_str(), entry.state))
        .collect();

    assert!(offered.contains(&(
        BEATING_THEM_BACK,
        "Beating Them Back!",
        QuestGiverQuestState::Available
    )));
    assert!(
        !offered
            .iter()
            .any(|(id, _, _)| [LIONS_FOR_LAMBS, BEATING_THEM_BACK_HUNTER].contains(id)),
        "{offered:?}"
    );
    assert_eq!(
        giver_status(db, &log, HUMAN_WARRIOR, MARSHAL_MCBRIDE, no_items),
        QuestGiverStatus::Available
    );
}

#[test]
fn lions_for_lambs_needs_beating_them_back_turned_in() {
    let Some(db) = test_quest_db() else { return };
    let quest = &db.quests[&LIONS_FOR_LAMBS];

    let fresh = can_take(db, &QuestLogData::default(), HUMAN_WARRIOR, quest);
    let in_log = can_take(
        db,
        &log_with(&[BEATING_THEM_BACK], &[]),
        HUMAN_WARRIOR,
        quest,
    );
    let rewarded = can_take(
        db,
        &log_with(&[], &[BEATING_THEM_BACK]),
        HUMAN_WARRIOR,
        quest,
    );

    assert_eq!(fresh, Err(QuestFailedReason::DontHaveRequirement));
    assert_eq!(in_log, Err(QuestFailedReason::DontHaveRequirement));
    assert_eq!(rewarded, Ok(()));
}

#[test]
fn willem_marks_reward_for_join_the_battle_and_then_offers_they_sent_assassins() {
    let Some(db) = test_quest_db() else { return };
    let accepted = log_with(&[JOIN_THE_BATTLE], &[BEATING_THEM_BACK, LIONS_FOR_LAMBS]);
    let turned_in = log_with(&[], &[BEATING_THEM_BACK, LIONS_FOR_LAMBS, JOIN_THE_BATTLE]);

    assert_eq!(
        giver_status(db, &accepted, HUMAN_WARRIOR, DEPUTY_WILLEM, no_items),
        QuestGiverStatus::Reward
    );
    assert_eq!(
        giver_status(db, &turned_in, HUMAN_WARRIOR, DEPUTY_WILLEM, no_items),
        QuestGiverStatus::Available
    );
    let offered: Vec<u32> =
        giver_quest_list(db, &turned_in, HUMAN_WARRIOR, DEPUTY_WILLEM, no_items)
            .iter()
            .map(|entry| entry.quest_id)
            .collect();
    assert!(offered.contains(&THEY_SENT_ASSASSINS));
    assert!(!offered.contains(&JOIN_THE_BATTLE));
}

#[test]
fn six_blackrock_worg_kills_complete_beating_them_back() {
    let Some(db) = test_quest_db() else { return };
    let mut log = log_with(&[BEATING_THEM_BACK], &[]);

    assert!(!credit_kill(db, &mut log, BLACKROCK_SPY));
    for _ in 0..5 {
        assert!(credit_kill(db, &mut log, BLACKROCK_WORG));
    }
    assert!(!is_complete_in_log(db, &log, BEATING_THEM_BACK, no_items));
    assert!(credit_kill(db, &mut log, BLACKROCK_WORG));
    assert!(!credit_kill(db, &mut log, BLACKROCK_WORG));

    assert_eq!(log.active(BEATING_THEM_BACK).unwrap().counts, vec![6]);
    assert!(is_complete_in_log(db, &log, BEATING_THEM_BACK, no_items));
    assert_eq!(
        giver_status(db, &log, HUMAN_WARRIOR, MARSHAL_MCBRIDE, no_items),
        QuestGiverStatus::Reward
    );
}

/// The quest scales to the player (QuestLevel -1): at level 10, ContentTuning
/// 1476's top, Retail still shows it as a normal quest.
#[test]
fn level_ten_sees_beating_them_back_as_a_normal_quest() {
    let Some(db) = test_quest_db() else { return };
    let player = QuestPlayer {
        level: 10,
        ..HUMAN_WARRIOR
    };

    assert_eq!(db.quests[&BEATING_THEM_BACK].level, -1);
    assert_eq!(
        giver_status(
            db,
            &QuestLogData::default(),
            player,
            MARSHAL_MCBRIDE,
            no_items
        ),
        QuestGiverStatus::Available
    );
}

#[test]
fn orc_cannot_take_alliance_quest_and_log_caps_at_35() {
    let Some(db) = test_quest_db() else { return };
    let quest = &db.quests[&BEATING_THEM_BACK];
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

/// Riverpaw Gnoll Bounty (11, Westfall): 8 Painted Gnoll Armbands.
const RIVERPAW_GNOLL_BOUNTY: u32 = 11;
const PAINTED_GNOLL_ARMBAND: u32 = 782;

#[test]
fn quest_needs_its_item_until_the_bags_hold_the_count() {
    let Some(db) = test_quest_db() else {
        return;
    };
    let log = log_with(&[RIVERPAW_GNOLL_BOUNTY], &[]);
    assert!(quest_needs_item(db, &log, PAINTED_GNOLL_ARMBAND, |_| 7));
    assert!(!quest_needs_item(db, &log, PAINTED_GNOLL_ARMBAND, |_| 8));
    assert!(!quest_needs_item(
        db,
        &log_with(&[], &[]),
        PAINTED_GNOLL_ARMBAND,
        no_items
    ));
    assert!(!quest_needs_item(db, &log, 755, no_items), "Melted Candle");
}

/// Quest::MoneyValue: QuestMoneyReward Difficulty_5 at the player's quest level
/// (ContentTuning 1476: levels 1-10) × RewardMoneyMultiplier 1.
#[test]
fn beating_them_back_pays_quest_money_for_the_players_level() {
    let Some(db) = test_quest_db() else { return };
    let quest = &db.quests[&BEATING_THEM_BACK];
    let at = |level| {
        quest_money(
            db,
            quest,
            QuestPlayer {
                level,
                ..HUMAN_WARRIOR
            },
        )
    };
    assert_eq!(at(1), 55);
    assert_eq!(at(10), 2000);
    assert_eq!(at(40), 2000, "capped at ContentTuning MaxLevel 10");
    assert_eq!(quest.reward_money, 2000, "RewardMoney keeps MaxMoneyValue");
}

fn scaled_quest(faction_group: u32) -> (QuestDb, QuestTemplate) {
    let mut db = QuestDb::default();
    db.money_rewards
        .insert(5, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90]);
    db.money_rewards
        .insert(20, [0, 100, 200, 300, 400, 500, 600, 700, 800, 900]);
    let quest = QuestTemplate {
        reward_money: 900,
        money_scaling: Some(crate::quest_data::QuestMoneyScaling {
            difficulty: 3,
            multiplier: 1.5,
            min_level: 5,
            max_level: 20,
            faction_group,
        }),
        ..Default::default()
    };
    (db, quest)
}

#[test]
fn below_its_minimum_a_quest_pays_its_minimum_levels_money_times_the_multiplier() {
    let (db, quest) = scaled_quest(0);
    // Level 1 < MinLevelWithDelta 5: QuestMoneyReward[5] 30 × 1.5.
    assert_eq!(quest_money(&db, &quest, HUMAN_WARRIOR), 45);
    // A level without a QuestMoneyReward row pays nothing.
    let player = QuestPlayer {
        level: 12,
        ..HUMAN_WARRIOR
    };
    assert_eq!(quest_money(&db, &quest, player), 0);
}

#[test]
fn player_outside_the_quests_faction_group_is_paid_at_its_maximum() {
    // Horde-scaled (5) quest: a Human is paid at MaxLevel 20, an Orc at its own level floor.
    let (db, quest) = scaled_quest(5);
    assert_eq!(quest_money(&db, &quest, HUMAN_WARRIOR), 450);
    let orc = QuestPlayer {
        race: 2,
        ..HUMAN_WARRIOR
    };
    assert_eq!(quest_money(&db, &quest, orc), 45);
}

#[test]
fn azerothcore_quest_pays_its_fixed_reward_money() {
    let quest = QuestTemplate {
        reward_money: -250,
        ..Default::default()
    };
    assert_eq!(
        quest_money(&QuestDb::default(), &quest, HUMAN_WARRIOR),
        -250
    );
}

#[test]
fn quest_complete_fills_the_vineyard_fires_but_needs_the_quest_in_the_log() {
    let Some(db) = test_quest_db() else { return };
    const EXTINGUISHING_HOPE: u32 = 26391;
    let mut log = log_with(&[EXTINGUISHING_HOPE], &[]);

    complete_counted_objectives(db, &mut log, EXTINGUISHING_HOPE).expect("in log");

    let quest = &db.quests[&EXTINGUISHING_HOPE];
    let counts = objective_counts(quest, log.active(EXTINGUISHING_HOPE).unwrap(), no_items);
    assert_eq!(counts, [8]);
    assert!(objectives_done(quest, &counts));
    assert_eq!(
        complete_counted_objectives(db, &mut log, BEATING_THEM_BACK),
        Err(format!("quest {BEATING_THEM_BACK} is not in the log"))
    );
}
