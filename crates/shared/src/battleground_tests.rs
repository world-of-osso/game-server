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
