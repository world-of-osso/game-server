//! Instance template data and runtime instance management.
//!
//! Static configuration (templates) and runtime creation/tracking of
//! instanced maps for dungeons and raids.
//! Ref: AzerothCore `instance_template` table, `MapInstanced.cpp`, `InstanceSaveMgr.cpp`.

use bevy::prelude::*;
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

/// ECS component tagging an entity as belonging to a specific instance.
///
/// Entities in the open world have no `InstanceId`. Entities inside a dungeon
/// or raid (players, creatures, game objects) carry this component so the
/// server can isolate them per-group.
///
/// Two groups in the same dungeon map get different `InstanceId` values,
/// ensuring their creatures, loot, and combat are fully independent.
#[derive(
    Component, Reflect, Serialize, Deserialize, bitcode::Encode, bitcode::Decode,
    Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub struct InstanceId(pub u32);

/// Open-world sentinel: entities without an `InstanceId` are in the open world.
/// This constant is used in visibility checks when a player is not in any instance.
pub const OPEN_WORLD: Option<InstanceId> = None;

/// Check whether two entities share the same instance (or both are in open world).
///
/// Returns `true` if both are `None` (open world) or both are `Some` with the
/// same instance ID. Used for visibility filtering — entities only see others
/// in the same instance.
pub fn same_instance(a: Option<&InstanceId>, b: Option<&InstanceId>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.0 == b.0,
        _ => false,
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

/// Why an instance operation failed.
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
    /// No active instance with this ID.
    InstanceNotFound,
    /// Group already has an active instance for this map+difficulty.
    AlreadyExists,
    /// Instance has no remaining player slots.
    InstanceFull,
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

// --- Runtime instance management ---
// Ref: AzerothCore `MapInstanced::CreateInstanceForPlayer`

use std::collections::HashMap;

/// A live instance of a dungeon or raid map.
///
/// Each group gets an isolated copy with its own creatures and state.
/// Entities spawned for this instance carry `InstanceId(instance_id)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instance {
    /// Unique instance ID (sequential, server-wide).
    pub instance_id: u32,
    /// Map ID linking to the template.
    pub map_id: u16,
    /// Difficulty this instance was created at.
    pub difficulty: Difficulty,
    /// Group leader entity ID that owns this instance.
    pub group_leader: u64,
    /// Players currently inside the instance.
    pub players: Vec<u64>,
    /// Entity IDs of creatures/objects spawned for this instance.
    pub spawned_entities: Vec<u64>,
    /// Server timestamp when created.
    pub created_at: u64,
}

impl Instance {
    /// Whether a player is inside this instance.
    pub fn contains_player(&self, player: u64) -> bool {
        self.players.contains(&player)
    }

    /// Add a player to the instance. Returns error if full.
    pub fn add_player(&mut self, player: u64, max_players: u32) -> Result<(), InstanceError> {
        if self.players.len() as u32 >= max_players {
            return Err(InstanceError::InstanceFull);
        }
        if !self.contains_player(player) {
            self.players.push(player);
        }
        Ok(())
    }

    /// Remove a player from the instance. Returns true if they were present.
    pub fn remove_player(&mut self, player: u64) -> bool {
        let before = self.players.len();
        self.players.retain(|&p| p != player);
        self.players.len() < before
    }

    /// Whether the instance has no players inside.
    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    /// Record a spawned entity (creature, game object) in this instance.
    pub fn track_entity(&mut self, entity: u64) {
        if !self.spawned_entities.contains(&entity) {
            self.spawned_entities.push(entity);
        }
    }

    /// Remove a tracked entity (e.g. on despawn).
    pub fn untrack_entity(&mut self, entity: u64) {
        self.spawned_entities.retain(|&e| e != entity);
    }

    /// The `InstanceId` component value for entities in this instance.
    pub fn component_id(&self) -> InstanceId {
        InstanceId(self.instance_id)
    }
}

