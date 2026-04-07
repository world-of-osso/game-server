use super::*;

#[test]
fn bracket_team_sizes() {
    assert_eq!(ArenaBracket::TwoVTwo.team_size(), 2);
    assert_eq!(ArenaBracket::ThreeVThree.team_size(), 3);
    assert_eq!(ArenaBracket::FiveVFive.team_size(), 5);
}

#[test]
fn create_team() {
    let team = ArenaTeam::new(1, "Test Team".into(), ArenaBracket::ThreeVThree, 100).unwrap();
    assert_eq!(team.id, 1);
    assert_eq!(team.name, "Test Team");
    assert_eq!(team.bracket, ArenaBracket::ThreeVThree);
    assert_eq!(team.captain, 100);
    assert_eq!(team.roster, vec![100]);
    assert_eq!(team.rating, 1500);
}

#[test]
fn create_team_empty_name() {
    let result = ArenaTeam::new(1, "".into(), ArenaBracket::TwoVTwo, 100);
    assert_eq!(result, Err(ArenaTeamError::EmptyName));
}

#[test]
fn add_member() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    team.add_member(2).unwrap();
    assert_eq!(team.roster, vec![1, 2]);
    assert_eq!(team.roster_size(), 2);
}

#[test]
fn add_duplicate_member() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    assert_eq!(team.add_member(1), Err(ArenaTeamError::AlreadyOnRoster));
}

#[test]
fn add_member_roster_full() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    for i in 2..=10 {
        team.add_member(i).unwrap();
    }
    assert_eq!(team.roster_size(), 10);
    assert_eq!(team.add_member(11), Err(ArenaTeamError::RosterFull));
}

#[test]
fn remove_member() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    team.add_member(2).unwrap();
    team.remove_member(2).unwrap();
    assert_eq!(team.roster, vec![1]);
}

#[test]
fn remove_captain_fails() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    assert_eq!(
        team.remove_member(1),
        Err(ArenaTeamError::CannotRemoveCaptain)
    );
}

#[test]
fn remove_nonmember() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    assert_eq!(team.remove_member(99), Err(ArenaTeamError::NotOnRoster));
}

#[test]
fn can_queue_2v2() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    assert!(!team.can_queue());
    team.add_member(2).unwrap();
    assert!(team.can_queue());
}

#[test]
fn can_queue_3v3() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::ThreeVThree, 1).unwrap();
    team.add_member(2).unwrap();
    assert!(!team.can_queue());
    team.add_member(3).unwrap();
    assert!(team.can_queue());
}

#[test]
fn can_queue_5v5() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::FiveVFive, 1).unwrap();
    for i in 2..=4 {
        team.add_member(i).unwrap();
    }
    assert!(!team.can_queue());
    team.add_member(5).unwrap();
    assert!(team.can_queue());
}

#[test]
fn set_captain() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    team.add_member(2).unwrap();
    team.set_captain(2).unwrap();
    assert_eq!(team.captain, 2);
}

#[test]
fn set_captain_not_on_roster() {
    let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    assert_eq!(team.set_captain(99), Err(ArenaTeamError::NotOnRoster));
}

