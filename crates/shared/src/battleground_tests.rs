use super::*;

#[test]
fn wsg_template() {
    let wsg = warsong_gulch();
    assert_eq!(wsg.id, 1);
    assert_eq!(wsg.map_id, 489);
    assert_eq!(wsg.team_size, 10);
    assert_eq!(wsg.objective, BgObjective::FlagCapture);
    assert_eq!(wsg.win_score, 3);
    assert_eq!(wsg.max_duration_secs, 25 * 60);
}

#[test]
fn ab_template() {
    let ab = arathi_basin();
    assert_eq!(ab.id, 2);
    assert_eq!(ab.team_size, 15);
    assert_eq!(ab.objective, BgObjective::NodeControl);
    assert_eq!(ab.win_score, 1600);
}

#[test]
fn av_template() {
    let av = alterac_valley();
    assert_eq!(av.id, 3);
    assert_eq!(av.team_size, 40);
    assert_eq!(av.objective, BgObjective::Reinforcements);
    assert_eq!(av.starting_reinforcements, 600);
}

#[test]
fn all_templates_returns_three() {
    let templates = all_templates();
    assert_eq!(templates.len(), 3);
}

#[test]
fn template_ids_unique() {
    let templates = all_templates();
    let mut ids: Vec<u32> = templates.iter().map(|t| t.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), templates.len());
}

#[test]
fn template_map_ids_unique() {
    let templates = all_templates();
    let mut maps: Vec<u32> = templates.iter().map(|t| t.map_id).collect();
    maps.sort();
    maps.dedup();
    assert_eq!(maps.len(), templates.len());
}

#[test]
fn lookup_by_id() {
    assert_eq!(template_by_id(1).unwrap().name, "Warsong Gulch");
    assert_eq!(template_by_id(2).unwrap().name, "Arathi Basin");
    assert_eq!(template_by_id(3).unwrap().name, "Alterac Valley");
    assert!(template_by_id(999).is_none());
}

#[test]
fn min_players_below_team_size() {
    for tmpl in all_templates() {
        assert!(
            tmpl.min_players_per_team <= tmpl.team_size,
            "{}: min {} > team_size {}",
            tmpl.name,
            tmpl.min_players_per_team,
            tmpl.team_size,
        );
    }
}

#[test]
fn level_bracket_contains() {
    let bracket = LevelBracket::new(10, 60);
    assert!(!bracket.contains(9));
    assert!(bracket.contains(10));
    assert!(bracket.contains(35));
    assert!(bracket.contains(60));
    assert!(!bracket.contains(61));
}

#[test]
fn all_templates_have_valid_brackets() {
    for tmpl in all_templates() {
        assert!(
            tmpl.bracket.min_level <= tmpl.bracket.max_level,
            "{}",
            tmpl.name
        );
        assert!(tmpl.bracket.min_level >= 1, "{}", tmpl.name);
    }
}

#[test]
fn serialization_round_trip() {
    let wsg = warsong_gulch();
    let json = serde_json::to_string(&wsg).unwrap();
    let restored: BgTemplate = serde_json::from_str(&json).unwrap();
    assert_eq!(wsg, restored);
}

// -- Queue tests --

#[test]
fn join_solo_and_is_queued() {
    let mut q = BgQueue::default();
    assert!(q.join_solo(1, 30, 1, 100).is_ok());
    assert!(q.is_queued(1));
    assert_eq!(q.len(), 1);
}

#[test]
fn join_solo_already_queued() {
    let mut q = BgQueue::default();
    q.join_solo(1, 30, 1, 100).unwrap();
    assert_eq!(
        q.join_solo(1, 30, 1, 101),
        Err(BgQueueError::AlreadyQueued)
    );
}

#[test]
fn join_unknown_bg() {
    let mut q = BgQueue::default();
    assert_eq!(q.join_solo(1, 30, 999, 100), Err(BgQueueError::UnknownBg));
}