/// Manages all active instances. Handles creation, lookup, and cleanup.
///
/// Ref: AzerothCore `MapInstanced` + `InstanceSaveMgr`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InstanceManager {
    /// Active instances by instance ID.
    instances: HashMap<u32, Instance>,
    /// Maps (group_leader, map_id, difficulty) → instance_id for fast lookup.
    group_index: HashMap<(u64, u16, Difficulty), u32>,
    /// Next instance ID to assign.
    next_id: u32,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            group_index: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create a new instance for a group entering a dungeon/raid.
    ///
    /// Validates against the template registry, then spawns the instance.
    /// The group leader and members are added as initial players.
    pub fn create_instance(
        &mut self,
        registry: &InstanceTemplateRegistry,
        map_id: u16,
        difficulty: Difficulty,
        group_leader: u64,
        members: &[u64],
        now: u64,
    ) -> Result<u32, InstanceError> {
        registry.validate_entry(map_id, difficulty, members.len() as u32)?;

        let key = (group_leader, map_id, difficulty);
        if self.group_index.contains_key(&key) {
            return Err(InstanceError::AlreadyExists);
        }

        let instance_id = self.allocate_id();
        let instance = Instance {
            instance_id, map_id, difficulty, group_leader,
            players: members.to_vec(), spawned_entities: Vec::new(),
            created_at: now,
        };
        self.instances.insert(instance_id, instance);
        self.group_index.insert(key, instance_id);
        Ok(instance_id)
    }

    fn allocate_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Look up an active instance by ID.
    pub fn get(&self, instance_id: u32) -> Option<&Instance> {
        self.instances.get(&instance_id)
    }

    /// Look up a mutable reference to an active instance.
    pub fn get_mut(&mut self, instance_id: u32) -> Option<&mut Instance> {
        self.instances.get_mut(&instance_id)
    }

    /// Find the instance a group already has for a given map+difficulty.
    pub fn find_group_instance(
        &self,
        group_leader: u64,
        map_id: u16,
        difficulty: Difficulty,
    ) -> Option<u32> {
        self.group_index
            .get(&(group_leader, map_id, difficulty))
            .copied()
    }

    /// Get or create an instance for a group.
    ///
    /// If the group already has an instance for this map+difficulty, returns it.
    /// Otherwise creates a new one.
    pub fn get_or_create(
        &mut self,
        registry: &InstanceTemplateRegistry,
        map_id: u16,
        difficulty: Difficulty,
        group_leader: u64,
        members: &[u64],
        now: u64,
    ) -> Result<u32, InstanceError> {
        if let Some(id) = self.find_group_instance(group_leader, map_id, difficulty) {
            return Ok(id);
        }
        self.create_instance(registry, map_id, difficulty, group_leader, members, now)
    }

    /// Destroy an instance by ID. Returns the removed instance, if found.
    pub fn destroy(&mut self, instance_id: u32) -> Option<Instance> {
        let instance = self.instances.remove(&instance_id)?;
        let key = (instance.group_leader, instance.map_id, instance.difficulty);
        self.group_index.remove(&key);
        Some(instance)
    }

    /// Remove all empty instances (no players remaining).
    pub fn cleanup_empty(&mut self) -> Vec<Instance> {
        let empty_ids: Vec<u32> = self
            .instances
            .iter()
            .filter(|(_, inst)| inst.is_empty())
            .map(|(&id, _)| id)
            .collect();

        empty_ids.iter().filter_map(|&id| self.destroy(id)).collect()
    }

    /// Number of active instances.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether there are no active instances.
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Find which instance a player is currently in.
    pub fn find_player_instance(&self, player: u64) -> Option<u32> {
        self.instances
            .iter()
            .find(|(_, inst)| inst.contains_player(player))
            .map(|(&id, _)| id)
    }

    /// Iterate over all active instances.
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &Instance)> {
        self.instances.iter()
    }
}

// --- Instance save / lockout (per-character) ---
// Ref: AzerothCore `InstanceSaveMgr.cpp`, `InstancePlayerBind`

/// A character's saved binding to a specific instance.
///
/// Created when a boss is killed in a lockout-tracked difficulty.
/// Prevents re-entering a fresh instance until the lockout expires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceSave {
    /// Map ID of the instance.
    pub map_id: u16,
    /// Difficulty the lockout applies to.
    pub difficulty: Difficulty,
    /// The specific instance ID the character is bound to.
    pub instance_id: u32,
    /// Bitmask of completed boss encounters (bit N = encounter N).
    pub completed_encounters: u32,
    /// Timestamp when the lockout was first acquired.
    pub acquired_at: u64,
    /// Reset timer inherited from the difficulty config.
    pub reset_timer: ResetTimer,
}

