use serde::{Deserialize, Serialize};

/// Type of PvP objective for a battleground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BgObjective {
    /// Capture the enemy flag and return it to your base (e.g. Warsong Gulch).
    FlagCapture,
    /// Control territory nodes that tick score over time (e.g. Arathi Basin).
    NodeControl,
    /// Deplete the enemy team's reinforcement count (e.g. Alterac Valley).
    Reinforcements,
}

/// Level bracket for battleground matchmaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LevelBracket {
    pub min_level: u8,
    pub max_level: u8,
}

impl LevelBracket {
    pub const fn new(min_level: u8, max_level: u8) -> Self {
        Self {
            min_level,
            max_level,
        }
    }

    /// Whether a player level falls within this bracket.
    pub fn contains(&self, level: u8) -> bool {
        level >= self.min_level && level <= self.max_level
    }
}

/// Static definition of a battleground type.
///
/// Immutable data loaded at startup. Describes the rules and parameters for
/// one kind of BG (e.g. "Warsong Gulch", "Arathi Basin").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BgTemplate {
    /// Unique battleground type ID.
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Map ID for the battleground instance.
    pub map_id: u32,
    /// Players per team.
    pub team_size: u8,
    /// Minimum players per team to start (below this, match waits).
    pub min_players_per_team: u8,
    /// Primary objective type.
    pub objective: BgObjective,
    /// Maximum match duration in seconds. Match ends early if objective met.
    pub max_duration_secs: u32,
    /// Score required to win (flag captures, resource points, etc.).
    pub win_score: u32,
    /// Starting reinforcement count (only for `BgObjective::Reinforcements`).
    pub starting_reinforcements: u32,
    /// Level bracket for matchmaking.
    pub bracket: LevelBracket,
}

// -- Built-in BG templates --

/// Warsong Gulch — 10v10 capture the flag.
pub fn warsong_gulch() -> BgTemplate {
    BgTemplate {
        id: 1,
        name: "Warsong Gulch".into(),
        map_id: 489,
        team_size: 10,
        min_players_per_team: 5,
        objective: BgObjective::FlagCapture,
        max_duration_secs: 25 * 60,
        win_score: 3,
        starting_reinforcements: 0,
        bracket: LevelBracket::new(10, 60),
    }
}

/// Arathi Basin — 15v15 node control (5 bases, 1600 resources to win).
pub fn arathi_basin() -> BgTemplate {
    BgTemplate {
        id: 2,
        name: "Arathi Basin".into(),
        map_id: 529,
        team_size: 15,
        min_players_per_team: 8,
        objective: BgObjective::NodeControl,
        max_duration_secs: 25 * 60,
        win_score: 1600,
        starting_reinforcements: 0,
        bracket: LevelBracket::new(10, 60),
    }
}

/// Alterac Valley — 40v40 reinforcement depletion.
pub fn alterac_valley() -> BgTemplate {
    BgTemplate {
        id: 3,
        name: "Alterac Valley".into(),
        map_id: 30,
        team_size: 40,
        min_players_per_team: 20,
        objective: BgObjective::Reinforcements,
        max_duration_secs: 40 * 60,
        win_score: 0,
        starting_reinforcements: 600,
        bracket: LevelBracket::new(10, 60),
    }
}

/// All built-in battleground templates.
pub fn all_templates() -> Vec<BgTemplate> {
    vec![warsong_gulch(), arathi_basin(), alterac_valley()]
}

/// Find a template by ID.
pub fn template_by_id(id: u32) -> Option<BgTemplate> {
    all_templates().into_iter().find(|t| t.id == id)
}

// -- BG Queue --

/// A player or group queued for a battleground.
#[derive(Debug, Clone, PartialEq)]
pub struct BgQueueEntry {
    /// Leader entity bits (solo player or group leader).
    pub leader: u64,
    /// All member entity bits.
    pub members: Vec<u64>,
    /// Level of each member (index matches `members`).
    pub levels: Vec<u8>,
    /// Battleground type ID.
    pub bg_id: u32,
    /// Server timestamp when queued.
    pub queued_at: u64,
}

/// Why joining the BG queue failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgQueueError {
    /// Player already in the queue.
    AlreadyQueued,
    /// Unknown battleground ID.
    UnknownBg,
    /// Player level outside the BG's bracket.
    OutOfBracket,
    /// Group has no members.
    EmptyGroup,
    /// Group members and level entries do not align.
    LevelCountMismatch,
    /// Group listed the same player more than once.
    DuplicatePlayer,
    /// Player has an active deserter penalty.
    Deserter,
}