#[test]
fn join_out_of_bracket() {
    let mut q = BgQueue::default();
    // WSG bracket is 10-60
    assert_eq!(
        q.join_solo(1, 5, 1, 100),
        Err(BgQueueError::OutOfBracket)
    );
    assert_eq!(
        q.join_solo(1, 61, 1, 100),
        Err(BgQueueError::OutOfBracket)
    );
}

#[test]
fn join_group() {
    let mut q = BgQueue::default();
    q.join_group(1, vec![1, 2, 3], vec![30, 30, 30], 1, 100)
        .unwrap();
    assert!(q.is_queued(1));
    assert!(q.is_queued(2));
    assert!(q.is_queued(3));
    assert_eq!(q.len(), 1);
}

#[test]
fn join_empty_group() {
    let mut q = BgQueue::default();
    assert_eq!(
        q.join_group(1, vec![], vec![], 1, 100),
        Err(BgQueueError::EmptyGroup)
    );
}

#[test]
fn leave_queue() {
    let mut q = BgQueue::default();
    q.join_solo(1, 30, 1, 100).unwrap();
    assert!(q.leave(1));
    assert!(!q.is_queued(1));
    assert!(q.is_empty());
}

#[test]
fn leave_not_queued() {
    let mut q = BgQueue::default();
    assert!(!q.leave(999));
}

#[test]
fn match_not_enough_players() {
    let mut q = BgQueue::default();
    // WSG needs 5 per team min (10 total), only queue 8
    for i in 1..=8 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }
    assert!(q.try_match(1).is_none());
    assert_eq!(q.len(), 8); // entries preserved
}

#[test]
fn match_forms_two_teams() {
    let mut q = BgQueue::default();
    // WSG: 10v10, min 5 per team. Queue 10 solo players.
    for i in 1..=10 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }
    let m = q.try_match(1).unwrap();
    assert_eq!(m.bg_id, 1);
    assert_eq!(m.team_a.len(), 5);
    assert_eq!(m.team_b.len(), 5);

    // All 10 players should be in one of the teams
    let mut all: Vec<u64> = m.team_a.iter().chain(&m.team_b).copied().collect();
    all.sort();
    assert_eq!(all, (1..=10).collect::<Vec<u64>>());

    // Queue should have 0 entries left
    assert!(q.is_empty());
}

#[test]
fn match_with_groups() {
    let mut q = BgQueue::default();
    // Queue 2 groups of 3 + 4 solos = 10 players for WSG
    q.join_group(1, vec![1, 2, 3], vec![30, 30, 30], 1, 100)
        .unwrap();
    q.join_group(4, vec![4, 5, 6], vec![30, 30, 30], 1, 100)
        .unwrap();
    for i in 7..=10 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }
    let m = q.try_match(1).unwrap();
    let total = m.team_a.len() + m.team_b.len();
    assert_eq!(total, 10);
}

#[test]
fn match_different_bg_types_independent() {
    let mut q = BgQueue::default();
    // 10 for WSG, 8 for AB (not enough for AB which needs 16 min)
    for i in 1..=10 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }
    for i in 11..=18 {
        q.join_solo(i, 30, 2, 100).unwrap();
    }
    assert!(q.try_match(1).is_some()); // WSG ok
    assert!(q.try_match(2).is_none()); // AB needs 16
}

#[test]
fn match_unknown_bg_returns_none() {
    let mut q = BgQueue::default();
    assert!(q.try_match(999).is_none());
}

// -- State machine tests --

fn wsg_instance() -> (BgInstance, BgTemplate) {
    let tmpl = warsong_gulch();
    let bg_match = BgMatch {
        bg_id: 1,
        team_a: vec![1, 2, 3, 4, 5],
        team_b: vec![6, 7, 8, 9, 10],
    };
    (BgInstance::from_match(&bg_match, &tmpl), tmpl)
}

