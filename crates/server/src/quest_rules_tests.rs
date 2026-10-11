//! Rules against real world.db rows (Northshire).
use shared::protocol::{
    QuestFailedReason, QuestGiverQuestState, QuestGiverStatus, QuestMarkerClass,
};

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
        ..Default::default()
    }
}

fn log_with(active_quests: &[u32], rewarded: &[u32]) -> QuestLogData {
    let mut rewarded = rewarded.to_vec();
    rewarded.sort_unstable();
    QuestLogData {
        active: active_quests.iter().copied().map(active).collect(),
        rewarded,
        watched: Vec::new(),
        ..Default::default()
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
        QuestGiverStatus::Available(QuestMarkerClass::Normal)
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
        QuestGiverStatus::Reward(QuestMarkerClass::Normal)
    );
    assert_eq!(
        giver_status(db, &turned_in, HUMAN_WARRIOR, DEPUTY_WILLEM, no_items),
        QuestGiverStatus::Available(QuestMarkerClass::Normal)
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
        QuestGiverStatus::Reward(QuestMarkerClass::Normal)
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
        QuestGiverStatus::Available(QuestMarkerClass::Normal)
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

const ANACHRONOS: u32 = 15192;
/// On a Wing and a Prayer: legendary (`FlagsEx` 0x100, tdb_quest_template), MinLevel
/// 35, any race or class, no previous quest.
const ON_A_WING_AND_A_PRAYER: u32 = 29193;
const PUBLIC_RELATIONS_AGENT: u32 = 37675;
/// A Perfect Puff of Perfume, A Cloudlet of Classy Cologne, Bonbon Blitz: dailies
/// (`Flags` 0x1000) she starts and ends, no level floor.
const HER_DAILIES: [u32; 3] = [24629, 24635, 24636];
const KORE: u32 = 16503;
/// Warrior Training: QuestLevel 2, MinLevel 1.
const WARRIOR_TRAINING: u32 = 9289;

fn at_level(level: u8) -> QuestPlayer {
    QuestPlayer {
        level,
        ..HUMAN_WARRIOR
    }
}

/// TrinityCore `Player::GetQuestDialogStatus` (Player.cpp:16234, 16245): a legendary
/// quest is `LegendaryQuest` once the level allows it and `FutureLegendaryQuest` below.
#[test]
fn anachronos_marks_his_legendary_quest_by_level() {
    let Some(db) = test_quest_db() else { return };
    let log = QuestLogData::default();
    let status = |level| giver_status(db, &log, at_level(level), ANACHRONOS, no_items);

    let quest = &db.quests[&ON_A_WING_AND_A_PRAYER];
    assert_eq!(quest.min_level, 35);
    assert_eq!(
        status(35),
        QuestGiverStatus::Available(QuestMarkerClass::Legendary)
    );
    assert_eq!(
        start_state(db, &log, at_level(35), quest),
        Some(QuestGiverStatus::Available(QuestMarkerClass::Legendary))
    );
    // At 30 his other quests outrank it at the giver; the quest itself is Future.
    assert_eq!(
        start_state(db, &log, at_level(30), quest),
        Some(QuestGiverStatus::Future(QuestMarkerClass::Legendary))
    );
}

/// Player.cpp:16194/16239: a daily quest is `RepeatableQuest` to offer and
/// `RepeatableReward` while it is in the log unfinished.
#[test]
fn public_relations_agent_marks_her_daily_quest_repeatable() {
    let Some(db) = test_quest_db() else { return };
    let status =
        |log: &QuestLogData| giver_status(db, log, at_level(10), PUBLIC_RELATIONS_AGENT, no_items);

    assert_eq!(
        status(&QuestLogData::default()),
        QuestGiverStatus::Available(QuestMarkerClass::Repeatable)
    );
    assert_eq!(
        status(&log_with(&HER_DAILIES, &[])),
        QuestGiverStatus::Incomplete(QuestMarkerClass::Repeatable)
    );
}

/// Player.cpp:16227: more than `Quests.LowLevelHideDiff` (4) levels above a fixed-level
/// quest it is `Trivial`.
#[test]
fn kore_marks_warrior_training_trivial_above_level_six() {
    let Some(db) = test_quest_db() else { return };
    let log = QuestLogData::default();
    let status = |level| giver_status(db, &log, at_level(level), KORE, no_items);

    assert_eq!(db.quests[&WARRIOR_TRAINING].level, 2);
    assert_eq!(
        status(6),
        QuestGiverStatus::Available(QuestMarkerClass::Normal)
    );
    assert_eq!(
        status(7),
        QuestGiverStatus::Trivial(QuestMarkerClass::Normal)
    );
}

/// Player.cpp:16227 with Player::GetQuestLevel (:14339-14353): a ContentTuning quest's
/// level is the player's level clamped to the tuning range, so Beating Them Back!
/// (ContentTuning 1476, levels 1-10) turns trivial above level 14.
#[test]
fn mcbride_marks_tuned_beating_them_back_trivial_above_level_fourteen() {
    let Some(db) = test_quest_db() else { return };
    let log = QuestLogData::default();
    let status = |level| giver_status(db, &log, at_level(level), MARSHAL_MCBRIDE, no_items);

    assert_eq!(db.quests[&BEATING_THEM_BACK].level, -1);
    assert_eq!(
        status(14),
        QuestGiverStatus::Available(QuestMarkerClass::Normal)
    );
    assert_eq!(
        status(15),
        QuestGiverStatus::Trivial(QuestMarkerClass::Normal)
    );
}

/// Stormwind's Aysa Cloudsinger offers A Choice of Allies (QuestInfo 282 "Important
/// Quest", Modifiers 0x400) after The Call for Allies: Player.cpp:16228-16229 picks
/// the Important kind (`Quest::IsImportant`, QuestDef.cpp:509-515).
#[test]
fn aysa_cloudsinger_marks_a_choice_of_allies_important() {
    const AYSA_CLOUDSINGER: u32 = 126332;
    const THE_CALL_FOR_ALLIES: u32 = 49929;
    const A_CHOICE_OF_ALLIES: u32 = 50239;
    let Some(db) = test_quest_db() else { return };
    let human_40 = at_level(40);

    assert_eq!(
        db.quests[&A_CHOICE_OF_ALLIES].marker_class(),
        QuestMarkerClass::Important
    );
    assert_eq!(
        giver_status(
            db,
            &log_with(&[], &[THE_CALL_FOR_ALLIES]),
            human_40,
            AYSA_CLOUDSINGER,
            no_items
        ),
        QuestGiverStatus::Available(QuestMarkerClass::Important)
    );
    assert_eq!(
        giver_status(
            db,
            &log_with(&[A_CHOICE_OF_ALLIES], &[THE_CALL_FOR_ALLIES]),
            human_40,
            AYSA_CLOUDSINGER,
            no_items
        ),
        QuestGiverStatus::Incomplete(QuestMarkerClass::Important)
    );
}

/// Player.cpp:16228-16233 kind order over real QuestInfo rows (no Meta or Calling
/// quest has a world.db giver): 284 "Meta Quest" (Modifiers 0x800), 271 "Calling
/// Quest" (Type 15), and 282 "Important Quest" winning over a daily flag.
#[test]
fn quest_info_meta_calling_and_important_pick_their_marker_kinds() {
    let with_info = |quest_info_type, quest_info_modifiers, flags| QuestTemplate {
        quest_info_type,
        quest_info_modifiers,
        flags,
        ..QuestTemplate::default()
    };
    const DAILY: u32 = 0x1000;

    assert_eq!(with_info(0, 2048, 0).marker_class(), QuestMarkerClass::Meta);
    assert_eq!(
        with_info(15, 0, 0).marker_class(),
        QuestMarkerClass::Calling
    );
    assert_eq!(
        with_info(0, 1024, DAILY).marker_class(),
        QuestMarkerClass::Important
    );
    assert_eq!(
        with_info(0, 0, DAILY).marker_class(),
        QuestMarkerClass::Repeatable
    );
}

#[test]
fn quest_reset_reopens_daily_exclusive_alternatives() {
    let quest = QuestTemplate {
        id: 1,
        flags: 0x1000,
        exclusive_group: 10,
        ..Default::default()
    };
    let alternate = QuestTemplate {
        id: 2,
        ..quest.clone()
    };
    let before = 1_791_730_799;
    let mut db = QuestDb {
        now: Some(before),
        ..Default::default()
    };
    db.quests.insert(1, quest);
    db.quests.insert(2, alternate.clone());
    db.exclusive_groups.insert(10, vec![1, 2]);
    let mut log = log_with(&[], &[1]);
    log.completed_at.insert(1, before);
    assert_eq!(
        can_take(&db, &log, HUMAN_WARRIOR, &alternate),
        Err(QuestFailedReason::DontHaveRequirement)
    );
    db.now = Some(before + 1);
    assert_eq!(can_take(&db, &log, HUMAN_WARRIOR, &alternate), Ok(()));
}

#[test]
fn daily_quest_is_available_again_after_reset() {
    // Sunday 2026-10-11 14:59:59 UTC -> local reset 15:00:00 UTC.
    let before = 1_791_730_799;
    let mut db = QuestDb {
        now: Some(before),
        ..Default::default()
    };
    for (flags, boundary) in [(0x1000, before + 1), (0x8000, before + 2 * 86400 + 1)] {
        let quest = QuestTemplate {
            id: 1,
            flags,
            ..Default::default()
        };
        let mut log = log_with(&[], &[1]);
        log.completed_at.insert(1, before);
        db.now = Some(boundary - 1);
        assert_eq!(
            can_take(&db, &log, HUMAN_WARRIOR, &quest),
            Err(QuestFailedReason::AlreadyDone)
        );
        db.now = Some(boundary);
        assert_eq!(can_take(&db, &log, HUMAN_WARRIOR, &quest), Ok(()));
        assert!(log.is_rewarded(1), "prerequisite history survives reset");
    }
}
