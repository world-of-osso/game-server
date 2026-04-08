use super::*;

fn sample_quest() -> QuestData {
    QuestData {
        id: 100,
        title: "The Defias Brotherhood".into(),
        description: "Investigate the Defias threat in Westfall.".into(),
        objective_text: "Kill 10 Defias Bandits".into(),
        objectives: vec![QuestObjective::Kill {
            creature_id: 94,
            count: 10,
        }],
        rewards: vec![QuestReward::Xp(1500), QuestReward::Gold(350)],
        reward_choices: vec![
            QuestRewardChoice {
                item_id: 2041,
                count: 1,
            },
            QuestRewardChoice {
                item_id: 2042,
                count: 1,
            },
        ],
        required_level: 10,
        suggested_level: 14,
        prerequisites: vec![50],
        quest_giver: 234,
        turn_in_npc: 234,
        next_quest_id: 0,
        exclusive_group: 0,
        repeat: QuestRepeat::None,
    }
}

#[test]
fn quest_data_construction() {
    let q = sample_quest();
    assert_eq!(q.id, 100);
    assert_eq!(q.objectives.len(), 1);
    assert_eq!(q.rewards.len(), 2);
    assert_eq!(q.reward_choices.len(), 2);
    assert_eq!(q.prerequisites, vec![50]);
}

#[test]
fn meets_level_req() {
    let q = sample_quest();
    assert!(!q.meets_level_req(9));
    assert!(q.meets_level_req(10));
    assert!(q.meets_level_req(80));
}

#[test]
fn prerequisites_met_empty() {
    let mut q = sample_quest();
    q.prerequisites.clear();
    assert!(q.prerequisites_met(&[]));
}

#[test]
fn prerequisites_met_with_completed() {
    let q = sample_quest();
    assert!(!q.prerequisites_met(&[]));
    assert!(!q.prerequisites_met(&[49]));
    assert!(q.prerequisites_met(&[50]));
    assert!(q.prerequisites_met(&[50, 51, 52]));
}

#[test]
fn can_accept_both_checks() {
    let q = sample_quest();
    assert!(!q.can_accept(9, &[50], &[])); // too low level
    assert!(!q.can_accept(10, &[], &[])); // prereq missing
    assert!(q.can_accept(10, &[50], &[])); // both met
}

#[test]
fn objective_variants() {
    let objs: [QuestObjective; 5] = [
        QuestObjective::Kill {
            creature_id: 1,
            count: 5,
        },
        QuestObjective::Collect {
            item_id: 2,
            count: 3,
        },
        QuestObjective::Interact { target_id: 3 },
        QuestObjective::Escort { npc_id: 4 },
        QuestObjective::ReachLocation {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            radius: 10.0,
        },
    ];
    assert_eq!(objs.len(), 5);
}

#[test]
fn reward_variants() {
    let rewards: [QuestReward; 4] = [
        QuestReward::Xp(1000),
        QuestReward::Gold(500),
        QuestReward::Item {
            item_id: 100,
            count: 1,
        },
        QuestReward::Reputation {
            faction_id: 72,
            amount: 250,
        },
    ];
    assert_eq!(rewards.len(), 4);
}

#[test]
fn quest_serialization_round_trip() {
    let q = sample_quest();
    let json = serde_json::to_string(&q).unwrap();
    let decoded: QuestData = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, q);
}

// --- Objective progress tests ---

#[test]
fn kill_progress_tracks_count() {
    let obj = QuestObjective::Kill {
        creature_id: 94,
        count: 3,
    };
    let mut progress = ObjectiveProgress::from_objective(&obj);
    assert!(!progress.is_complete());

    assert!(progress.record_kill(94, &obj));
    assert!(progress.record_kill(94, &obj));
    assert!(!progress.is_complete());

    assert!(progress.record_kill(94, &obj));
    assert!(progress.is_complete());
}

#[test]
fn kill_wrong_creature_ignored() {
    let obj = QuestObjective::Kill {
        creature_id: 94,
        count: 3,
    };
    let mut progress = ObjectiveProgress::from_objective(&obj);
    assert!(!progress.record_kill(999, &obj));
}