/// A successful battleground match — two teams ready to enter.
#[derive(Debug, Clone, PartialEq)]
pub struct BgMatch {
    pub bg_id: u32,
    pub team_a: Vec<u64>,
    pub team_b: Vec<u64>,
}

/// Battleground matchmaking queue.
///
/// Players join for a specific BG type. Matchmaking fills two teams from
/// queued entries, ignoring roles (unlike dungeon finder). Players are
/// assigned to teams in queue order until both teams reach `team_size`.
#[derive(Debug, Clone, Default)]
pub struct BgQueue {
    entries: Vec<BgQueueEntry>,
}

impl BgQueue {
    /// Queue a solo player.
    pub fn join_solo(
        &mut self,
        player: u64,
        level: u8,
        bg_id: u32,
        now: u64,
    ) -> Result<(), BgQueueError> {
        self.join_group(player, vec![player], vec![level], bg_id, now)
    }

    /// Queue a solo player, checking deserter status.
    pub fn join_solo_checked(
        &mut self,
        player: u64,
        level: u8,
        bg_id: u32,
        now: u64,
        deserters: &BgDeserterTracker,
    ) -> Result<(), BgQueueError> {
        if deserters.is_deserter(player, now) {
            return Err(BgQueueError::Deserter);
        }
        self.join_solo(player, level, bg_id, now)
    }

    /// Queue a group of players.
    pub fn join_group(
        &mut self,
        leader: u64,
        members: Vec<u64>,
        levels: Vec<u8>,
        bg_id: u32,
        now: u64,
    ) -> Result<(), BgQueueError> {
        if members.is_empty() {
            return Err(BgQueueError::EmptyGroup);
        }
        if members.len() != levels.len() {
            return Err(BgQueueError::LevelCountMismatch);
        }
        let Some(tmpl) = template_by_id(bg_id) else {
            return Err(BgQueueError::UnknownBg);
        };
        if !levels.iter().all(|&lvl| tmpl.bracket.contains(lvl)) {
            return Err(BgQueueError::OutOfBracket);
        }
        if has_duplicate_members(&members) {
            return Err(BgQueueError::DuplicatePlayer);
        }
        if members.iter().any(|&member| self.is_queued(member)) {
            return Err(BgQueueError::AlreadyQueued);
        }
        self.entries.push(BgQueueEntry {
            leader,
            members,
            levels,
            bg_id,
            queued_at: now,
        });
        Ok(())
    }

    /// Remove a player/group from the queue.
    pub fn leave(&mut self, leader: u64) -> bool {
        let had = self.entries.iter().any(|e| e.leader == leader);
        self.entries.retain(|e| e.leader != leader);
        had
    }

    /// Whether a player is already queued (as leader or member).
    pub fn is_queued(&self, player: u64) -> bool {
        self.entries.iter().any(|e| e.members.contains(&player))
    }

    /// Number of entries in the queue.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Try to form a match for a given BG type.
    ///
    /// Collects queued players for the BG until both teams reach
    /// `min_players_per_team`. Players are assigned alternately to balance
    /// team sizes. Returns `None` if not enough players.
    pub fn try_match(&mut self, bg_id: u32) -> Option<BgMatch> {
        let tmpl = template_by_id(bg_id)?;
        let candidate_indices = self.candidates_for(bg_id);

        let total_players: usize = candidate_indices
            .iter()
            .map(|&i| self.entries[i].members.len())
            .sum();
        let min = tmpl.min_players_per_team as usize;
        if total_players < min * 2 {
            return None;
        }

        let (team_a, team_b, used) =
            fill_teams(&self.entries, &candidate_indices, tmpl.team_size as usize);
        if team_a.len() < min || team_b.len() < min {
            return None;
        }

        remove_indices(&mut self.entries, &used);
        Some(BgMatch {
            bg_id,
            team_a,
            team_b,
        })
    }

    fn candidates_for(&self, bg_id: u32) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.bg_id == bg_id)
            .map(|(i, _)| i)
            .collect()
    }
}

fn has_duplicate_members(members: &[u64]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    members.iter().any(|member| !seen.insert(*member))
}

/// Assign queued entries alternately to two teams up to `target` size each.
fn fill_teams(
    entries: &[BgQueueEntry],
    candidates: &[usize],
    target: usize,
) -> (Vec<u64>, Vec<u64>, Vec<usize>) {
    let mut team_a = Vec::new();
    let mut team_b = Vec::new();
    let mut used = Vec::new();

    for &idx in candidates {
        let members = &entries[idx].members;
        let can_fit_a = team_a.len() + members.len() <= target;
        let can_fit_b = team_b.len() + members.len() <= target;
        let assign_to_a = can_fit_a && (team_a.len() <= team_b.len() || !can_fit_b);

        if assign_to_a {
            team_a.extend(members);
            used.push(idx);
        } else if can_fit_b {
            team_b.extend(members);
            used.push(idx);
        }
        if team_a.len() >= target && team_b.len() >= target {
            break;
        }
    }
    (team_a, team_b, used)
}

