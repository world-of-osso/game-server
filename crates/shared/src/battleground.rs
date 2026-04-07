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
        let Some(tmpl) = template_by_id(bg_id) else {
            return Err(BgQueueError::UnknownBg);
        };
        if !levels.iter().all(|&lvl| tmpl.bracket.contains(lvl)) {
            return Err(BgQueueError::OutOfBracket);
        }
        if self.is_queued(leader) {
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
        self.entries
            .iter()
            .any(|e| e.members.contains(&player))
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
        Some(BgMatch { bg_id, team_a, team_b })
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
            score_a: TeamScore { score: 0, reinforcements: tmpl.starting_reinforcements },
            score_b: TeamScore { score: 0, reinforcements: tmpl.starting_reinforcements },
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

// -- Objective: Flag Capture (WSG) --

/// State of a single flag in capture-the-flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagState {
    /// Flag is at its home base.
    AtBase,
    /// Flag is being carried by a player.
    Carried { carrier: u64 },
    /// Flag was dropped and is on the ground (returns to base after timeout).
    Dropped,
}

/// Flag capture state for both teams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagObjective {
    /// Team A's flag (defended by A, captured by B).
    pub flag_a: FlagState,
    /// Team B's flag (defended by B, captured by A).
    pub flag_b: FlagState,
}

impl Default for FlagObjective {
    fn default() -> Self {
        Self {
            flag_a: FlagState::AtBase,
            flag_b: FlagState::AtBase,
        }
    }
}

impl FlagObjective {
    /// A player picks up the enemy flag. Returns false if the flag isn't available.
    pub fn pickup(&mut self, team: BgTeam, carrier: u64) -> bool {
        let flag = self.enemy_flag_mut(team);
        let available = matches!(flag, FlagState::AtBase | FlagState::Dropped);
        if available {
            *flag = FlagState::Carried { carrier };
        }
        available
    }

    /// The flag carrier drops the flag (death or disconnect).
    pub fn drop_flag(&mut self, team: BgTeam) {
        let flag = self.enemy_flag_mut(team);
        if matches!(flag, FlagState::Carried { .. }) {
            *flag = FlagState::Dropped;
        }
    }

    /// A defender returns their team's dropped flag to base.
    pub fn return_flag(&mut self, team: BgTeam) -> bool {
        let flag = self.own_flag_mut(team);
        if *flag == FlagState::Dropped {
            *flag = FlagState::AtBase;
            return true;
        }
        false
    }

    /// Carrier reaches their own base with enemy flag. Awards a capture if
    /// the carrier's own flag is at base. Returns true if captured.
    pub fn capture(&mut self, team: BgTeam, instance: &mut BgInstance, tmpl: &BgTemplate) -> bool {
        let own_flag_at_base = *self.own_flag(team) == FlagState::AtBase;
        let enemy_flag = self.enemy_flag_mut(team);
        let is_carried = matches!(enemy_flag, FlagState::Carried { .. });
        if !is_carried || !own_flag_at_base {
            return false;
        }
        *enemy_flag = FlagState::AtBase;
        instance.add_score(team, 1, tmpl);
        true
    }

    fn enemy_flag_mut(&mut self, team: BgTeam) -> &mut FlagState {
        match team {
            BgTeam::A => &mut self.flag_b,
            BgTeam::B => &mut self.flag_a,
        }
    }

    fn own_flag_mut(&mut self, team: BgTeam) -> &mut FlagState {
        match team {
            BgTeam::A => &mut self.flag_a,
            BgTeam::B => &mut self.flag_b,
        }
    }

    fn own_flag(&self, team: BgTeam) -> &FlagState {
        match team {
            BgTeam::A => &self.flag_a,
            BgTeam::B => &self.flag_b,
        }
    }
}

// -- Objective: Node Control (AB) --

/// Who controls a capture node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeOwner {
    Neutral,
    Contested(BgTeam),
    Owned(BgTeam),
}

/// A capturable territory node.
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureNode {
    pub name: String,
    pub owner: NodeOwner,
}

/// Node control objective state.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeObjective {
    pub nodes: Vec<CaptureNode>,
    /// Resource points awarded per owned node per tick.
    pub points_per_node_per_tick: u32,
}

/// Points per tick based on number of owned nodes (AB-style scaling).
/// 1 node=10, 2=10, 3=10, 4=30, 5=30 per tick.
const AB_POINTS_PER_TICK: [u32; 6] = [0, 10, 10, 10, 30, 30];

impl NodeObjective {
    /// Create AB-style 5-node objective.
    pub fn arathi_basin() -> Self {
        let names = ["Stables", "Blacksmith", "Lumber Mill", "Gold Mine", "Farm"];
        Self {
            nodes: names
                .iter()
                .map(|&n| CaptureNode {
                    name: n.into(),
                    owner: NodeOwner::Neutral,
                })
                .collect(),
            points_per_node_per_tick: 10,
        }
    }

    /// Begin capturing a node. Transitions Neutral/enemy Owned → Contested.
    pub fn start_capture(&mut self, node_idx: usize, team: BgTeam) -> bool {
        let Some(node) = self.nodes.get_mut(node_idx) else {
            return false;
        };
        let can_contest = match node.owner {
            NodeOwner::Neutral => true,
            NodeOwner::Owned(owner) => owner != team,
            NodeOwner::Contested(_) => false,
        };
        if can_contest {
            node.owner = NodeOwner::Contested(team);
        }
        can_contest
    }

    /// Complete capture of a contested node. Transitions Contested → Owned.
    pub fn finish_capture(&mut self, node_idx: usize) -> bool {
        let Some(node) = self.nodes.get_mut(node_idx) else {
            return false;
        };
        if let NodeOwner::Contested(team) = node.owner {
            node.owner = NodeOwner::Owned(team);
            true
        } else {
            false
        }
    }

    /// Count nodes owned by a team.
    pub fn owned_count(&self, team: BgTeam) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.owner == NodeOwner::Owned(team))
            .count()
    }

    /// Tick resource points based on owned nodes and apply to instance score.
    pub fn tick_resources(&self, instance: &mut BgInstance, tmpl: &BgTemplate) {
        for team in [BgTeam::A, BgTeam::B] {
            let count = self.owned_count(team);
            let points = AB_POINTS_PER_TICK.get(count).copied().unwrap_or(30);
            if points > 0 {
                instance.add_score(team, points, tmpl);
            }
        }
    }
}

// -- Objective: Reinforcements (AV) --

/// Reinforcement depletion config for kill events.
pub const REINFORCEMENT_LOSS_PER_KILL: u32 = 1;

/// Process a player kill in a reinforcement BG.
/// Deducts from the victim's team.
pub fn on_kill_reinforcement(instance: &mut BgInstance, victim_team: BgTeam) {
    instance.deduct_reinforcements(victim_team, REINFORCEMENT_LOSS_PER_KILL);
}

#[cfg(test)]
#[path = "battleground_tests.rs"]
mod tests;
