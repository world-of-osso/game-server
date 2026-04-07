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

// -- Arena Queue --

/// Maximum rating difference for a match (widens over time in real implementation).
const MAX_RATING_DIFF: u32 = 150;

/// A team queued for an arena match.
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaQueueEntry {
    /// Arena team ID.
    pub team_id: u64,
    /// Team rating at time of queuing.
    pub rating: u32,
    /// Bracket.
    pub bracket: ArenaBracket,
    /// Active players for this match (subset of roster, exactly bracket team_size).
    pub players: Vec<u64>,
    /// Server timestamp when queued.
    pub queued_at: u64,
}

/// Why joining the arena queue failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaQueueError {
    /// Team is already queued.
    AlreadyQueued,
    /// Not enough players specified (need exactly bracket team_size).
    WrongPlayerCount,
    /// A listed player is not on the team roster.
    PlayerNotOnRoster,
}

/// A successful arena match — two teams paired by rating.
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaMatch {
    pub bracket: ArenaBracket,
    pub team_a_id: u64,
    pub team_a_players: Vec<u64>,
    pub team_a_rating: u32,
    pub team_b_id: u64,
    pub team_b_players: Vec<u64>,
    pub team_b_rating: u32,
}

/// Arena matchmaking queue.
///
/// Teams queue by bracket. Matchmaking pairs teams with similar ratings
/// (within `MAX_RATING_DIFF`).
#[derive(Debug, Clone, Default)]
pub struct ArenaQueue {
    entries: Vec<ArenaQueueEntry>,
}

impl ArenaQueue {
    /// Queue a team for arena. `players` must be exactly `bracket.team_size()`
    /// members from the team's roster.
    pub fn join(
        &mut self,
        team: &ArenaTeam,
        players: Vec<u64>,
        now: u64,
    ) -> Result<(), ArenaQueueError> {
        if self.is_queued(team.id) {
            return Err(ArenaQueueError::AlreadyQueued);
        }
        if players.len() != team.bracket.team_size() {
            return Err(ArenaQueueError::WrongPlayerCount);
        }
        if !players.iter().all(|p| team.roster.contains(p)) {
            return Err(ArenaQueueError::PlayerNotOnRoster);
        }
        self.entries.push(ArenaQueueEntry {
            team_id: team.id,
            rating: team.rating,
            bracket: team.bracket,
            players,
            queued_at: now,
        });
        Ok(())
    }

    /// Remove a team from the queue.
    pub fn leave(&mut self, team_id: u64) -> bool {
        let had = self.entries.iter().any(|e| e.team_id == team_id);
        self.entries.retain(|e| e.team_id != team_id);
        had
    }

    /// Whether a team is currently queued.
    pub fn is_queued(&self, team_id: u64) -> bool {
        self.entries.iter().any(|e| e.team_id == team_id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Try to form a match in the given bracket.
    ///
    /// Finds two teams with rating difference ≤ `MAX_RATING_DIFF`.
    /// Returns the matched pair (removed from queue), or `None`.
    pub fn try_match(&mut self, bracket: ArenaBracket) -> Option<ArenaMatch> {
        let candidates: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.bracket == bracket)
            .map(|(i, _)| i)
            .collect();

        let pair = find_closest_pair(&self.entries, &candidates)?;
        let (idx_a, idx_b) = pair;

        let a = &self.entries[idx_a];
        let b = &self.entries[idx_b];
        let result = ArenaMatch {
            bracket,
            team_a_id: a.team_id,
            team_a_players: a.players.clone(),
            team_a_rating: a.rating,
            team_b_id: b.team_id,
            team_b_players: b.players.clone(),
            team_b_rating: b.rating,
        };

        // Remove in reverse index order
        let (first, second) = if idx_a > idx_b {
            (idx_a, idx_b)
        } else {
            (idx_b, idx_a)
        };
        self.entries.remove(first);
        self.entries.remove(second);

        Some(result)
    }
}

/// Find the closest-rated pair of teams within `MAX_RATING_DIFF`.
fn find_closest_pair(entries: &[ArenaQueueEntry], candidates: &[usize]) -> Option<(usize, usize)> {
    if candidates.len() < 2 {
        return None;
    }
    let mut best: Option<(usize, usize, u32)> = None;
    for (i, &idx_a) in candidates.iter().enumerate() {
        for &idx_b in &candidates[i + 1..] {
            let diff = entries[idx_a].rating.abs_diff(entries[idx_b].rating);
            if diff > MAX_RATING_DIFF {
                continue;
            }
            let is_closer = best.is_none_or(|(_, _, d)| diff < d);
            if is_closer {
                best = Some((idx_a, idx_b, diff));
            }
        }
    }
    best.map(|(a, b, _)| (a, b))
}

// -- Arena Match State Machine --

/// Gate preparation time before combat starts (seconds).
const GATE_DURATION_SECS: u32 = 60;
/// Time into the match when dampening begins (seconds).
const DAMPENING_START_SECS: u32 = 300;
/// Dampening increases by this percentage per tick after it starts.
const DAMPENING_PER_TICK: f32 = 1.0;
/// Maximum dampening percentage.
const DAMPENING_CAP: f32 = 100.0;

/// Phase of an arena match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaPhase {
    /// Gates are closed; players prepare (buffs, positioning).
    Gates,
    /// Gates opened; combat is live.
    Combat,
    /// Match ended — one team eliminated or all left.
    Ended,
}

