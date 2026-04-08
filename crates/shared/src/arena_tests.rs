use super::*;

fn make_2v2_team(id: u64, p1: u64, p2: u64, rating: u32) -> ArenaTeam {
    let mut team = ArenaTeam::new(id, format!("Team{id}"), ArenaBracket::TwoVTwo, p1).unwrap();
    team.add_member(p2).unwrap();
    team.rating = rating;
    team
}

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

#[path = "arena_tests/match_and_rating.rs"]
mod match_and_rating;
#[path = "arena_tests/season_and_integration.rs"]
mod season_and_integration;
#[path = "arena_tests/team_and_queue.rs"]
mod team_and_queue;