#[test]
fn default_rating_is_1500() {
    let team = ArenaTeam::new(1, "T".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    assert_eq!(team.rating, 1500);
}

#[test]
fn serialization_round_trip() {
    let mut team =
        ArenaTeam::new(1, "Gladiators".into(), ArenaBracket::ThreeVThree, 10).unwrap();
    team.add_member(20).unwrap();
    team.add_member(30).unwrap();
    team.rating = 1850;

    let json = serde_json::to_string(&team).unwrap();
    let restored: ArenaTeam = serde_json::from_str(&json).unwrap();
    assert_eq!(team, restored);
}

// -- Queue tests --

fn make_2v2_team(id: u64, p1: u64, p2: u64, rating: u32) -> ArenaTeam {
    let mut team = ArenaTeam::new(id, format!("Team{id}"), ArenaBracket::TwoVTwo, p1).unwrap();
    team.add_member(p2).unwrap();
    team.rating = rating;
    team
}

#[test]
fn queue_join() {
    let mut q = ArenaQueue::default();
    let team = make_2v2_team(1, 10, 20, 1500);
    q.join(&team, vec![10, 20], 100).unwrap();
    assert!(q.is_queued(1));
    assert_eq!(q.len(), 1);
}

#[test]
fn queue_already_queued() {
    let mut q = ArenaQueue::default();
    let team = make_2v2_team(1, 10, 20, 1500);
    q.join(&team, vec![10, 20], 100).unwrap();
    assert_eq!(
        q.join(&team, vec![10, 20], 101),
        Err(ArenaQueueError::AlreadyQueued)
    );
}

#[test]
fn queue_wrong_player_count() {
    let mut q = ArenaQueue::default();
    let team = make_2v2_team(1, 10, 20, 1500);
    assert_eq!(
        q.join(&team, vec![10], 100),
        Err(ArenaQueueError::WrongPlayerCount)
    );
}

#[test]
fn queue_player_not_on_roster() {
    let mut q = ArenaQueue::default();
    let team = make_2v2_team(1, 10, 20, 1500);
    assert_eq!(
        q.join(&team, vec![10, 99], 100),
        Err(ArenaQueueError::PlayerNotOnRoster)
    );
}

#[test]
fn queue_leave() {
    let mut q = ArenaQueue::default();
    let team = make_2v2_team(1, 10, 20, 1500);
    q.join(&team, vec![10, 20], 100).unwrap();
    assert!(q.leave(1));
    assert!(!q.is_queued(1));
    assert!(q.is_empty());
}

#[test]
fn match_close_ratings() {
    let mut q = ArenaQueue::default();
    let t1 = make_2v2_team(1, 10, 20, 1500);
    let t2 = make_2v2_team(2, 30, 40, 1550);
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();

    let m = q.try_match(ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(m.bracket, ArenaBracket::TwoVTwo);
    let ids = [m.team_a_id, m.team_b_id];
    assert!(ids.contains(&1));
    assert!(ids.contains(&2));
    assert!(q.is_empty());
}

#[test]
fn match_ratings_too_far_apart() {
    let mut q = ArenaQueue::default();
    let t1 = make_2v2_team(1, 10, 20, 1500);
    let t2 = make_2v2_team(2, 30, 40, 1700);
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();

    assert!(q.try_match(ArenaBracket::TwoVTwo).is_none());
    assert_eq!(q.len(), 2);
}

#[test]
fn match_picks_closest_pair() {
    let mut q = ArenaQueue::default();
    let t1 = make_2v2_team(1, 10, 20, 1500);
    let t2 = make_2v2_team(2, 30, 40, 1600);
    let t3 = make_2v2_team(3, 50, 60, 1510);
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();
    q.join(&t3, vec![50, 60], 100).unwrap();

    let m = q.try_match(ArenaBracket::TwoVTwo).unwrap();
    let ids = [m.team_a_id, m.team_b_id];
    assert!(ids.contains(&1));
    assert!(ids.contains(&3));
    assert_eq!(q.len(), 1);
    assert!(q.is_queued(2));
}

#[test]
fn match_wrong_bracket_ignored() {
    let mut q = ArenaQueue::default();
    let t1 = make_2v2_team(1, 10, 20, 1500);
    let t2 = make_2v2_team(2, 30, 40, 1500);
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();

    assert!(q.try_match(ArenaBracket::ThreeVThree).is_none());
    assert_eq!(q.len(), 2);
}

#[test]
fn match_preserves_player_lists() {
    let mut q = ArenaQueue::default();
    let t1 = make_2v2_team(1, 10, 20, 1500);
    let t2 = make_2v2_team(2, 30, 40, 1500);
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();

    let m = q.try_match(ArenaBracket::TwoVTwo).unwrap();
    let all_players: Vec<u64> = m
        .team_a_players
        .iter()
        .chain(&m.team_b_players)
        .copied()
        .collect();
    assert_eq!(all_players.len(), 4);
    for p in [10, 20, 30, 40] {
        assert!(all_players.contains(&p));
    }
}

#[test]
fn match_at_exact_boundary() {
    let mut q = ArenaQueue::default();
    let t1 = make_2v2_team(1, 10, 20, 1500);
    let t2 = make_2v2_team(2, 30, 40, 1650);
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();

    assert!(q.try_match(ArenaBracket::TwoVTwo).is_some());
}

#[test]
fn match_just_over_boundary() {
    let mut q = ArenaQueue::default();
    let t1 = make_2v2_team(1, 10, 20, 1500);
    let t2 = make_2v2_team(2, 30, 40, 1651);
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();

    assert!(q.try_match(ArenaBracket::TwoVTwo).is_none());
}

// -- Match state machine tests --

fn make_2v2_match() -> ArenaMatch {
    ArenaMatch {
        bracket: ArenaBracket::TwoVTwo,
        team_a_id: 1,
        team_a_players: vec![10, 20],
        team_a_rating: 1500,
        team_b_id: 2,
        team_b_players: vec![30, 40],
        team_b_rating: 1500,
    }
}

#[test]
fn instance_starts_in_gates() {
    let inst = ArenaInstance::from_match(&make_2v2_match());
    assert_eq!(inst.phase, ArenaPhase::Gates);
    assert_eq!(inst.elapsed_secs, 0);
    assert_eq!(inst.dampening, 0.0);
    assert!(inst.outcome.is_none());
}

#[test]
fn open_gates_transitions_to_combat() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    assert!(inst.open_gates());
    assert_eq!(inst.phase, ArenaPhase::Combat);
}

#[test]
fn open_gates_twice_fails() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    assert!(!inst.open_gates());
}

#[test]
fn tick_auto_opens_gates() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.tick(59);
    assert_eq!(inst.phase, ArenaPhase::Gates);
    inst.tick(1);
    assert_eq!(inst.phase, ArenaPhase::Combat);
}