#[test]
fn kill_over_count_capped() {
    let obj = QuestObjective::Kill {
        creature_id: 94,
        count: 2,
    };
    let mut progress = ObjectiveProgress::from_objective(&obj);
    progress.record_kill(94, &obj);
    progress.record_kill(94, &obj);
    assert!(!progress.record_kill(94, &obj)); // already at max
}

#[test]
fn collect_progress_tracks_count() {
    let obj = QuestObjective::Collect {
        item_id: 50,
        count: 5,
    };
    let mut progress = ObjectiveProgress::from_objective(&obj);
    assert!(progress.record_collect(50, 3, &obj));
    assert!(!progress.is_complete());
    assert!(progress.record_collect(50, 3, &obj)); // clamped to 5
    assert!(progress.is_complete());
}

#[test]
fn interact_mark_done() {
    let obj = QuestObjective::Interact { target_id: 10 };
    let mut progress = ObjectiveProgress::from_objective(&obj);
    assert!(!progress.is_complete());
    progress.mark_done();
    assert!(progress.is_complete());
}

#[test]
fn escort_mark_done() {
    let obj = QuestObjective::Escort { npc_id: 5 };
    let mut progress = ObjectiveProgress::from_objective(&obj);
    progress.mark_done();
    assert!(progress.is_complete());
}

#[test]
fn reach_location_mark_done() {
    let obj = QuestObjective::ReachLocation {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        radius: 10.0,
    };
    let mut progress = ObjectiveProgress::from_objective(&obj);
    progress.mark_done();
    assert!(progress.is_complete());
}

#[test]
fn quest_progress_complete_check() {
    let q = sample_quest();
    let mut progress = QuestProgress::new(&q);
    assert!(!progress.objectives_complete());

    let obj = &q.objectives[0];
    for _ in 0..10 {
        progress.objectives[0].record_kill(94, obj);
    }
    assert!(progress.objectives_complete());
}

// --- Quest state tests ---

#[test]
fn quest_state_accepted_to_in_progress() {
    let q = sample_quest();
    let mut progress = QuestProgress::new(&q);
    assert_eq!(progress.state, QuestState::Accepted);

    progress.update_state();
    assert_eq!(progress.state, QuestState::InProgress);
}

#[test]
fn quest_state_in_progress_to_complete() {
    let q = sample_quest();
    let mut progress = QuestProgress::new(&q);
    progress.update_state(); // → InProgress

    let obj = &q.objectives[0];
    for _ in 0..10 {
        progress.objectives[0].record_kill(94, obj);
    }
    progress.update_state();
    assert_eq!(progress.state, QuestState::Complete);
}

#[test]
fn quest_state_turn_in() {
    let q = sample_quest();
    let mut progress = QuestProgress::new(&q);
    progress.state = QuestState::Complete;
    assert!(progress.turn_in());
    assert_eq!(progress.state, QuestState::TurnedIn);
}

#[test]
fn quest_state_cant_turn_in_incomplete() {
    let q = sample_quest();
    let mut progress = QuestProgress::new(&q);
    progress.update_state(); // → InProgress
    assert!(!progress.turn_in());
    assert_eq!(progress.state, QuestState::InProgress);
}

#[test]
fn quest_state_abandon_resets() {
    let q = sample_quest();
    let mut progress = QuestProgress::new(&q);
    progress.update_state();

    let obj = &q.objectives[0];
    for _ in 0..5 {
        progress.objectives[0].record_kill(94, obj);
    }

    progress.abandon();
    assert_eq!(progress.state, QuestState::Available);
    assert!(!progress.objectives[0].is_complete());
    if let ObjectiveProgress::Kill { current, .. } = progress.objectives[0] {
        assert_eq!(current, 0);
    }
}

#[test]
fn quest_instant_complete_no_objectives() {
    let mut q = sample_quest();
    q.objectives.clear();
    let mut progress = QuestProgress::new(&q);
    progress.update_state();
    assert_eq!(progress.state, QuestState::Complete);
}