#[test]
fn instance_starts_in_waiting() {
    let (inst, _) = wsg_instance();
    assert_eq!(inst.phase, BgPhase::Waiting);
    assert_eq!(inst.elapsed_secs, 0);
    assert!(inst.outcome.is_none());
}

#[test]
fn start_transitions_to_in_progress() {
    let (mut inst, _) = wsg_instance();
    assert!(inst.start());
    assert_eq!(inst.phase, BgPhase::InProgress);
}

#[test]
fn start_twice_returns_false() {
    let (mut inst, _) = wsg_instance();
    inst.start();
    assert!(!inst.start());
}

#[test]
fn score_win_ends_match() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    // WSG win_score = 3 flag captures
    inst.add_score(BgTeam::A, 2, &tmpl);
    assert_eq!(inst.phase, BgPhase::InProgress);
    inst.add_score(BgTeam::A, 1, &tmpl);
    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
    assert_eq!(inst.score_a.score, 3);
}

#[test]
fn score_ignored_when_not_in_progress() {
    let (mut inst, tmpl) = wsg_instance();
    // Still waiting — score should be ignored
    inst.add_score(BgTeam::A, 5, &tmpl);
    assert_eq!(inst.score_a.score, 0);
}

#[test]
fn score_ignored_after_ended() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl); // ends match
    inst.add_score(BgTeam::B, 1, &tmpl); // should be ignored
    assert_eq!(inst.score_b.score, 0);
}

#[test]
fn reinforcement_depletion_ends_match() {
    let tmpl = alterac_valley();
    let bg_match = BgMatch {
        bg_id: 3,
        team_a: vec![1],
        team_b: vec![2],
    };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();
    assert_eq!(inst.score_a.reinforcements, 600);

    inst.deduct_reinforcements(BgTeam::A, 599);
    assert_eq!(inst.phase, BgPhase::InProgress);
    assert_eq!(inst.score_a.reinforcements, 1);

    inst.deduct_reinforcements(BgTeam::A, 1);
    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::B)));
}

#[test]
fn reinforcement_saturates_at_zero() {
    let tmpl = alterac_valley();
    let bg_match = BgMatch {
        bg_id: 3,
        team_a: vec![1],
        team_b: vec![2],
    };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();
    inst.deduct_reinforcements(BgTeam::B, 9999);
    assert_eq!(inst.score_b.reinforcements, 0);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
}

#[test]
fn tick_advances_time() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.tick(60, &tmpl);
    assert_eq!(inst.elapsed_secs, 60);
    assert_eq!(inst.phase, BgPhase::InProgress);
}

#[test]
fn tick_timeout_draws() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    // WSG max_duration = 25 * 60 = 1500s
    inst.tick(1500, &tmpl);
    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Draw));
}

#[test]
fn tick_ignored_when_waiting() {
    let (mut inst, tmpl) = wsg_instance();
    inst.tick(100, &tmpl);
    assert_eq!(inst.elapsed_secs, 0);
}

#[test]
fn team_b_can_win() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::B, 3, &tmpl);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::B)));
}

#[test]
fn both_teams_score_tracked_independently() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 1, &tmpl);
    inst.add_score(BgTeam::B, 2, &tmpl);
    assert_eq!(inst.score_a.score, 1);
    assert_eq!(inst.score_b.score, 2);
}

// -- Flag capture tests --

#[test]
fn flag_pickup_from_base() {
    let mut flags = FlagObjective::default();
    assert!(flags.pickup(BgTeam::A, 1)); // A picks up B's flag
    assert_eq!(flags.flag_b, FlagState::Carried { carrier: 1 });
}

#[test]
fn flag_pickup_already_carried() {
    let mut flags = FlagObjective::default();
    flags.pickup(BgTeam::A, 1);
    assert!(!flags.pickup(BgTeam::A, 2)); // can't pick up what's already carried
}

