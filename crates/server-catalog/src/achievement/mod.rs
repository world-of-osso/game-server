//! Generated achievement lookups and their immutable data records.
pub mod conditions;
pub mod mythic_predicates;
pub mod node_counts;
pub mod overlays;
pub mod rank_factions;
pub mod single_modifiers;

#[derive(Debug, Clone, Copy)]
pub struct MythicRequirements {
    pub min_level: u32,
    pub challenge_map: Option<u32>,
    pub player_level: Option<u32>,
    pub in_time: bool,
}

/// Compiled TC DungeonDifficulty68/LegacyDungeonDifficulty20 tree: the player's current
/// map difficulty must (`allowed`) or must not be one of `ids`.
#[derive(Debug, Clone, Copy)]
pub struct DifficultyRule {
    pub allowed: bool,
    pub ids: &'static [u32],
}

/// Compiled All facts or a plain Some of complete target-creature/map/area/aura alternatives.
#[derive(Debug, Clone, Copy)]
pub enum CompiledConditions {
    All {
        difficulty: Option<DifficultyRule>,
        // Negative compiled kinds denote bounded SingleFalse leaves, never raw DB2 types.
        facts: &'static [(i32, u32)],
    },
    TargetCreatures(&'static [u32]),
    PlayerMaps(&'static [u32]),
    PlayerLocations(&'static [(i32, u32)]),
    PlayerAuras(&'static [u32]),
    EventItems(&'static [u32]),
}
