use super::*;

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
    inst.add_score(BgTeam::A, 5, &tmpl);
    assert_eq!(inst.score_a.score, 0);
}

#[test]
fn score_ignored_after_ended() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl);
    inst.add_score(BgTeam::B, 1, &tmpl);
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

#[test]
fn flag_pickup_from_base() {
    let mut flags = FlagObjective::default();
    assert!(flags.pickup(BgTeam::A, 1));
    assert_eq!(flags.flag_b, FlagState::Carried { carrier: 1 });
}

#[test]
fn flag_pickup_already_carried() {
    let mut flags = FlagObjective::default();
    flags.pickup(BgTeam::A, 1);
    assert!(!flags.pickup(BgTeam::A, 2));
}

#[test]
fn flag_drop_and_return() {
    let mut flags = FlagObjective::default();
    flags.pickup(BgTeam::A, 1);
    flags.drop_flag(BgTeam::A);
    assert_eq!(flags.flag_b, FlagState::Dropped);
    assert!(flags.return_flag(BgTeam::B));
    assert_eq!(flags.flag_b, FlagState::AtBase);
}

#[test]
fn flag_return_at_base_fails() {
    let mut flags = FlagObjective::default();
    assert!(!flags.return_flag(BgTeam::A));
}

#[test]
fn flag_capture_scores_point() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    let mut flags = FlagObjective::default();

    flags.pickup(BgTeam::A, 1);
    assert!(flags.capture(BgTeam::A, &mut inst, &tmpl));
    assert_eq!(inst.score_a.score, 1);
    assert_eq!(flags.flag_b, FlagState::AtBase);
}

#[test]
fn flag_capture_requires_own_flag_at_base() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    let mut flags = FlagObjective::default();

    flags.pickup(BgTeam::A, 1);
    flags.pickup(BgTeam::B, 2);

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
    assert!(flags.pickup(BgTeam::A, 2));
    assert_eq!(flags.flag_b, FlagState::Carried { carrier: 2 });
}

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
    assert!(!nodes.start_capture(0, BgTeam::A));
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
    assert!(!nodes.start_capture(0, BgTeam::B));
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
    let bg_match = BgMatch {
        bg_id: 2,
        team_a: vec![1],
        team_b: vec![2],
    };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    let mut nodes = NodeObjective::arathi_basin();
    nodes.start_capture(0, BgTeam::A);
    nodes.finish_capture(0);
    nodes.start_capture(1, BgTeam::A);
    nodes.finish_capture(1);

    nodes.tick_resources(&mut inst, &tmpl);
    assert_eq!(inst.score_a.score, 10);
    assert_eq!(inst.score_b.score, 0);
}

#[test]
fn node_five_nodes_gives_thirty_per_tick() {
    let tmpl = arathi_basin();
    let bg_match = BgMatch {
        bg_id: 2,
        team_a: vec![1],
        team_b: vec![2],
    };
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

#[test]
fn kill_deducts_reinforcements() {
    let tmpl = alterac_valley();
    let bg_match = BgMatch {
        bg_id: 3,
        team_a: vec![1],
        team_b: vec![2],
    };
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
    let bg_match = BgMatch {
        bg_id: 3,
        team_a: vec![1],
        team_b: vec![2],
    };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    for _ in 0..600 {
        on_kill_reinforcement(&mut inst, BgTeam::B);
    }
    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
}