#[test]
fn flag_drop_and_return() {
    let mut flags = FlagObjective::default();
    flags.pickup(BgTeam::A, 1);
    flags.drop_flag(BgTeam::A);
    assert_eq!(flags.flag_b, FlagState::Dropped);

    // Team B returns their own flag
    assert!(flags.return_flag(BgTeam::B));
    assert_eq!(flags.flag_b, FlagState::AtBase);
}

#[test]
fn flag_return_at_base_fails() {
    let mut flags = FlagObjective::default();
    assert!(!flags.return_flag(BgTeam::A)); // already at base
}

#[test]
fn flag_capture_scores_point() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    let mut flags = FlagObjective::default();

    // Team A picks up B's flag and captures it
    flags.pickup(BgTeam::A, 1);
    assert!(flags.capture(BgTeam::A, &mut inst, &tmpl));
    assert_eq!(inst.score_a.score, 1);
    assert_eq!(flags.flag_b, FlagState::AtBase); // flag returns after capture
}

#[test]
fn flag_capture_requires_own_flag_at_base() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    let mut flags = FlagObjective::default();

    // Both flags picked up
    flags.pickup(BgTeam::A, 1); // A has B's flag
    flags.pickup(BgTeam::B, 2); // B has A's flag

    // A can't capture while their own flag is carried
    assert!(!flags.capture(BgTeam::A, &mut inst, &tmpl));
    assert_eq!(inst.score_a.score, 0);
}

#[test]
fn flag_three_captures_wins_wsg() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    let mut flags = FlagObjective::default();

    for _ in 0..3 {
        flags.pickup(BgTeam::A, 1);
        flags.capture(BgTeam::A, &mut inst, &tmpl);
    }
    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
}

#[test]
fn flag_pickup_dropped_flag() {
    let mut flags = FlagObjective::default();
    flags.pickup(BgTeam::A, 1);
    flags.drop_flag(BgTeam::A);
    // Another player picks up the dropped flag
    assert!(flags.pickup(BgTeam::A, 2));
    assert_eq!(flags.flag_b, FlagState::Carried { carrier: 2 });
}

// -- Node control tests --

#[test]
fn node_start_capture_neutral() {
    let mut nodes = NodeObjective::arathi_basin();
    assert!(nodes.start_capture(0, BgTeam::A));
    assert_eq!(nodes.nodes[0].owner, NodeOwner::Contested(BgTeam::A));
}

#[test]
fn node_finish_capture() {
    let mut nodes = NodeObjective::arathi_basin();
    nodes.start_capture(0, BgTeam::A);
    assert!(nodes.finish_capture(0));
    assert_eq!(nodes.nodes[0].owner, NodeOwner::Owned(BgTeam::A));
}

#[test]
fn node_cannot_capture_own_node() {
    let mut nodes = NodeObjective::arathi_basin();
    nodes.start_capture(0, BgTeam::A);
    nodes.finish_capture(0);
    assert!(!nodes.start_capture(0, BgTeam::A)); // already own it
}

#[test]
fn node_enemy_can_contest_owned() {
    let mut nodes = NodeObjective::arathi_basin();
    nodes.start_capture(0, BgTeam::A);
    nodes.finish_capture(0);
    assert!(nodes.start_capture(0, BgTeam::B));
    assert_eq!(nodes.nodes[0].owner, NodeOwner::Contested(BgTeam::B));
}

#[test]
fn node_cannot_contest_contested() {
    let mut nodes = NodeObjective::arathi_basin();
    nodes.start_capture(0, BgTeam::A);
    assert!(!nodes.start_capture(0, BgTeam::B)); // already contested
}

#[test]
fn node_owned_count() {
    let mut nodes = NodeObjective::arathi_basin();
    assert_eq!(nodes.owned_count(BgTeam::A), 0);
    nodes.start_capture(0, BgTeam::A);
    nodes.finish_capture(0);
    nodes.start_capture(1, BgTeam::A);
    nodes.finish_capture(1);
    assert_eq!(nodes.owned_count(BgTeam::A), 2);
    assert_eq!(nodes.owned_count(BgTeam::B), 0);
}

