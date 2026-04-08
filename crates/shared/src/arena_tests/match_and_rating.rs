use super::*;

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
    assert_eq!(winner.change, 16);
    assert_eq!(loser.change, -16);
    assert_eq!(winner.new_rating, 1516);
    assert_eq!(loser.new_rating, 1484);
}

#[test]
fn rating_upset_rewards_more() {
    let (winner, loser) = calculate_rating(1400, 1600);
    assert!(winner.change > 16);
    assert!(loser.change < -16);
}

#[test]
fn rating_expected_win_rewards_less() {
    let (winner, loser) = calculate_rating(1600, 1400);
    assert!(winner.change < 16);
    assert!(winner.change > 0);
    assert!(loser.change > -16);
    assert!(loser.change < 0);
}

#[test]
fn rating_never_goes_negative() {
    let (_, loser) = calculate_rating(16, 16);
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
    let (winner, _) = calculate_rating(2000, 1000);
    assert!(winner.change <= 1, "change was {}", winner.change);
}
