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
    let mut team = ArenaTeam::new(1, "Gladiators".into(), ArenaBracket::ThreeVThree, 10).unwrap();
    team.add_member(20).unwrap();
    team.add_member(30).unwrap();
    team.rating = 1850;

    let json = serde_json::to_string(&team).unwrap();
    let restored: ArenaTeam = serde_json::from_str(&json).unwrap();
    assert_eq!(team, restored);
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
fn queue_duplicate_player_rejected() {
    let mut q = ArenaQueue::default();
    let team = make_2v2_team(1, 10, 20, 1500);
    assert_eq!(
        q.join(&team, vec![10, 10], 100),
        Err(ArenaQueueError::DuplicatePlayer)
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