#[test]
fn node_tick_resources_adds_score() {
    let tmpl = arathi_basin();
    let bg_match = BgMatch { bg_id: 2, team_a: vec![1], team_b: vec![2] };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    let mut nodes = NodeObjective::arathi_basin();
    // Give A two nodes
    nodes.start_capture(0, BgTeam::A);
    nodes.finish_capture(0);
    nodes.start_capture(1, BgTeam::A);
    nodes.finish_capture(1);

    nodes.tick_resources(&mut inst, &tmpl);
    assert_eq!(inst.score_a.score, 10); // 2 nodes = 10 pts/tick
    assert_eq!(inst.score_b.score, 0);
}

#[test]
fn node_five_nodes_gives_thirty_per_tick() {
    let tmpl = arathi_basin();
    let bg_match = BgMatch { bg_id: 2, team_a: vec![1], team_b: vec![2] };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    let mut nodes = NodeObjective::arathi_basin();
    for i in 0..5 {
        nodes.start_capture(i, BgTeam::A);
        nodes.finish_capture(i);
    }
    nodes.tick_resources(&mut inst, &tmpl);
    assert_eq!(inst.score_a.score, 30);
}

#[test]
fn node_ab_has_five_nodes() {
    let nodes = NodeObjective::arathi_basin();
    assert_eq!(nodes.nodes.len(), 5);
}

// -- Reinforcement tests --

#[test]
fn kill_deducts_reinforcements() {
    let tmpl = alterac_valley();
    let bg_match = BgMatch { bg_id: 3, team_a: vec![1], team_b: vec![2] };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    on_kill_reinforcement(&mut inst, BgTeam::A);
    assert_eq!(inst.score_a.reinforcements, 599);

    on_kill_reinforcement(&mut inst, BgTeam::B);
    assert_eq!(inst.score_b.reinforcements, 599);
}

#[test]
fn kills_deplete_reinforcements_to_zero() {
    let tmpl = alterac_valley();
    let bg_match = BgMatch { bg_id: 3, team_a: vec![1], team_b: vec![2] };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    for _ in 0..600 {
        on_kill_reinforcement(&mut inst, BgTeam::B);
    }
    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
}

// -- Reward distribution tests --

#[test]
fn rewards_none_when_not_ended() {
    let (inst, _) = wsg_instance();
    assert!(distribute_rewards(&inst).is_none()); // still Waiting
}

#[test]
fn rewards_winner_gets_more() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl); // A wins

    let dist = distribute_rewards(&inst).unwrap();
    let a_reward = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let b_reward = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;

    assert_eq!(a_reward.honor, 150);
    assert_eq!(a_reward.marks, 3);
    assert_eq!(b_reward.honor, 50);
    assert_eq!(b_reward.marks, 1);
}

#[test]
fn rewards_team_b_wins() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::B, 3, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    let a_reward = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let b_reward = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;

    assert_eq!(a_reward.honor, 50);
    assert_eq!(b_reward.honor, 150);
}

#[test]
fn rewards_draw() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.tick(tmpl.max_duration_secs, &tmpl); // timeout → draw

    let dist = distribute_rewards(&inst).unwrap();
    let a_reward = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let b_reward = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;

    assert_eq!(a_reward.honor, 75);
    assert_eq!(a_reward.marks, 2);
    assert_eq!(b_reward.honor, 75);
    assert_eq!(b_reward.marks, 2);
}

#[test]
fn rewards_all_players_included() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    assert_eq!(dist.rewards.len(), 10); // 5 per team
}

#[test]
fn rewards_loser_still_gets_marks() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    let loser = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;
    assert!(loser.marks > 0, "losers should still get marks");
    assert!(loser.honor > 0, "losers should still get honor");
}

// -- Deserter penalty tests --

#[test]
fn deserter_applied_and_checked() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    assert!(tracker.is_deserter(1, 10000));
    assert!(tracker.is_deserter(1, 10899)); // 899s later, still active
    assert!(!tracker.is_deserter(1, 10900)); // 900s = expired
}