// --- Reward claiming tests ---

#[test]
fn claim_rewards_basic() {
    let q = sample_quest(); // rewards: Xp(1500), Gold(350), choices: 2 items
    let rewards = claim_rewards(&q, Some(0)).unwrap();
    assert_eq!(rewards.xp, 1500);
    assert_eq!(rewards.gold, 350);
    assert_eq!(rewards.items.len(), 1); // chosen item
    assert_eq!(rewards.items[0].0, 2041);
}

#[test]
fn claim_rewards_second_choice() {
    let q = sample_quest();
    let rewards = claim_rewards(&q, Some(1)).unwrap();
    assert_eq!(rewards.items[0].0, 2042);
}

#[test]
fn claim_rewards_invalid_choice() {
    let q = sample_quest();
    assert!(claim_rewards(&q, Some(5)).is_none()); // out of range
}

#[test]
fn claim_rewards_no_choice_required() {
    let q = second_quest(); // no reward_choices
    let rewards = claim_rewards(&q, None).unwrap();
    assert_eq!(rewards.xp, 800);
    assert!(rewards.items.is_empty());
}

#[test]
fn claim_rewards_must_choose_when_choices_exist() {
    let q = sample_quest(); // has reward_choices
    assert!(claim_rewards(&q, None).is_none());
}

#[test]
fn claim_rewards_with_reputation() {
    let mut q = second_quest();
    q.rewards.push(QuestReward::Reputation {
        faction_id: 72,
        amount: 250,
    });
    let rewards = claim_rewards(&q, None).unwrap();
    assert_eq!(rewards.reputation.len(), 1);
    assert_eq!(rewards.reputation[0], (72, 250));
}

#[test]
fn claim_rewards_with_guaranteed_item() {
    let mut q = second_quest();
    q.rewards.push(QuestReward::Item {
        item_id: 5000,
        count: 3,
    });
    let rewards = claim_rewards(&q, None).unwrap();
    assert_eq!(rewards.items, vec![(5000, 3)]);
}

// --- Quest marker tests ---

#[test]
fn marker_available_quest() {
    let q = sample_quest(); // quest_giver=234, required_level=10, prereq=[50]
    let mut log = QuestLog::default();
    log.completed.push(50); // prereq met
    let marker = npc_quest_marker(234, &[q], &log, 15);
    assert_eq!(marker, QuestMarker::Available);
}

#[test]
fn marker_low_level() {
    let q = sample_quest(); // required_level=10
    let mut log = QuestLog::default();
    log.completed.push(50);
    let marker = npc_quest_marker(234, &[q], &log, 5); // too low
    assert_eq!(marker, QuestMarker::AvailableLowLevel);
}

#[test]
fn marker_turn_in_complete() {
    let q = sample_quest();
    let mut log = QuestLog::default();
    log.accept(&q);
    log.get_mut(100).unwrap().state = QuestState::Complete;
    let marker = npc_quest_marker(234, &[q], &log, 80);
    assert_eq!(marker, QuestMarker::TurnIn);
}

#[test]
fn marker_turn_in_incomplete() {
    let q = sample_quest();
    let mut log = QuestLog::default();
    log.accept(&q);
    log.get_mut(100).unwrap().state = QuestState::InProgress;
    let marker = npc_quest_marker(234, &[q], &log, 80);
    assert_eq!(marker, QuestMarker::TurnInIncomplete);
}

#[test]
fn marker_none_for_unrelated_npc() {
    let q = sample_quest(); // quest_giver=234
    let log = QuestLog::default();
    let marker = npc_quest_marker(999, &[q], &log, 80);
    assert_eq!(marker, QuestMarker::None);
}

#[test]
fn marker_turn_in_beats_available() {
    // NPC offers a new quest AND accepts a completed one → TurnIn wins
    let q1 = sample_quest(); // giver=234
    let mut q2 = second_quest();
    q2.quest_giver = 234;
    q2.prerequisites.clear();

    let mut log = QuestLog::default();
    log.accept(&q1);
    log.get_mut(100).unwrap().state = QuestState::Complete;

    let marker = npc_quest_marker(234, &[q1, q2], &log, 80);
    assert_eq!(marker, QuestMarker::TurnIn);
}