#[test]
fn eliminate_all_team_a_wins_for_b() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    inst.eliminate(10);
    assert_eq!(inst.phase, ArenaPhase::Combat);
    inst.eliminate(20);
    assert_eq!(inst.phase, ArenaPhase::Ended);
    assert_eq!(inst.outcome, Some(ArenaOutcome::TeamB));
}

#[test]
fn eliminate_all_team_b_wins_for_a() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    inst.eliminate(30);
    inst.eliminate(40);
    assert_eq!(inst.outcome, Some(ArenaOutcome::TeamA));
}

#[test]
fn eliminate_during_gates_ignored() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.eliminate(10);
    assert_eq!(inst.team_a_alive.len(), 2);
}

#[test]
fn dampening_starts_after_300s() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    inst.tick(300);
    assert_eq!(inst.dampening, 0.0);
    inst.tick(1);
    assert!(inst.dampening > 0.0);
}

#[test]
fn dampening_increases_each_tick() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    inst.tick(301);
    let first = inst.dampening;
    inst.tick(1);
    assert!(inst.dampening > first);
}

#[test]
fn dampening_caps_at_100() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    for _ in 0..500 {
        inst.tick(1);
    }
    assert!(inst.dampening <= 100.0);
}

#[test]
fn healing_multiplier_at_zero_dampening() {
    let inst = ArenaInstance::from_match(&make_2v2_match());
    assert_eq!(inst.healing_multiplier(), 1.0);
}

#[test]
fn healing_multiplier_at_50_percent() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.dampening = 50.0;
    assert!((inst.healing_multiplier() - 0.5).abs() < 0.001);
}

#[test]
fn tick_after_ended_no_effect() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    inst.eliminate(10);
    inst.eliminate(20);
    let elapsed = inst.elapsed_secs;
    inst.tick(100);
    assert_eq!(inst.elapsed_secs, elapsed);
}

#[test]
fn alive_lists_track_eliminations() {
    let mut inst = ArenaInstance::from_match(&make_2v2_match());
    inst.open_gates();
    inst.eliminate(10);
    assert_eq!(inst.team_a_alive, vec![20]);
    assert_eq!(inst.team_b_alive, vec![30, 40]);
}

// -- Rating tests --

#[test]
fn expected_score_equal_ratings() {
    let e = expected_score(1500, 1500);
    assert!((e - 0.5).abs() < 0.001);
}

#[test]
fn expected_score_higher_rated_favored() {
    let e = expected_score(1600, 1400);
    assert!(e > 0.5);
    assert!(e < 1.0);
}

#[test]
fn expected_score_lower_rated_underdog() {
    let e = expected_score(1400, 1600);
    assert!(e < 0.5);
    assert!(e > 0.0);
}

#[test]
fn expected_scores_symmetric() {
    let e_a = expected_score(1500, 1600);
    let e_b = expected_score(1600, 1500);
    assert!((e_a + e_b - 1.0).abs() < 0.001);
}

#[test]
fn rating_equal_teams() {
    let (winner, loser) = calculate_rating(1500, 1500);
    // Equal teams: winner gains 16, loser loses 16 (K/2)
    assert_eq!(winner.change, 16);
    assert_eq!(loser.change, -16);
    assert_eq!(winner.new_rating, 1516);
    assert_eq!(loser.new_rating, 1484);
}

#[test]
fn rating_upset_rewards_more() {
    // Lower-rated team beats higher-rated team
    let (winner, loser) = calculate_rating(1400, 1600);
    // Underdog win: larger gain
    assert!(winner.change > 16);
    // Favored loss: larger loss
    assert!(loser.change < -16);
}

#[test]
fn rating_expected_win_rewards_less() {
    // Higher-rated team beats lower-rated team
    let (winner, loser) = calculate_rating(1600, 1400);
    // Expected win: smaller gain
    assert!(winner.change < 16);
    assert!(winner.change > 0);
    // Expected loss: smaller loss
    assert!(loser.change > -16);
    assert!(loser.change < 0);
}

#[test]
fn rating_never_goes_negative() {
    // Equal ratings, loser with very low rating
    let (_, loser) = calculate_rating(16, 16);
    // loser loses 16 points: 16 - 16 = 0
    assert_eq!(loser.new_rating, 0);
    assert!(loser.change < 0);
}

#[test]
fn rating_preserves_old_rating() {
    let (winner, loser) = calculate_rating(1500, 1500);
    assert_eq!(winner.old_rating, 1500);
    assert_eq!(loser.old_rating, 1500);
}

#[test]
fn rating_changes_are_opposite_sign() {
    let (winner, loser) = calculate_rating(1500, 1500);
    assert!(winner.change > 0);
    assert!(loser.change < 0);
}

#[test]
fn rating_large_gap_winner_gains_almost_nothing() {
    // 2000 vs 1000: overwhelming favorite wins
    let (winner, _) = calculate_rating(2000, 1000);
    assert!(winner.change <= 1, "change was {}", winner.change);
}
