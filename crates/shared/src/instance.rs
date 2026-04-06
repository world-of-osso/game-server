//! Instance template data for dungeons and raids.
//!
//! Defines the static configuration for instanced maps: player limits,
//! reset timers, and difficulty modes.
//! Ref: AzerothCore `instance_template` table, `InstanceSaveMgr.cpp`.

use serde::{Deserialize, Serialize};

/// Instance difficulty mode.
///
/// Affects creature stats, loot tables, and lockout periods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    /// Standard difficulty (5-player dungeons, 10/25 raids).
    Normal,
    /// Harder tuning, daily lockout for dungeons, weekly for raids.
    Heroic,
    /// Hardest tuning, weekly lockout. Dungeons and raids.
    Mythic,
    /// Scaling keystone difficulty (dungeons only).
    MythicPlus,
}

/// Type of instanced content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InstanceType {
    /// 5-player dungeon.
    Dungeon,
    /// Multi-group raid (10, 25, or flex).
    Raid,
    /// PvP battleground.
    Battleground,
    /// PvP arena.
    Arena,
    /// Scenario (1-3 players).
    Scenario,
}

/// Reset period for instance lockouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResetTimer {
    /// No lockout (normal dungeons, scenarios).
    None,
    /// Resets daily at server reset time.
    Daily,
    /// Resets weekly on Tuesday (US) / Wednesday (EU).
    Weekly,
}

/// Seconds per day.
const DAY_SECS: u64 = 86_400;
/// Seconds per week.
const WEEK_SECS: u64 = 604_800;

impl ResetTimer {
    /// Duration in seconds, or 0 for no lockout.
    pub fn duration_secs(self) -> u64 {
        match self {
            Self::None => 0,
            Self::Daily => DAY_SECS,
            Self::Weekly => WEEK_SECS,
        }
    }

    /// Next reset timestamp after `acquired_at`, or `None` for no lockout.
    pub fn next_reset(self, acquired_at: u64) -> Option<u64> {
        let period = self.duration_secs();
        if period == 0 {
            return None;
        }
        let periods = acquired_at / period;
        Some((periods + 1) * period)
    }
}

/// Difficulty-specific configuration within an instance template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyConfig {
    pub difficulty: Difficulty,
    /// Maximum players allowed for this difficulty.
    pub max_players: u32,
    /// Lockout reset period.
    pub reset_timer: ResetTimer,
    /// Creature stat multiplier (1.0 = normal baseline).
    pub creature_health_multiplier: f32,
    /// Creature damage multiplier (1.0 = normal baseline).
    pub creature_damage_multiplier: f32,
    /// Loot table suffix/variant ID for this difficulty.
    pub loot_table_variant: u32,
}

/// Static template for an instanced map.
///
/// Loaded at server startup. One per dungeon/raid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceTemplate {
    /// Map ID from world data.
    pub map_id: u16,
    /// Human-readable name.
    pub name: String,
    /// What kind of instance this is.
    pub instance_type: InstanceType,
    /// Parent (overworld) map ID for entrance location.
    pub parent_map_id: u16,
    /// Whether mounts are allowed inside.
    pub allow_mount: bool,
    /// Available difficulty configurations.
    pub difficulties: Vec<DifficultyConfig>,
}

impl InstanceTemplate {
    /// Get configuration for a specific difficulty, if supported.
    pub fn difficulty_config(&self, difficulty: Difficulty) -> Option<&DifficultyConfig> {
        self.difficulties.iter().find(|d| d.difficulty == difficulty)
    }

    /// Whether this template supports the given difficulty.
    pub fn supports_difficulty(&self, difficulty: Difficulty) -> bool {
        self.difficulties.iter().any(|d| d.difficulty == difficulty)
    }

    /// Maximum players for a given difficulty, or `None` if unsupported.
    pub fn max_players(&self, difficulty: Difficulty) -> Option<u32> {
        self.difficulty_config(difficulty).map(|d| d.max_players)
    }

    /// Reset timer for a given difficulty, or `None` if unsupported.
    pub fn reset_timer(&self, difficulty: Difficulty) -> Option<ResetTimer> {
        self.difficulty_config(difficulty).map(|d| d.reset_timer)
    }

    /// All supported difficulties for this instance.
    pub fn supported_difficulties(&self) -> Vec<Difficulty> {
        self.difficulties.iter().map(|d| d.difficulty).collect()
    }
}

