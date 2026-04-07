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
}