fn remove_indices<T>(vec: &mut Vec<T>, indices: &[usize]) {
    let mut sorted = indices.to_vec();
    sorted.sort_unstable();
    for idx in sorted.into_iter().rev() {
        vec.remove(idx);
    }
}

// -- BG Instance State Machine --

/// Which team a player belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BgTeam {
    A,
    B,
}

/// Phase of a battleground instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgPhase {
    /// Players are entering; waiting for min_players_per_team on each side.
    Waiting,
    /// Match is live.
    InProgress,
    /// Match has ended (winner determined or time expired).
    Ended,
}

/// Outcome of a finished battleground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgOutcome {
    /// One team won by reaching win_score or depleting enemy reinforcements.
    Winner(BgTeam),
    /// Time expired with no winner — draw.
    Draw,
}

/// Per-team score in a battleground instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TeamScore {
    pub score: u32,
    pub reinforcements: u32,
}

/// A live battleground instance.
#[derive(Debug, Clone, PartialEq)]
pub struct BgInstance {
    pub bg_id: u32,
    pub phase: BgPhase,
    pub team_a_players: Vec<u64>,
    pub team_b_players: Vec<u64>,
    pub score_a: TeamScore,
    pub score_b: TeamScore,
    pub elapsed_secs: u32,
    pub outcome: Option<BgOutcome>,
}

impl BgInstance {
    /// Create a new instance from a match result and template.
    pub fn from_match(bg_match: &BgMatch, tmpl: &BgTemplate) -> Self {
        Self {
            bg_id: bg_match.bg_id,
            phase: BgPhase::Waiting,
            team_a_players: bg_match.team_a.clone(),
            team_b_players: bg_match.team_b.clone(),
            score_a: TeamScore {
                score: 0,
                reinforcements: tmpl.starting_reinforcements,
            },
            score_b: TeamScore {
                score: 0,
                reinforcements: tmpl.starting_reinforcements,
            },
            elapsed_secs: 0,
            outcome: None,
        }
    }

    /// Transition from Waiting to InProgress. Returns false if already started/ended.
    pub fn start(&mut self) -> bool {
        if self.phase != BgPhase::Waiting {
            return false;
        }
        self.phase = BgPhase::InProgress;
        true
    }

    /// Add score points for a team (flag captures, resource ticks).
    /// Checks win condition against the template's `win_score`.
    pub fn add_score(&mut self, team: BgTeam, points: u32, tmpl: &BgTemplate) {
        if self.phase != BgPhase::InProgress {
            return;
        }
        let score = match team {
            BgTeam::A => &mut self.score_a,
            BgTeam::B => &mut self.score_b,
        };
        score.score += points;
        if tmpl.win_score > 0 && score.score >= tmpl.win_score {
            self.end(BgOutcome::Winner(team));
        }
    }

    /// Deduct reinforcements from a team. Triggers loss at zero.
    pub fn deduct_reinforcements(&mut self, team: BgTeam, amount: u32) {
        if self.phase != BgPhase::InProgress {
            return;
        }
        let score = match team {
            BgTeam::A => &mut self.score_a,
            BgTeam::B => &mut self.score_b,
        };
        score.reinforcements = score.reinforcements.saturating_sub(amount);
        if score.reinforcements == 0 {
            let winner = match team {
                BgTeam::A => BgTeam::B,
                BgTeam::B => BgTeam::A,
            };
            self.end(BgOutcome::Winner(winner));
        }
    }

    /// Advance elapsed time. Ends the match as a draw if max_duration exceeded.
    pub fn tick(&mut self, delta_secs: u32, tmpl: &BgTemplate) {
        if self.phase != BgPhase::InProgress {
            return;
        }
        self.elapsed_secs += delta_secs;
        if self.elapsed_secs >= tmpl.max_duration_secs {
            self.end(BgOutcome::Draw);
        }
    }

    fn end(&mut self, outcome: BgOutcome) {
        self.phase = BgPhase::Ended;
        self.outcome = Some(outcome);
    }
}

#[path = "battleground_objectives.rs"]
mod objectives;
pub use objectives::*;

// -- Scoring & Rewards --

/// Honor currency ID (matches `currency.rs` ALL_CURRENCIES).
pub const HONOR_CURRENCY_ID: u32 = 1;