#[test]
fn marker_already_completed_no_marker() {
    let q = sample_quest();
    let mut log = QuestLog::default();
    log.completed.push(100); // already turned in
    let marker = npc_quest_marker(234, &[q], &log, 80);
    assert_eq!(marker, QuestMarker::None);
}

// --- Quest log tests ---

fn second_quest() -> QuestData {
    QuestData {
        id: 200,
        title: "Red Ridge".into(),
        description: "Help the town.".into(),
        objective_text: "Talk to the marshal.".into(),
        objectives: vec![QuestObjective::Interact { target_id: 500 }],
        rewards: vec![QuestReward::Xp(800)],
        reward_choices: vec![],
        required_level: 15,
        suggested_level: 18,
        prerequisites: vec![100],
        quest_giver: 300,
        turn_in_npc: 300,
        next_quest_id: 0,
        exclusive_group: 0,
        repeat: QuestRepeat::None,
    }
}

#[test]
fn quest_log_accept() {
    let mut log = QuestLog::default();
    assert!(log.accept(&sample_quest()));
    assert_eq!(log.active_count(), 1);
    assert!(log.has_quest(100));
}

#[test]
fn quest_log_reject_duplicate() {
    let mut log = QuestLog::default();
    log.accept(&sample_quest());
    assert!(!log.accept(&sample_quest()));
    assert_eq!(log.active_count(), 1);
}

#[test]
fn quest_log_full_rejects() {
    let mut log = QuestLog::default();
    for i in 0..25 {
        let mut q = sample_quest();
        q.id = 1000 + i;
        assert!(log.accept(&q));
    }
    assert!(!log.accept(&second_quest()));
}

#[test]
fn quest_log_turn_in_flow() {
    let q = sample_quest();
    let mut log = QuestLog::default();
    log.accept(&q);

    // Complete objectives
    let obj = &q.objectives[0];
    let progress = log.get_mut(100).unwrap();
    progress.update_state();
    for _ in 0..10 {
        progress.objectives[0].record_kill(94, obj);
    }
    progress.update_state();

    assert!(log.turn_in(100, &q, 0));
    assert!(!log.has_quest(100));
    assert!(log.is_completed(100));
    assert_eq!(log.active_count(), 0);
}

#[test]
fn quest_log_cant_turn_in_incomplete() {
    let q = sample_quest();
    let mut log = QuestLog::default();
    log.accept(&q);
    log.get_mut(100).unwrap().update_state();
    assert!(!log.turn_in(100, &q, 0));
}

#[test]
fn quest_log_abandon() {
    let mut log = QuestLog::default();
    log.accept(&sample_quest());
    assert!(log.abandon(100));
    assert!(!log.has_quest(100));
    assert!(!log.is_completed(100));
}

#[test]
fn quest_log_completed_enables_prereq() {
    let _q1 = sample_quest(); // id=100
    let q2 = second_quest(); // prereq=[100]
    let mut log = QuestLog::default();

    // q2 needs q1 completed
    assert!(!q2.prerequisites_met(&log.completed));

    // Complete q1
    log.completed.push(100);
    assert!(q2.prerequisites_met(&log.completed));
}

// --- Quest chain and branching tests ---

#[test]
fn chain_next_quest_id() {
    let mut q1 = sample_quest();
    q1.next_quest_id = 200;
    assert_eq!(q1.next_quest_id, 200);
}

#[test]
fn exclusive_group_blocks_alternatives() {
    let mut q_alliance = sample_quest();
    q_alliance.id = 300;
    q_alliance.exclusive_group = 10;

    let mut q_horde = sample_quest();
    q_horde.id = 301;
    q_horde.exclusive_group = 10;
    q_horde.prerequisites.clear();

    let all = vec![q_alliance.clone(), q_horde.clone()];

    // Neither completed — both available
    assert!(!q_alliance.excluded_by(&[], &all));
    assert!(!q_horde.excluded_by(&[], &all));

    // Complete alliance version — horde is blocked
    let completed = vec![300];
    assert!(!q_alliance.excluded_by(&completed, &all)); // self doesn't block self
    assert!(q_horde.excluded_by(&completed, &all));
}