#[test]
fn deserter_remaining_time() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    assert_eq!(tracker.remaining(1, 10000), 900);
    assert_eq!(tracker.remaining(1, 10500), 400);
    assert_eq!(tracker.remaining(1, 10900), 0);
    assert_eq!(tracker.remaining(2, 10000), 0); // not a deserter
}

#[test]
fn deserter_not_active_for_others() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    assert!(!tracker.is_deserter(2, 10000));
}

#[test]
fn deserter_cleanup_removes_expired() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    tracker.apply(2, 10500);
    tracker.cleanup(10900);
    assert!(!tracker.is_deserter(1, 10900)); // expired
    assert!(tracker.is_deserter(2, 10900)); // still active until 11400
}

#[test]
fn deserter_refresh_extends() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000); // expires at 10900
    tracker.apply(1, 10500); // extends to 11400
    assert!(tracker.is_deserter(1, 11000));
    assert!(!tracker.is_deserter(1, 11400));
}

#[test]
fn leave_bg_applies_deserter() {
    let (mut inst, _) = wsg_instance();
    inst.start();
    let mut tracker = BgDeserterTracker::default();

    assert!(leave_bg(&mut inst, &mut tracker, 1, 5000));
    assert!(tracker.is_deserter(1, 5000));
    assert!(!inst.team_a_players.contains(&1));
    assert_eq!(inst.team_a_players.len(), 4); // was 5, now 4
}

#[test]
fn leave_bg_team_b_player() {
    let (mut inst, _) = wsg_instance();
    inst.start();
    let mut tracker = BgDeserterTracker::default();

    assert!(leave_bg(&mut inst, &mut tracker, 6, 5000));
    assert!(tracker.is_deserter(6, 5000));
    assert!(!inst.team_b_players.contains(&6));
}

#[test]
fn leave_bg_not_in_match() {
    let (mut inst, _) = wsg_instance();
    inst.start();
    let mut tracker = BgDeserterTracker::default();

    assert!(!leave_bg(&mut inst, &mut tracker, 99, 5000));
    assert!(!tracker.is_deserter(99, 5000));
}

#[test]
fn leave_bg_not_in_progress() {
    let (mut inst, _) = wsg_instance();
    // Still waiting — leaving shouldn't apply deserter
    let mut tracker = BgDeserterTracker::default();

    assert!(!leave_bg(&mut inst, &mut tracker, 1, 5000));
    assert!(!tracker.is_deserter(1, 5000));
}

#[test]
fn deserter_blocks_queue_join() {
    let mut q = BgQueue::default();
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);

    assert_eq!(
        q.join_solo_checked(1, 30, 1, 10000, &tracker),
        Err(BgQueueError::Deserter)
    );

    // After penalty expires, can join again
    assert!(q.join_solo_checked(1, 30, 1, 10900, &tracker).is_ok());
}

// -- Integration tests: full pipeline --

#[test]
fn wsg_full_pipeline_queue_to_rewards() {
    // 1. Queue 10 players
    let mut q = BgQueue::default();
    for i in 1..=10 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }

    // 2. Match
    let bg_match = q.try_match(1).unwrap();
    assert!(q.is_empty());

    // 3. Create instance and start
    let tmpl = warsong_gulch();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    // 4. Play out flag captures — team A wins 3-1
    let mut flags = FlagObjective::default();
    for _ in 0..3 {
        flags.pickup(BgTeam::A, bg_match.team_a[0]);
        flags.capture(BgTeam::A, &mut inst, &tmpl);
    }

    // 5. Verify outcome and rewards
    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));

    let dist = distribute_rewards(&inst).unwrap();
    assert_eq!(dist.rewards.len(), 10);

    let winner = dist.rewards.iter().find(|(p, _)| *p == bg_match.team_a[0]).unwrap().1;
    let loser = dist.rewards.iter().find(|(p, _)| *p == bg_match.team_b[0]).unwrap().1;
    assert!(winner.honor > loser.honor);
    assert!(winner.marks > loser.marks);
}

