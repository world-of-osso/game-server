use super::*;

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
    assert_eq!(stats.highest_rating, 1500);
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
    assert_eq!(stats.win_rate(), 0.0);
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

    let stats = history.get_or_create(1, 1, ArenaBracket::TwoVTwo);
    assert_eq!(stats.wins, 1);
    assert_eq!(history.entries.len(), 1);
}

#[test]
fn history_different_brackets_separate() {
    let mut history = PlayerSeasonHistory::default();
    history
        .get_or_create(1, 1, ArenaBracket::TwoVTwo)
        .record_win(1516);
    history
        .get_or_create(1, 1, ArenaBracket::ThreeVThree)
        .record_loss(1484);

    assert_eq!(history.entries.len(), 2);
    let two = history.get(1, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(two.wins, 1);
    let three = history.get(1, 1, ArenaBracket::ThreeVThree).unwrap();
    assert_eq!(three.losses, 1);
}

#[test]
fn history_different_seasons_separate() {
    let mut history = PlayerSeasonHistory::default();
    history
        .get_or_create(1, 1, ArenaBracket::TwoVTwo)
        .record_win(1600);
    history.get_or_create(1, 2, ArenaBracket::TwoVTwo);

    let s1 = history.get(1, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(s1.current_rating, 1600);
    let s2 = history.get(1, 2, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(s2.current_rating, 1500);
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

#[test]
fn full_pipeline_queue_match_combat_rating_season() {
    let mut t1 = ArenaTeam::new(1, "Alpha".into(), ArenaBracket::TwoVTwo, 10).unwrap();
    t1.add_member(20).unwrap();
    t1.rating = 1500;

    let mut t2 = ArenaTeam::new(2, "Beta".into(), ArenaBracket::TwoVTwo, 30).unwrap();
    t2.add_member(40).unwrap();
    t2.rating = 1550;

    let mut q = ArenaQueue::default();
    q.join(&t1, vec![10, 20], 100).unwrap();
    q.join(&t2, vec![30, 40], 100).unwrap();
    let arena_match = q.try_match(ArenaBracket::TwoVTwo).unwrap();
    assert!(q.is_empty());

    let mut inst = ArenaInstance::from_match(&arena_match);
    inst.tick(60);
    assert_eq!(inst.phase, ArenaPhase::Combat);

    inst.eliminate(arena_match.team_b_players[0]);
    inst.eliminate(arena_match.team_b_players[1]);
    assert_eq!(inst.phase, ArenaPhase::Ended);
    assert_eq!(inst.outcome, Some(ArenaOutcome::TeamA));

    let (winner_adj, loser_adj) =
        calculate_rating(arena_match.team_a_rating, arena_match.team_b_rating);
    assert!(winner_adj.change > 0);
    assert!(loser_adj.change < 0);

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

    let p10 = history.get(10, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(p10.wins, 1);
    assert_eq!(p10.losses, 0);
    assert!(p10.current_rating > 1500);
    assert_eq!(p10.highest_rating, p10.current_rating);

    let p30 = history.get(30, 1, ArenaBracket::TwoVTwo).unwrap();
    assert_eq!(p30.wins, 0);
    assert_eq!(p30.losses, 1);
    assert!(p30.current_rating < 1550);
}

#[test]
fn dampening_escalates_during_long_match() {
    let arena_match = make_2v2_match();
    let mut inst = ArenaInstance::from_match(&arena_match);
    inst.open_gates();

    assert_eq!(inst.healing_multiplier(), 1.0);

    for _ in 0..300 {
        inst.tick(1);
    }
    assert_eq!(inst.dampening, 0.0);
    assert_eq!(inst.healing_multiplier(), 1.0);

    for _ in 0..50 {
        inst.tick(1);
    }
    assert!(inst.dampening > 0.0);
    assert!(inst.healing_multiplier() < 1.0);

    let healing = inst.healing_multiplier();
    for _ in 0..50 {
        inst.tick(1);
    }
    assert!(inst.healing_multiplier() < healing);
}

#[test]
fn rating_reflects_in_highest_after_streak() {
    let mut history = PlayerSeasonHistory::default();
    let stats = history.get_or_create(1, 1, ArenaBracket::TwoVTwo);

    stats.record_win(1516);
    stats.record_win(1532);
    stats.record_win(1548);
    assert_eq!(stats.highest_rating, 1548);

    stats.record_loss(1532);
    assert_eq!(stats.highest_rating, 1548);
    assert_eq!(stats.current_rating, 1532);

    stats.record_win(1560);
    assert_eq!(stats.highest_rating, 1560);
}

#[test]
fn mmr_matching_prefers_close_ratings_over_time() {
    let mut q = ArenaQueue::default();

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

    let m = q.try_match(ArenaBracket::TwoVTwo).unwrap();
    let ids = [m.team_a_id, m.team_b_id];
    assert!(ids.contains(&2));
    assert!(ids.contains(&3));
    assert!(q.is_queued(1));
    assert_eq!(q.len(), 1);
}

#[test]
fn multiple_seasons_independent() {
    let mut history = PlayerSeasonHistory::default();

    let s1 = history.get_or_create(1, 1, ArenaBracket::ThreeVThree);
    s1.record_win(1600);
    s1.record_win(1700);
    s1.record_win(1800);

    let s2 = history.get_or_create(1, 2, ArenaBracket::ThreeVThree);
    assert_eq!(s2.current_rating, 1500);
    assert_eq!(s2.games_played(), 0);
    s2.record_loss(1484);

    let s1 = history.get(1, 1, ArenaBracket::ThreeVThree).unwrap();
    assert_eq!(s1.highest_rating, 1800);
    assert_eq!(s1.games_played(), 3);

    let s2 = history.get(1, 2, ArenaBracket::ThreeVThree).unwrap();
    assert_eq!(s2.current_rating, 1484);
    assert_eq!(s2.games_played(), 1);
}

#[test]
fn upset_win_gives_bigger_rating_gain() {
    let (underdog_adj, _) = calculate_rating(1400, 1600);
    let (equal_adj, _) = calculate_rating(1500, 1500);

    assert!(
        underdog_adj.change > equal_adj.change,
        "upset ({}) should gain more than equal match ({})",
        underdog_adj.change,
        equal_adj.change,
    );
}