/// Base honor awarded to the winning team.
const WINNER_HONOR: u32 = 150;
/// Base honor awarded to the losing team.
const LOSER_HONOR: u32 = 50;
/// Honor for a draw (both teams).
const DRAW_HONOR: u32 = 75;
/// Marks awarded to the winning team.
const WINNER_MARKS: u32 = 3;
/// Marks awarded to the losing team.
const LOSER_MARKS: u32 = 1;
/// Marks for a draw.
const DRAW_MARKS: u32 = 2;

/// Rewards for a single player after a battleground ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BgReward {
    pub honor: u32,
    pub marks: u32,
}

/// Per-player reward list for a completed battleground.
#[derive(Debug, Clone, PartialEq)]
pub struct BgRewardDistribution {
    pub rewards: Vec<(u64, BgReward)>,
}

/// Compute rewards for all players in a finished battleground.
///
/// Winners get more honor and marks than losers. Draws get an
/// intermediate amount. Returns `None` if the match hasn't ended.
pub fn distribute_rewards(instance: &BgInstance) -> Option<BgRewardDistribution> {
    let outcome = instance.outcome?;
    if instance.phase != BgPhase::Ended {
        return None;
    }

    let (reward_a, reward_b) = team_rewards(outcome);
    let mut rewards = team_reward_entries(&instance.team_a_players, reward_a);
    rewards.extend(team_reward_entries(&instance.team_b_players, reward_b));
    Some(BgRewardDistribution { rewards })
}

fn team_rewards(outcome: BgOutcome) -> (BgReward, BgReward) {
    match outcome {
        BgOutcome::Winner(BgTeam::A) => (winner_reward(), loser_reward()),
        BgOutcome::Winner(BgTeam::B) => (loser_reward(), winner_reward()),
        BgOutcome::Draw => (draw_reward(), draw_reward()),
    }
}

fn winner_reward() -> BgReward {
    BgReward {
        honor: WINNER_HONOR,
        marks: WINNER_MARKS,
    }
}

fn loser_reward() -> BgReward {
    BgReward {
        honor: LOSER_HONOR,
        marks: LOSER_MARKS,
    }
}

fn draw_reward() -> BgReward {
    BgReward {
        honor: DRAW_HONOR,
        marks: DRAW_MARKS,
    }
}

fn team_reward_entries(players: &[u64], reward: BgReward) -> Vec<(u64, BgReward)> {
    players
        .iter()
        .copied()
        .map(|player| (player, reward))
        .collect()
}

// -- Deserter Penalty --

/// BG deserter debuff duration in seconds (15 minutes).
pub const BG_DESERTER_DURATION: u64 = 900;

/// Tracks BG deserter penalties for players who leave matches early.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BgDeserterTracker {
    /// (player_entity, expires_at_timestamp) pairs.
    entries: Vec<(u64, u64)>,
}

impl BgDeserterTracker {
    /// Apply a deserter penalty to a player.
    pub fn apply(&mut self, player: u64, now: u64) {
        let expires = now + BG_DESERTER_DURATION;
        if let Some(entry) = self.entries.iter_mut().find(|(p, _)| *p == player) {
            entry.1 = expires;
        } else {
            self.entries.push((player, expires));
        }
    }

    /// Whether a player has an active deserter penalty.
    pub fn is_deserter(&self, player: u64, now: u64) -> bool {
        self.entries
            .iter()
            .any(|(p, exp)| *p == player && now < *exp)
    }

    /// Remaining deserter time in seconds (0 if not active).
    pub fn remaining(&self, player: u64, now: u64) -> u64 {
        self.entries
            .iter()
            .find(|(p, _)| *p == player)
            .map_or(0, |(_, exp)| exp.saturating_sub(now))
    }

    /// Remove expired penalties.
    pub fn cleanup(&mut self, now: u64) {
        self.entries.retain(|(_, exp)| now < *exp);
    }
}

/// Process a player leaving an in-progress BG. Applies deserter and
/// removes them from the instance. Returns false if player not found.
pub fn leave_bg(
    instance: &mut BgInstance,
    deserters: &mut BgDeserterTracker,
    player: u64,
    now: u64,
) -> bool {
    if instance.phase != BgPhase::InProgress {
        return false;
    }
    let in_a = instance.team_a_players.contains(&player);
    let in_b = instance.team_b_players.contains(&player);
    if !in_a && !in_b {
        return false;
    }
    if in_a {
        instance.team_a_players.retain(|&p| p != player);
    } else {
        instance.team_b_players.retain(|&p| p != player);
    }
    deserters.apply(player, now);
    true
}

#[cfg(test)]
#[path = "battleground_tests.rs"]
mod tests;
