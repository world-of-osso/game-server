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

// -- Season tracking tests --

#[test]
fn season_stats_new() {
    let stats = SeasonStats::new(1, 1, ArenaBracket::TwoVTwo);
    assert_eq!(stats.current_rating, 1500);
    assert_eq!(stats.highest_rating, 1500);
    assert_eq!(stats.wins, 0);
    assert_eq!(stats.losses, 0);
    assert_eq!(stats.games_played(), 0);
}

#[test]
fn season_stats_record_win() {
    let mut stats = SeasonStats::new(1, 1, ArenaBracket::TwoVTwo);
    stats.record_win(1516);
    assert_eq!(stats.wins, 1);
    assert_eq!(stats.current_rating, 1516);
    assert_eq!(stats.highest_rating, 1516);
    assert_eq!(stats.games_played(), 1);
}

#[test]
fn season_stats_record_loss() {
    let mut stats = SeasonStats::new(1, 1, ArenaBracket::TwoVTwo);
    stats.record_loss(1484);
    assert_eq!(stats.losses, 1);
    assert_eq!(stats.current_rating, 1484);
    assert_eq!(stats.highest_rating, 1500); // unchanged
}

#[test]
fn season_stats_highest_rating_tracked() {
    let mut stats = SeasonStats::new(1, 1, ArenaBracket::TwoVTwo);
    stats.record_win(1600);
    stats.record_loss(1570);
    stats.record_win(1590);
    assert_eq!(stats.highest_rating, 1600);
    assert_eq!(stats.current_rating, 1590);
}

#[test]
fn season_stats_win_rate() {
    let mut stats = SeasonStats::new(1, 1, ArenaBracket::TwoVTwo);
    assert_eq!(stats.win_rate(), 0.0); // no games
    stats.record_win(1516);
    stats.record_win(1532);
    stats.record_loss(1516);
    assert!((stats.win_rate() - 2.0 / 3.0).abs() < 0.01);
}

#[test]
fn history_get_or_create() {
    let mut history = PlayerSeasonHistory::default();
    let stats = history.get_or_create(1, 1, ArenaBracket::TwoVTwo);
    stats.record_win(1516);

    // Second call returns same entry
    let stats = history.get_or_create(1, 1, ArenaBracket::TwoVTwo);
    assert_eq!(stats.wins, 1);
    assert_eq!(history.entries.len(), 1);
}

