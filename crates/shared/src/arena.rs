use serde::{Deserialize, Serialize};

/// Arena bracket sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArenaBracket {
    /// 2v2 arena.
    TwoVTwo,
    /// 3v3 arena.
    ThreeVThree,
    /// 5v5 arena.
    FiveVFive,
}

impl ArenaBracket {
    /// Team size for this bracket.
    pub fn team_size(self) -> usize {
        match self {
            Self::TwoVTwo => 2,
            Self::ThreeVThree => 3,
            Self::FiveVFive => 5,
        }
    }
}

/// Default starting rating for new arena teams.
const DEFAULT_RATING: u32 = 1500;
/// Maximum roster size (active + bench).
const MAX_ROSTER_SIZE: usize = 10;

/// A persistent arena team with roster and rating.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaTeam {
    /// Unique team ID.
    pub id: u64,
    /// Team display name.
    pub name: String,
    /// Bracket (2v2, 3v3, or 5v5).
    pub bracket: ArenaBracket,
    /// Team captain (entity bits).
    pub captain: u64,
    /// Roster of player entity bits (includes captain).
    pub roster: Vec<u64>,
    /// Current team rating (Matchmaking Rating).
    pub rating: u32,
}

/// Why an arena team operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaTeamError {
    /// Team name is empty.
    EmptyName,
    /// Roster is full.
    RosterFull,
    /// Player is already on the roster.
    AlreadyOnRoster,
    /// Player is not on the roster.
    NotOnRoster,
    /// Cannot remove the captain.
    CannotRemoveCaptain,
    /// Not enough roster members to queue (need at least bracket team_size).
    NotEnoughMembers,
}

impl ArenaTeam {
    /// Create a new arena team with the captain as the sole member.
    pub fn new(id: u64, name: String, bracket: ArenaBracket, captain: u64) -> Result<Self, ArenaTeamError> {
        if name.is_empty() {
            return Err(ArenaTeamError::EmptyName);
        }
        Ok(Self {
            id,
            name,
            bracket,
            captain,
            roster: vec![captain],
            rating: DEFAULT_RATING,
        })
    }

    /// Add a player to the roster.
    pub fn add_member(&mut self, player: u64) -> Result<(), ArenaTeamError> {
        if self.roster.len() >= MAX_ROSTER_SIZE {
            return Err(ArenaTeamError::RosterFull);
        }
        if self.roster.contains(&player) {
            return Err(ArenaTeamError::AlreadyOnRoster);
        }
        self.roster.push(player);
        Ok(())
    }

    /// Remove a player from the roster (captain cannot be removed).
    pub fn remove_member(&mut self, player: u64) -> Result<(), ArenaTeamError> {
        if player == self.captain {
            return Err(ArenaTeamError::CannotRemoveCaptain);
        }
        if !self.roster.contains(&player) {
            return Err(ArenaTeamError::NotOnRoster);
        }
        self.roster.retain(|&p| p != player);
        Ok(())
    }

    /// Whether the roster has enough members to queue for a match.
    pub fn can_queue(&self) -> bool {
        self.roster.len() >= self.bracket.team_size()
    }

    /// Transfer captaincy to another roster member.
    pub fn set_captain(&mut self, new_captain: u64) -> Result<(), ArenaTeamError> {
        if !self.roster.contains(&new_captain) {
            return Err(ArenaTeamError::NotOnRoster);
        }
        self.captain = new_captain;
        Ok(())
    }

    /// Number of players on the roster.
    pub fn roster_size(&self) -> usize {
        self.roster.len()
    }
}

#[cfg(test)]
mod tests {
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
        assert!(!team.can_queue()); // only 1 member
        team.add_member(2).unwrap();
        assert!(team.can_queue()); // 2 members = 2v2 ready
    }

    #[test]
    fn can_queue_3v3() {
        let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::ThreeVThree, 1).unwrap();
        team.add_member(2).unwrap();
        assert!(!team.can_queue()); // 2 < 3
        team.add_member(3).unwrap();
        assert!(team.can_queue());
    }

    #[test]
    fn can_queue_5v5() {
        let mut team = ArenaTeam::new(1, "T".into(), ArenaBracket::FiveVFive, 1).unwrap();
        for i in 2..=4 {
            team.add_member(i).unwrap();
        }
        assert!(!team.can_queue()); // 4 < 5
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
}