/// Why an instance template lookup or validation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceError {
    /// No template found for this map ID.
    UnknownMap,
    /// The requested difficulty is not available for this instance.
    UnsupportedDifficulty,
    /// Player count exceeds the difficulty's max.
    TooManyPlayers,
    /// No difficulties configured.
    NoDifficulties,
}

/// Registry of all instance templates, indexed by map ID.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InstanceTemplateRegistry {
    templates: Vec<InstanceTemplate>,
}

impl InstanceTemplateRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    /// Register a template. Returns error if it has no difficulties.
    pub fn add(&mut self, template: InstanceTemplate) -> Result<(), InstanceError> {
        if template.difficulties.is_empty() {
            return Err(InstanceError::NoDifficulties);
        }
        // Replace existing template for same map_id
        self.templates.retain(|t| t.map_id != template.map_id);
        self.templates.push(template);
        Ok(())
    }

    /// Look up a template by map ID.
    pub fn get(&self, map_id: u16) -> Option<&InstanceTemplate> {
        self.templates.iter().find(|t| t.map_id == map_id)
    }

    /// Validate that a group can enter an instance at a given difficulty.
    pub fn validate_entry(
        &self,
        map_id: u16,
        difficulty: Difficulty,
        player_count: u32,
    ) -> Result<&InstanceTemplate, InstanceError> {
        let template = self.get(map_id).ok_or(InstanceError::UnknownMap)?;
        let config = template
            .difficulty_config(difficulty)
            .ok_or(InstanceError::UnsupportedDifficulty)?;
        if player_count > config.max_players {
            return Err(InstanceError::TooManyPlayers);
        }
        Ok(template)
    }

    /// Number of registered templates.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Iterate over all templates.
    pub fn iter(&self) -> impl Iterator<Item = &InstanceTemplate> {
        self.templates.iter()
    }

    /// All templates of a given type (e.g. all raids).
    pub fn by_type(&self, instance_type: InstanceType) -> Vec<&InstanceTemplate> {
        self.templates
            .iter()
            .filter(|t| t.instance_type == instance_type)
            .collect()
    }
}

/// Helper to build a standard 5-player dungeon template.
pub fn dungeon_template(map_id: u16, name: &str, parent_map_id: u16) -> InstanceTemplate {
    InstanceTemplate {
        map_id,
        name: name.to_string(),
        instance_type: InstanceType::Dungeon,
        parent_map_id,
        allow_mount: false,
        difficulties: vec![
            DifficultyConfig {
                difficulty: Difficulty::Normal,
                max_players: 5,
                reset_timer: ResetTimer::None,
                creature_health_multiplier: 1.0,
                creature_damage_multiplier: 1.0,
                loot_table_variant: 0,
            },
            DifficultyConfig {
                difficulty: Difficulty::Heroic,
                max_players: 5,
                reset_timer: ResetTimer::Daily,
                creature_health_multiplier: 2.0,
                creature_damage_multiplier: 1.5,
                loot_table_variant: 1,
            },
            DifficultyConfig {
                difficulty: Difficulty::Mythic,
                max_players: 5,
                reset_timer: ResetTimer::Weekly,
                creature_health_multiplier: 3.0,
                creature_damage_multiplier: 2.0,
                loot_table_variant: 2,
            },
        ],
    }
}

/// Helper to build a standard raid template.
pub fn raid_template(map_id: u16, name: &str, parent_map_id: u16) -> InstanceTemplate {
    InstanceTemplate {
        map_id,
        name: name.to_string(),
        instance_type: InstanceType::Raid,
        parent_map_id,
        allow_mount: false,
        difficulties: vec![
            DifficultyConfig {
                difficulty: Difficulty::Normal,
                max_players: 25,
                reset_timer: ResetTimer::Weekly,
                creature_health_multiplier: 1.0,
                creature_damage_multiplier: 1.0,
                loot_table_variant: 0,
            },
            DifficultyConfig {
                difficulty: Difficulty::Heroic,
                max_players: 25,
                reset_timer: ResetTimer::Weekly,
                creature_health_multiplier: 1.5,
                creature_damage_multiplier: 1.3,
                loot_table_variant: 1,
            },
            DifficultyConfig {
                difficulty: Difficulty::Mythic,
                max_players: 20,
                reset_timer: ResetTimer::Weekly,
                creature_health_multiplier: 2.5,
                creature_damage_multiplier: 2.0,
                loot_table_variant: 2,
            },
        ],
    }
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;