#[test]
fn history_different_brackets_separate() {
    let mut history = PlayerSeasonHistory::default();
    history.get_or_create(1, 1, ArenaBracket::TwoVTwo).record_win(1516);
    history.get_or_create(1, 1, ArenaBracket::ThreeVThree).record_loss(1484);

    assert_eq!(history.entries.len(), 2);
    let two = history.get(1, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(two.wins, 1);
    let three = history.get(1, 1, ArenaBracket::ThreeVThree).unwrap();
    assert_eq!(three.losses, 1);
}

#[test]
fn history_different_seasons_separate() {
    let mut history = PlayerSeasonHistory::default();
    history.get_or_create(1, 1, ArenaBracket::TwoVTwo).record_win(1600);
    history.get_or_create(1, 2, ArenaBracket::TwoVTwo); // new season, fresh stats

    let s1 = history.get(1, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(s1.current_rating, 1600);
    let s2 = history.get(1, 2, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(s2.current_rating, 1500); // fresh
}

#[test]
fn history_for_player() {
    let mut history = PlayerSeasonHistory::default();
    history.get_or_create(1, 1, ArenaBracket::TwoVTwo);
    history.get_or_create(1, 1, ArenaBracket::ThreeVThree);
    history.get_or_create(2, 1, ArenaBracket::TwoVTwo);

    assert_eq!(history.for_player(1).len(), 2);
    assert_eq!(history.for_player(2).len(), 1);
    assert_eq!(history.for_player(99).len(), 0);
}

#[test]
fn history_get_nonexistent() {
    let history = PlayerSeasonHistory::default();
    assert!(history.get(1, 1, ArenaBracket::TwoVTwo).is_none());
}

// -- Integration tests: full arena pipeline --

#[test]
fn full_pipeline_queue_match_combat_rating_season() {
    // 1. Create two teams
    let mut t1 = ArenaTeam::new(1, "Alpha".into(), ArenaBracket::TwoVTwo, 10).unwrap();
    t1.add_member(20).unwrap();
    t1.rating = 1500;

    let mut t2 = ArenaTeam::new(2, "Beta".into(), ArenaBracket::TwoVTwo, 30).unwrap();
    t2.add_member(40).unwrap();
    t2.rating = 1550;

    // 2. Queue and match
    let mut q = ArenaQueue::default();
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();
    let arena_match = q.try_match(ArenaBracket::TwoVTwo).unwrap();
    assert!(q.is_empty());

    // 3. Play the match: gates → combat → team A eliminates team B
    let mut inst = ArenaInstance::from_match(&arena_match);
    inst.tick(60); // auto-open gates
    assert_eq!(inst.phase, ArenaPhase::Combat);

    inst.eliminate(arena_match.team_b_players[0]);
    inst.eliminate(arena_match.team_b_players[1]);
    assert_eq!(inst.phase, ArenaPhase::Ended);
    assert_eq!(inst.outcome, Some(ArenaOutcome::TeamA));

    // 4. Calculate rating adjustments
    let (winner_adj, loser_adj) = calculate_rating(
        arena_match.team_a_rating,
        arena_match.team_b_rating,
    );
    assert!(winner_adj.change > 0);
    assert!(loser_adj.change < 0);

    // 5. Update season stats
    let mut history = PlayerSeasonHistory::default();
    for &player in &arena_match.team_a_players {
        history
            .get_or_create(player, 1, ArenaBracket::TwoVTwo)
            .record_win(winner_adj.new_rating);
    }
    for &player in &arena_match.team_b_players {
        history
            .get_or_create(player, 1, ArenaBracket::TwoVTwo)
            .record_loss(loser_adj.new_rating);
    }

    // 6. Verify season stats
    let p10 = history.get(10, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(p10.wins, 1);
    assert_eq!(p10.losses, 0);
    assert!(p10.current_rating > 1500);
    assert_eq!(p10.highest_rating, p10.current_rating);

    let p30 = history.get(30, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(p30.wins, 0);
    assert_eq!(p30.losses, 1);
    assert!(p30.current_rating < 1550, "loser should drop from team rating 1550");
}

#[test]
fn dampening_escalates_during_long_match() {
    let arena_match = make_2v2_match();
    let mut inst = ArenaInstance::from_match(&arena_match);
    inst.open_gates();

    // No dampening at start
    assert_eq!(inst.healing_multiplier(), 1.0);

    // Tick to just before dampening threshold (300s)
    for _ in 0..300 {
        inst.tick(1);
    }
    assert_eq!(inst.dampening, 0.0);
    assert_eq!(inst.healing_multiplier(), 1.0);

    // Tick into dampening territory
    for _ in 0..50 {
        inst.tick(1);
    }
    assert!(inst.dampening > 0.0);
    assert!(inst.healing_multiplier() < 1.0);

    // Dampening should be substantial after many ticks
    let healing = inst.healing_multiplier();
    for _ in 0..50 {
        inst.tick(1);
    }
    assert!(inst.healing_multiplier() < healing, "dampening should keep increasing");
}

#[test]
fn rating_reflects_in_highest_after_streak() {
    let mut history = PlayerSeasonHistory::default();
    let stats = history.get_or_create(1, 1, ArenaBracket::TwoVTwo);

    // Win streak pushes rating up
    stats.record_win(1516);
    stats.record_win(1532);
    stats.record_win(1548);
    assert_eq!(stats.highest_rating, 1548);

    // Loss doesn't lower highest
    stats.record_loss(1532);
    assert_eq!(stats.highest_rating, 1548);
    assert_eq!(stats.current_rating, 1532);

    // New high replaces old
    stats.record_win(1560);
    assert_eq!(stats.highest_rating, 1560);
}

#[test]
fn mmr_matching_prefers_close_ratings_over_time() {
    let mut q = ArenaQueue::default();

    // Three teams with different ratings
    let mut t_low = ArenaTeam::new(1, "Low".into(), ArenaBracket::TwoVTwo, 1).unwrap();
    t_low.add_member(2).unwrap();
    t_low.rating = 1400;

    let mut t_mid = ArenaTeam::new(2, "Mid".into(), ArenaBracket::TwoVTwo, 3).unwrap();
    t_mid.add_member(4).unwrap();
    t_mid.rating = 1500;

    let mut t_high = ArenaTeam::new(3, "High".into(), ArenaBracket::TwoVTwo, 5).unwrap();
    t_high.add_member(6).unwrap();
    t_high.rating = 1520;

    q.join(&t_low, vec![1, 2], 100).unwrap();
    q.join(&t_mid, vec![3, 4], 100).unwrap();
    q.join(&t_high, vec![5, 6], 100).unwrap();

    // Should match Mid (1500) with High (1520) — closest pair
    let m = q.try_match(ArenaBracket::TwoVTwo).unwrap();
    let ids = [m.team_a_id, m.team_b_id];
    assert!(ids.contains(&2), "mid team should match");
    assert!(ids.contains(&3), "high team should match");

    // Low team left in queue
    assert!(q.is_queued(1));
    assert_eq!(q.len(), 1);
}

#[test]
fn multiple_seasons_independent() {
    let mut history = PlayerSeasonHistory::default();

    // Season 1: player reaches 1800
    let s1 = history.get_or_create(1, 1, ArenaBracket::ThreeVThree);
    s1.record_win(1600);
    s1.record_win(1700);
    s1.record_win(1800);

    // Season 2: fresh start at 1500
    let s2 = history.get_or_create(1, 2, ArenaBracket::ThreeVThree);
    assert_eq!(s2.current_rating, 1500);
    assert_eq!(s2.games_played(), 0);
    s2.record_loss(1484);

    // Season 1 stats unchanged
    let s1 = history.get(1, 1, ArenaBracket::ThreeVThree).unwrap();
    assert_eq!(s1.highest_rating, 1800);
    assert_eq!(s1.games_played(), 3);

    // Season 2 has its own stats
    let s2 = history.get(1, 2, ArenaBracket::ThreeVThree).unwrap();
    assert_eq!(s2.current_rating, 1484);
    assert_eq!(s2.games_played(), 1);
}

#[test]
fn upset_win_gives_bigger_rating_gain() {
    // Lower-rated team beats higher-rated
    let (underdog_adj, _) = calculate_rating(1400, 1600);
    // Equal teams
    let (equal_adj, _) = calculate_rating(1500, 1500);

    assert!(
        underdog_adj.change > equal_adj.change,
        "upset ({}) should gain more than equal match ({})",
        underdog_adj.change,
        equal_adj.change,
    );
}