impl InstanceSave {
    /// Timestamp when this lockout expires.
    pub fn expires_at(&self) -> Option<u64> {
        self.reset_timer.next_reset(self.acquired_at)
    }

    /// Whether this lockout has expired.
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at().is_some_and(|t| now >= t)
    }

    /// Record a boss kill (encounter index 0-31).
    pub fn complete_encounter(&mut self, encounter_index: u8) {
        self.completed_encounters |= 1 << encounter_index;
    }

    /// Whether a specific encounter has been completed.
    pub fn is_encounter_done(&self, encounter_index: u8) -> bool {
        self.completed_encounters & (1 << encounter_index) != 0
    }

    /// Number of completed encounters.
    pub fn completed_count(&self) -> u32 {
        self.completed_encounters.count_ones()
    }
}

/// Per-character instance lockout tracker.
///
/// Each character has their own lockouts independent of group.
/// A character locked to heroic Shadowfang Keep cannot enter a fresh
/// heroic Shadowfang Keep until the daily reset, but can enter normal.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CharacterLockouts {
    saves: Vec<InstanceSave>,
}

impl CharacterLockouts {
    /// Bind a character to an instance after a boss kill.
    ///
    /// If already bound to the same map+difficulty, updates the encounter mask.
    /// Otherwise creates a new save.
    pub fn bind(
        &mut self,
        map_id: u16,
        difficulty: Difficulty,
        instance_id: u32,
        encounter_index: u8,
        reset_timer: ResetTimer,
        now: u64,
    ) {
        if let Some(save) = self.find_mut(map_id, difficulty) {
            save.complete_encounter(encounter_index);
            return;
        }
        let mut save = InstanceSave {
            map_id,
            difficulty,
            instance_id,
            completed_encounters: 0,
            acquired_at: now,
            reset_timer,
        };
        save.complete_encounter(encounter_index);
        self.saves.push(save);
    }

    /// Check if a character is locked out of a map at a given difficulty.
    pub fn is_locked(&self, map_id: u16, difficulty: Difficulty, now: u64) -> bool {
        self.saves
            .iter()
            .any(|s| s.map_id == map_id && s.difficulty == difficulty && !s.is_expired(now))
    }

    /// Get the active save for a map+difficulty, if any.
    pub fn find(&self, map_id: u16, difficulty: Difficulty) -> Option<&InstanceSave> {
        self.saves
            .iter()
            .find(|s| s.map_id == map_id && s.difficulty == difficulty)
    }

    fn find_mut(&mut self, map_id: u16, difficulty: Difficulty) -> Option<&mut InstanceSave> {
        self.saves
            .iter_mut()
            .find(|s| s.map_id == map_id && s.difficulty == difficulty)
    }

    /// Get the instance ID a character is bound to for a map+difficulty.
    ///
    /// Used to rejoin the same instance after disconnect/relog.
    pub fn bound_instance(&self, map_id: u16, difficulty: Difficulty, now: u64) -> Option<u32> {
        self.find(map_id, difficulty)
            .filter(|s| !s.is_expired(now))
            .map(|s| s.instance_id)
    }

    /// Remove all expired lockouts.
    pub fn cleanup(&mut self, now: u64) {
        self.saves.retain(|s| !s.is_expired(now));
    }

    /// All active (non-expired) lockouts.
    pub fn active_lockouts(&self, now: u64) -> Vec<&InstanceSave> {
        self.saves.iter().filter(|s| !s.is_expired(now)).collect()
    }

    /// Next reset time across all active lockouts, or `None` if no lockouts.
    pub fn next_reset(&self, now: u64) -> Option<u64> {
        self.saves
            .iter()
            .filter(|s| !s.is_expired(now))
            .filter_map(|s| s.expires_at())
            .min()
    }

    /// Total number of saves (including expired).
    pub fn len(&self) -> usize {
        self.saves.len()
    }

    /// Whether the character has no saves.
    pub fn is_empty(&self) -> bool {
        self.saves.is_empty()
    }
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;
