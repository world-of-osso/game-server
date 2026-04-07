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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsg_template() {
        let wsg = warsong_gulch();
        assert_eq!(wsg.id, 1);
        assert_eq!(wsg.map_id, 489);
        assert_eq!(wsg.team_size, 10);
        assert_eq!(wsg.objective, BgObjective::FlagCapture);
        assert_eq!(wsg.win_score, 3);
        assert_eq!(wsg.max_duration_secs, 25 * 60);
    }

    #[test]
    fn ab_template() {
        let ab = arathi_basin();
        assert_eq!(ab.id, 2);
        assert_eq!(ab.team_size, 15);
        assert_eq!(ab.objective, BgObjective::NodeControl);
        assert_eq!(ab.win_score, 1600);
    }

    #[test]
    fn av_template() {
        let av = alterac_valley();
        assert_eq!(av.id, 3);
        assert_eq!(av.team_size, 40);
        assert_eq!(av.objective, BgObjective::Reinforcements);
        assert_eq!(av.starting_reinforcements, 600);
    }

    #[test]
    fn all_templates_returns_three() {
        let templates = all_templates();
        assert_eq!(templates.len(), 3);
    }

    #[test]
    fn template_ids_unique() {
        let templates = all_templates();
        let mut ids: Vec<u32> = templates.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), templates.len());
    }

    #[test]
    fn template_map_ids_unique() {
        let templates = all_templates();
        let mut maps: Vec<u32> = templates.iter().map(|t| t.map_id).collect();
        maps.sort();
        maps.dedup();
        assert_eq!(maps.len(), templates.len());
    }

    #[test]
    fn lookup_by_id() {
        assert_eq!(template_by_id(1).unwrap().name, "Warsong Gulch");
        assert_eq!(template_by_id(2).unwrap().name, "Arathi Basin");
        assert_eq!(template_by_id(3).unwrap().name, "Alterac Valley");
        assert!(template_by_id(999).is_none());
    }

    #[test]
    fn min_players_below_team_size() {
        for tmpl in all_templates() {
            assert!(
                tmpl.min_players_per_team <= tmpl.team_size,
                "{}: min {} > team_size {}",
                tmpl.name,
                tmpl.min_players_per_team,
                tmpl.team_size,
            );
        }
    }

    #[test]
    fn level_bracket_contains() {
        let bracket = LevelBracket::new(10, 60);
        assert!(!bracket.contains(9));
        assert!(bracket.contains(10));
        assert!(bracket.contains(35));
        assert!(bracket.contains(60));
        assert!(!bracket.contains(61));
    }

    #[test]
    fn all_templates_have_valid_brackets() {
        for tmpl in all_templates() {
            assert!(tmpl.bracket.min_level <= tmpl.bracket.max_level, "{}", tmpl.name);
            assert!(tmpl.bracket.min_level >= 1, "{}", tmpl.name);
        }
    }

    #[test]
    fn serialization_round_trip() {
        let wsg = warsong_gulch();
        let json = serde_json::to_string(&wsg).unwrap();
        let restored: BgTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(wsg, restored);
    }

    // -- Queue tests --

    #[test]
    fn join_solo_and_is_queued() {
        let mut q = BgQueue::default();
        assert!(q.join_solo(1, 30, 1, 100).is_ok());
        assert!(q.is_queued(1));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn join_solo_already_queued() {
        let mut q = BgQueue::default();
        q.join_solo(1, 30, 1, 100).unwrap();
        assert_eq!(q.join_solo(1, 30, 1, 101), Err(BgQueueError::AlreadyQueued));
    }

    #[test]
    fn join_unknown_bg() {
        let mut q = BgQueue::default();
        assert_eq!(q.join_solo(1, 30, 999, 100), Err(BgQueueError::UnknownBg));
    }

    #[test]
    fn join_out_of_bracket() {
        let mut q = BgQueue::default();
        // WSG bracket is 10-60
        assert_eq!(q.join_solo(1, 5, 1, 100), Err(BgQueueError::OutOfBracket));
        assert_eq!(q.join_solo(1, 61, 1, 100), Err(BgQueueError::OutOfBracket));
    }

    #[test]
    fn join_group() {
        let mut q = BgQueue::default();
        q.join_group(1, vec![1, 2, 3], vec![30, 30, 30], 1, 100).unwrap();
        assert!(q.is_queued(1));
        assert!(q.is_queued(2));
        assert!(q.is_queued(3));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn join_empty_group() {
        let mut q = BgQueue::default();
        assert_eq!(
            q.join_group(1, vec![], vec![], 1, 100),
            Err(BgQueueError::EmptyGroup)
        );
    }

    #[test]
    fn leave_queue() {
        let mut q = BgQueue::default();
        q.join_solo(1, 30, 1, 100).unwrap();
        assert!(q.leave(1));
        assert!(!q.is_queued(1));
        assert!(q.is_empty());
    }

    #[test]
    fn leave_not_queued() {
        let mut q = BgQueue::default();
        assert!(!q.leave(999));
    }

    #[test]
    fn match_not_enough_players() {
        let mut q = BgQueue::default();
        // WSG needs 5 per team min (10 total), only queue 8
        for i in 1..=8 {
            q.join_solo(i, 30, 1, 100).unwrap();
        }
        assert!(q.try_match(1).is_none());
        assert_eq!(q.len(), 8); // entries preserved
    }

    #[test]
    fn match_forms_two_teams() {
        let mut q = BgQueue::default();
        // WSG: 10v10, min 5 per team. Queue 10 solo players.
        for i in 1..=10 {
            q.join_solo(i, 30, 1, 100).unwrap();
        }
        let m = q.try_match(1).unwrap();
        assert_eq!(m.bg_id, 1);
        assert_eq!(m.team_a.len(), 5);
        assert_eq!(m.team_b.len(), 5);

        // All 10 players should be in one of the teams
        let mut all: Vec<u64> = m.team_a.iter().chain(&m.team_b).copied().collect();
        all.sort();
        assert_eq!(all, (1..=10).collect::<Vec<u64>>());

        // Queue should have 0 entries left
        assert!(q.is_empty());
    }

    #[test]
    fn match_with_groups() {
        let mut q = BgQueue::default();
        // Queue 2 groups of 3 + 4 solos = 10 players for WSG
        q.join_group(1, vec![1, 2, 3], vec![30, 30, 30], 1, 100).unwrap();
        q.join_group(4, vec![4, 5, 6], vec![30, 30, 30], 1, 100).unwrap();
        for i in 7..=10 {
            q.join_solo(i, 30, 1, 100).unwrap();
        }
        let m = q.try_match(1).unwrap();
        let total = m.team_a.len() + m.team_b.len();
        assert_eq!(total, 10);
    }

    #[test]
    fn match_different_bg_types_independent() {
        let mut q = BgQueue::default();
        // 10 for WSG, 8 for AB (not enough for AB which needs 16 min)
        for i in 1..=10 {
            q.join_solo(i, 30, 1, 100).unwrap();
        }
        for i in 11..=18 {
            q.join_solo(i, 30, 2, 100).unwrap();
        }
        assert!(q.try_match(1).is_some()); // WSG ok
        assert!(q.try_match(2).is_none()); // AB needs 16
    }

    #[test]
    fn match_unknown_bg_returns_none() {
        let mut q = BgQueue::default();
        assert!(q.try_match(999).is_none());
    }
}