/// Which arena team won.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaOutcome {
    TeamA,
    TeamB,
    /// Both teams eliminated simultaneously (extremely rare).
    Draw,
}

/// A live arena match instance.
#[derive(Debug, Clone, PartialEq)]
pub struct ArenaInstance {
    pub bracket: ArenaBracket,
    pub phase: ArenaPhase,
    pub team_a_id: u64,
    pub team_b_id: u64,
    pub team_a_alive: Vec<u64>,
    pub team_b_alive: Vec<u64>,
    pub elapsed_secs: u32,
    /// Current healing reduction percentage (0.0–100.0).
    pub dampening: f32,
    pub outcome: Option<ArenaOutcome>,
}

impl ArenaInstance {
    /// Create from a queue match result.
    pub fn from_match(arena_match: &ArenaMatch) -> Self {
        Self {
            bracket: arena_match.bracket,
            phase: ArenaPhase::Gates,
            team_a_id: arena_match.team_a_id,
            team_b_id: arena_match.team_b_id,
            team_a_alive: arena_match.team_a_players.clone(),
            team_b_alive: arena_match.team_b_players.clone(),
            elapsed_secs: 0,
            dampening: 0.0,
            outcome: None,
        }
    }

    /// Open gates and begin combat. Returns false if not in Gates phase.
    pub fn open_gates(&mut self) -> bool {
        if self.phase != ArenaPhase::Gates {
            return false;
        }
        self.phase = ArenaPhase::Combat;
        true
    }

    /// Advance time. During Gates phase, auto-opens gates after `GATE_DURATION_SECS`.
    /// During Combat phase, applies dampening after `DAMPENING_START_SECS`.
    pub fn tick(&mut self, delta_secs: u32) {
        if self.phase == ArenaPhase::Ended {
            return;
        }
        self.elapsed_secs += delta_secs;
        if self.phase == ArenaPhase::Gates && self.elapsed_secs >= GATE_DURATION_SECS {
            self.phase = ArenaPhase::Combat;
        }
        if self.phase == ArenaPhase::Combat && self.elapsed_secs > DAMPENING_START_SECS {
            self.dampening = (self.dampening + DAMPENING_PER_TICK).min(DAMPENING_CAP);
        }
    }

    /// Record a player death. Checks win condition after removal.
    pub fn eliminate(&mut self, player: u64) {
        if self.phase != ArenaPhase::Combat {
            return;
        }
        self.team_a_alive.retain(|&p| p != player);
        self.team_b_alive.retain(|&p| p != player);
        self.check_win();
    }

    /// Current dampening as a multiplier (e.g. 0.75 means 75% healing).
    pub fn healing_multiplier(&self) -> f32 {
        1.0 - self.dampening / 100.0
    }

    fn check_win(&mut self) {
        let a_dead = self.team_a_alive.is_empty();
        let b_dead = self.team_b_alive.is_empty();
        let outcome = match (a_dead, b_dead) {
            (true, true) => Some(ArenaOutcome::Draw),
            (true, false) => Some(ArenaOutcome::TeamB),
            (false, true) => Some(ArenaOutcome::TeamA),
            (false, false) => None,
        };
        if let Some(result) = outcome {
            self.phase = ArenaPhase::Ended;
            self.outcome = Some(result);
        }
    }
}

#[cfg(test)]
#[path = "arena_tests.rs"]
mod tests;