#[test]
fn can_accept_respects_exclusion() {
    let mut q_a = sample_quest();
    q_a.id = 400;
    q_a.exclusive_group = 20;
    q_a.prerequisites.clear();

    let mut q_b = sample_quest();
    q_b.id = 401;
    q_b.exclusive_group = 20;
    q_b.prerequisites.clear();

    let all = vec![q_a.clone(), q_b.clone()];

    assert!(q_b.can_accept(80, &[], &all));
    assert!(!q_b.can_accept(80, &[400], &all)); // 400 completed, 401 blocked
}

#[test]
fn no_exclusive_group_not_blocked() {
    let q = sample_quest(); // exclusive_group=0
    assert!(!q.excluded_by(&[100, 200, 300], std::slice::from_ref(&q)));
}

#[test]
fn chain_prereq_flow() {
    let mut q1 = sample_quest();
    q1.id = 500;
    q1.prerequisites.clear();
    q1.next_quest_id = 501;

    let mut q2 = sample_quest();
    q2.id = 501;
    q2.prerequisites = vec![500];

    let all = vec![q1.clone(), q2.clone()];

    // q2 not available until q1 done
    assert!(!q2.can_accept(80, &[], &all));
    assert!(q2.can_accept(80, &[500], &all));
}

// --- Daily/weekly reset tests ---

#[test]
fn daily_quest_cooldown_and_reset() {
    let mut q = sample_quest();
    q.repeat = QuestRepeat::Daily;

    let mut log = QuestLog::default();
    log.accept(&q);
    log.get_mut(100).unwrap().state = QuestState::Complete;

    // Turn in at hour 12 of day 0 (43200s)
    let noon = 43200;
    assert!(log.turn_in(100, &q, noon));
    assert!(log.is_on_cooldown(100));

    // Can't re-accept while on cooldown
    assert!(!log.accept(&q));

    // Process resets at next day boundary (86400)
    log.process_resets(86400);
    assert!(!log.is_on_cooldown(100));
    assert!(!log.is_completed(100)); // removed from completed
    assert!(log.accept(&q)); // can accept again
}

#[test]
fn weekly_quest_not_reset_before_period() {
    let mut q = sample_quest();
    q.repeat = QuestRepeat::Weekly;

    let mut log = QuestLog::default();
    log.accept(&q);
    log.get_mut(100).unwrap().state = QuestState::Complete;
    assert!(log.turn_in(100, &q, 100000));

    // 3 days later — not reset yet
    log.process_resets(100000 + 3 * 86400);
    assert!(log.is_on_cooldown(100));
}

#[test]
fn weekly_quest_resets_after_period() {
    let mut q = sample_quest();
    q.repeat = QuestRepeat::Weekly;

    let mut log = QuestLog::default();
    log.accept(&q);
    log.get_mut(100).unwrap().state = QuestState::Complete;
    assert!(log.turn_in(100, &q, 100000));

    // 8 days later — reset
    log.process_resets(100000 + 8 * 86400);
    assert!(!log.is_on_cooldown(100));
}

#[test]
fn non_repeatable_no_cooldown() {
    let q = sample_quest(); // repeat = None
    let mut log = QuestLog::default();
    log.accept(&q);
    log.get_mut(100).unwrap().state = QuestState::Complete;
    assert!(log.turn_in(100, &q, 0));
    assert!(!log.is_on_cooldown(100)); // no cooldown for non-repeatable
}

#[test]
fn cooldown_next_reset_daily() {
    let cd = QuestCooldown {
        quest_id: 1,
        repeat: QuestRepeat::Daily,
        completed_at: 50000, // in the middle of day 0
    };
    assert_eq!(cd.next_reset(), 86400); // start of day 1
    assert!(!cd.is_reset(50000));
    assert!(cd.is_reset(86400));
}