#[test]
fn ab_full_pipeline_node_control_to_win() {
    // 1. Queue 16 players for AB (min 8 per team)
    let mut q = BgQueue::default();
    for i in 1..=16 {
        q.join_solo(i, 30, 2, 100).unwrap();
    }

    // 2. Match
    let bg_match = q.try_match(2).unwrap();

    // 3. Create instance and start
    let tmpl = arathi_basin();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    // 4. Team A captures all 5 nodes
    let mut nodes = NodeObjective::arathi_basin();
    for i in 0..5 {
        nodes.start_capture(i, BgTeam::A);
        nodes.finish_capture(i);
    }

    // 5. Tick resources until A wins (1600 pts at 30/tick = 54 ticks)
    while inst.phase == BgPhase::InProgress {
        nodes.tick_resources(&mut inst, &tmpl);
    }

    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
    assert!(inst.score_a.score >= 1600);

    let dist = distribute_rewards(&inst).unwrap();
    let winner = dist.rewards.iter().find(|(p, _)| *p == bg_match.team_a[0]).unwrap().1;
    assert_eq!(winner.honor, 150);
    assert_eq!(winner.marks, 3);
}

#[test]
fn av_full_pipeline_reinforcements_to_win() {
    // Use a small match for AV (just need the mechanics)
    let tmpl = alterac_valley();
    let bg_match = BgMatch {
        bg_id: 3,
        team_a: vec![1, 2],
        team_b: vec![3, 4],
    };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    assert_eq!(inst.score_b.reinforcements, 600);

    // Team A kills deplete B's reinforcements
    for _ in 0..600 {
        on_kill_reinforcement(&mut inst, BgTeam::B);
    }

    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
    assert_eq!(inst.score_b.reinforcements, 0);

    let dist = distribute_rewards(&inst).unwrap();
    let winner = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let loser = dist.rewards.iter().find(|(p, _)| *p == 3).unwrap().1;
    assert_eq!(winner.honor, 150);
    assert_eq!(loser.honor, 50);
}

#[test]
fn deserter_lifecycle_leave_block_expire_rejoin() {
    let mut q = BgQueue::default();
    let mut deserters = BgDeserterTracker::default();

    // Queue and match
    for i in 1..=10 {
        q.join_solo(i, 30, 1, 1000).unwrap();
    }
    let bg_match = q.try_match(1).unwrap();
    let tmpl = warsong_gulch();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    // Player 1 leaves mid-match
    leave_bg(&mut inst, &mut deserters, bg_match.team_a[0], 2000);
    assert!(deserters.is_deserter(bg_match.team_a[0], 2000));

    // Can't re-queue while deserter
    assert_eq!(
        q.join_solo_checked(bg_match.team_a[0], 30, 1, 2000, &deserters),
        Err(BgQueueError::Deserter)
    );

    // After 15 minutes (900s), penalty expires
    let after_expiry = 2000 + 900;
    assert!(!deserters.is_deserter(bg_match.team_a[0], after_expiry));
    assert!(q.join_solo_checked(bg_match.team_a[0], 30, 1, after_expiry, &deserters).is_ok());
}

#[test]
fn timeout_draw_both_teams_get_draw_rewards() {
    let mut q = BgQueue::default();
    for i in 1..=10 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }
    let bg_match = q.try_match(1).unwrap();
    let tmpl = warsong_gulch();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    // Neither team scores — time runs out
    inst.tick(tmpl.max_duration_secs, &tmpl);
    assert_eq!(inst.outcome, Some(BgOutcome::Draw));

    let dist = distribute_rewards(&inst).unwrap();
    for (_, reward) in &dist.rewards {
        assert_eq!(reward.honor, 75);
        assert_eq!(reward.marks, 2);
    }
}
