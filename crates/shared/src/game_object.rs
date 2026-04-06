//! Game object system: interactive world objects.
//!
//! Chests, doors, mining nodes, quest objects, traps, and more.
//! Ref: AzerothCore `GameObject.cpp`, `GameObjectAI.cpp`.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Game object type — determines interaction behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameObjectType {
    /// Lootable container.
    Chest,
    /// Openable/closable door or gate.
    Door,
    /// Interactable quest objective.
    QuestObject,
    /// Mining ore node (requires Mining skill).
    MiningNode,
    /// Herb node (requires Herbalism skill).
    HerbNode,
    /// Proximity-triggered trap.
    Trap,
    /// Mailbox for sending/receiving mail.
    Mailbox,
    /// Forge for blacksmithing.
    Forge,
    /// Anvil for blacksmithing.
    Anvil,
}

/// Bitflags for game object properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GameObjectFlags(pub u32);

impl GameObjectFlags {
    pub const INTERACTABLE: Self = Self(0x01);
    pub const NO_DESPAWN: Self = Self(0x02);
    pub const LOCKED: Self = Self(0x04);
    pub const IN_USE: Self = Self(0x08);
    pub const TRIGGERED: Self = Self(0x10);

    pub fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn insert(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn remove(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
}

/// Static template for a game object type (loaded from world data).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameObjectTemplate {
    /// Unique template ID.
    pub id: u32,
    /// Object type determining interaction behavior.
    pub object_type: GameObjectType,
    /// Display model ID for the client.
    pub display_id: u32,
    /// Human-readable name.
    pub name: String,
    /// Faction ID (0 = neutral).
    pub faction: u32,
    /// Default flags for new instances.
    pub flags: GameObjectFlags,
}

/// Lifecycle state of a game object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameObjectState {
    /// Available for interaction.
    Ready,
    /// Currently being used by a player.
    InUse,
    /// Consumed/looted/gathered — waiting for respawn or removal.
    Depleted,
    /// Despawned, timer running until it reappears.
    Respawning,
}

/// ECS component for a spawned game object in the world.
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameObject {
    /// Template ID linking to `GameObjectTemplate`.
    pub template_id: u32,
    /// Object type (copied from template for fast access).
    pub object_type: GameObjectType,
    /// Display model ID.
    pub display_id: u32,
    /// Rotation quaternion (x, y, z, w).
    pub rotation: [f32; 4],
    /// Faction ID.
    pub faction: u32,
    /// Current flags.
    pub flags: GameObjectFlags,
    /// Lifecycle state.
    pub state: GameObjectState,
}

/// Why a state transition is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateError {
    /// Transition not allowed from the current state.
    InvalidTransition { from: GameObjectState, to: GameObjectState },
}

impl GameObject {
    /// Create from a template with a rotation.
    pub fn from_template(template: &GameObjectTemplate, rotation: [f32; 4]) -> Self {
        Self {
            template_id: template.id,
            object_type: template.object_type,
            display_id: template.display_id,
            rotation,
            faction: template.faction,
            flags: template.flags,
            state: GameObjectState::Ready,
        }
    }

    /// Whether the object can be interacted with.
    pub fn is_interactable(&self) -> bool {
        self.flags.contains(GameObjectFlags::INTERACTABLE)
            && self.state == GameObjectState::Ready
    }

    /// Whether the object is locked (requires a key).
    pub fn is_locked(&self) -> bool {
        self.flags.contains(GameObjectFlags::LOCKED)
    }

    /// Whether the object is currently in use by a player.
    pub fn is_in_use(&self) -> bool {
        self.state == GameObjectState::InUse
    }

    // --- State transitions ---

    /// Ready → InUse: a player starts interacting.
    pub fn begin_use(&mut self) -> Result<(), StateError> {
        self.transition(GameObjectState::InUse)
    }

    /// InUse → Depleted: interaction complete (looted, gathered).
    pub fn deplete(&mut self) -> Result<(), StateError> {
        if self.state != GameObjectState::InUse {
            return Err(StateError::InvalidTransition {
                from: self.state, to: GameObjectState::Depleted,
            });
        }
        self.state = GameObjectState::Depleted;
        Ok(())
    }

    /// Depleted → Respawning: server starts the respawn timer.
    pub fn begin_respawn(&mut self) -> Result<(), StateError> {
        if self.state != GameObjectState::Depleted {
            return Err(StateError::InvalidTransition {
                from: self.state, to: GameObjectState::Respawning,
            });
        }
        self.state = GameObjectState::Respawning;
        Ok(())
    }

    /// Respawning → Ready: respawn timer elapsed, object reappears.
    pub fn respawn(&mut self) -> Result<(), StateError> {
        if self.state != GameObjectState::Respawning {
            return Err(StateError::InvalidTransition {
                from: self.state, to: GameObjectState::Ready,
            });
        }
        self.state = GameObjectState::Ready;
        Ok(())
    }

    /// InUse → Ready: interaction cancelled (e.g. player walked away).
    pub fn cancel_use(&mut self) -> Result<(), StateError> {
        if self.state != GameObjectState::InUse {
            return Err(StateError::InvalidTransition {
                from: self.state, to: GameObjectState::Ready,
            });
        }
        self.state = GameObjectState::Ready;
        Ok(())
    }

    fn transition(&mut self, to: GameObjectState) -> Result<(), StateError> {
        if self.state != GameObjectState::Ready {
            return Err(StateError::InvalidTransition { from: self.state, to });
        }
        self.state = to;
        Ok(())
    }
}

/// Registry of all game object templates.
#[derive(Debug, Clone, PartialEq, Default, Resource)]
pub struct GameObjectRegistry {
    templates: Vec<GameObjectTemplate>,
}

impl GameObjectRegistry {
    pub fn new() -> Self {
        Self { templates: Vec::new() }
    }

    /// Register a template.
    pub fn add(&mut self, template: GameObjectTemplate) {
        self.templates.retain(|t| t.id != template.id);
        self.templates.push(template);
    }

    /// Look up a template by ID.
    pub fn get(&self, id: u32) -> Option<&GameObjectTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }

    /// All templates of a given type.
    pub fn by_type(&self, object_type: GameObjectType) -> Vec<&GameObjectTemplate> {
        self.templates.iter().filter(|t| t.object_type == object_type).collect()
    }

    /// Number of registered templates.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

// --- Interaction system ---

/// Why an interaction failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionError {
    /// Object is not interactable.
    NotInteractable,
    /// Object is currently in use by another player.
    InUse,
    /// Object is locked and the player has no key.
    Locked,
    /// Object is depleted (e.g. already looted, gathered).
    Depleted,
    /// Player doesn't have the required skill.
    MissingSkill,
    /// Player is too far from the object.
    OutOfRange,
}

/// The result of interacting with a game object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionResult {
    /// Open a loot window for this chest.
    LootChest { template_id: u32 },
    /// Toggle a door open or closed.
    ToggleDoor { template_id: u32, now_open: bool },
    /// Begin gathering from a node.
    GatherNode { template_id: u32, node_type: GameObjectType },
    /// Complete a quest objective.
    QuestObjective { template_id: u32 },
    /// Trigger a trap effect.
    TriggerTrap { template_id: u32 },
    /// Open the mailbox UI.
    OpenMailbox,
    /// Access a crafting station (forge, anvil).
    UseCraftingStation { template_id: u32, station_type: GameObjectType },
}

/// Validate and process a player interaction with a game object.
///
/// Checks state and flags, transitions Ready → InUse, then dispatches
/// to the type-specific handler.
pub fn interact(go: &mut GameObject) -> Result<InteractionResult, InteractionError> {
    validate_interaction(go)?;
    go.begin_use().map_err(|_| InteractionError::InUse)?;
    dispatch_interaction(go)
}

fn validate_interaction(go: &GameObject) -> Result<(), InteractionError> {
    if !go.flags.contains(GameObjectFlags::INTERACTABLE) {
        return Err(InteractionError::NotInteractable);
    }
    if go.state != GameObjectState::Ready {
        return match go.state {
            GameObjectState::InUse => Err(InteractionError::InUse),
            GameObjectState::Depleted | GameObjectState::Respawning => Err(InteractionError::Depleted),
            GameObjectState::Ready => unreachable!(),
        };
    }
    if go.is_locked() {
        return Err(InteractionError::Locked);
    }
    Ok(())
}

fn dispatch_interaction(go: &GameObject) -> Result<InteractionResult, InteractionError> {
    let id = go.template_id;
    match go.object_type {
        GameObjectType::Chest => Ok(InteractionResult::LootChest { template_id: id }),
        GameObjectType::Door => Ok(InteractionResult::ToggleDoor { template_id: id, now_open: true }),
        GameObjectType::MiningNode | GameObjectType::HerbNode => {
            Ok(InteractionResult::GatherNode { template_id: id, node_type: go.object_type })
        }
        GameObjectType::QuestObject => Ok(InteractionResult::QuestObjective { template_id: id }),
        GameObjectType::Trap => Ok(InteractionResult::TriggerTrap { template_id: id }),
        GameObjectType::Mailbox => Ok(InteractionResult::OpenMailbox),
        GameObjectType::Forge | GameObjectType::Anvil => {
            Ok(InteractionResult::UseCraftingStation { template_id: id, station_type: go.object_type })
        }
    }
}

/// Complete interaction: InUse → Depleted (for consumable objects like chests/nodes).
pub fn deplete_object(go: &mut GameObject) -> Result<(), StateError> {
    go.deplete()
}

/// Cancel interaction: InUse → Ready (player walked away).
pub fn cancel_interaction(go: &mut GameObject) -> Result<(), StateError> {
    go.cancel_use()
}

// --- Doors ---

/// Configuration for a door game object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorData {
    /// Item ID required to unlock (0 = no key required).
    pub key_item_id: u32,
    /// Lockpicking skill required (0 = cannot be picked).
    pub lockpick_skill: u16,
    /// Seconds before the door auto-closes (0 = stays open).
    pub auto_close_secs: u32,
}

/// Result of toggling a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorToggleResult {
    /// Whether the door is now open.
    pub is_open: bool,
    /// Seconds until auto-close (0 = won't auto-close).
    pub auto_close_secs: u32,
}

/// Toggle a door open or closed.
///
/// - If locked, requires the correct key item or sufficient lockpick skill.
/// - Opens: sets IN_USE flag (door is open). Closes: clears it.
/// - Doors don't use the Depleted/Respawning states — they toggle.
pub fn toggle_door(
    go: &mut GameObject,
    door: &DoorData,
    has_key: bool,
    lockpick_skill: u16,
) -> Result<DoorToggleResult, InteractionError> {
    if go.object_type != GameObjectType::Door {
        return Err(InteractionError::NotInteractable);
    }
    if !go.flags.contains(GameObjectFlags::INTERACTABLE) {
        return Err(InteractionError::NotInteractable);
    }

    let is_open = go.state == GameObjectState::InUse;

    if is_open {
        // Close the door
        go.state = GameObjectState::Ready;
        return Ok(DoorToggleResult { is_open: false, auto_close_secs: 0 });
    }

    // Opening — check lock
    if go.is_locked() {
        let can_unlock = has_key
            || (door.lockpick_skill > 0 && lockpick_skill >= door.lockpick_skill);
        if !can_unlock {
            return Err(InteractionError::Locked);
        }
        go.flags.remove(GameObjectFlags::LOCKED);
    }

    go.state = GameObjectState::InUse;
    Ok(DoorToggleResult { is_open: true, auto_close_secs: door.auto_close_secs })
}

/// Auto-close a door (called by server timer). Sets back to Ready.
pub fn close_door(go: &mut GameObject) {
    if go.object_type == GameObjectType::Door && go.state == GameObjectState::InUse {
        go.state = GameObjectState::Ready;
    }
}

// --- Traps ---

/// Configuration for a trap game object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrapData {
    /// Spell ID applied to entities within trigger radius.
    pub spell_id: u32,
    /// Trigger radius in yards.
    pub radius: f32,
    /// Cooldown in seconds before the trap can trigger again (0 = one-shot, depletes).
    pub cooldown: u32,
}

/// An entity affected by a trap trigger.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrapTarget {
    pub entity: u64,
    pub spell_id: u32,
}

/// Result of a trap triggering.
#[derive(Debug, Clone, PartialEq)]
pub struct TrapTriggerResult {
    pub targets: Vec<TrapTarget>,
    /// Whether the trap is now depleted (one-shot) or on cooldown.
    pub depleted: bool,
}

/// Check if a position is within a trap's trigger radius.
pub fn in_trap_radius(
    trap_x: f32, trap_y: f32, trap_z: f32,
    target_x: f32, target_y: f32, target_z: f32,
    radius: f32,
) -> bool {
    let dx = trap_x - target_x;
    let dy = trap_y - target_y;
    let dz = trap_z - target_z;
    dx * dx + dy * dy + dz * dz <= radius * radius
}

/// Trigger a trap, applying its spell to all nearby entities.
///
/// `nearby_entities`: (entity_id, x, y, z) of entities to check.
/// Only entities within the trap's radius are affected.
/// One-shot traps (cooldown=0) transition to Depleted.
/// Cooldown traps set the TRIGGERED flag.
pub fn trigger_trap(
    go: &mut GameObject,
    trap: &TrapData,
    trap_pos: (f32, f32, f32),
    nearby_entities: &[(u64, f32, f32, f32)],
) -> Result<TrapTriggerResult, InteractionError> {
    if go.object_type != GameObjectType::Trap {
        return Err(InteractionError::NotInteractable);
    }
    if go.state != GameObjectState::Ready {
        return Err(InteractionError::Depleted);
    }
    if go.flags.contains(GameObjectFlags::TRIGGERED) {
        return Err(InteractionError::Depleted);
    }

    let targets: Vec<TrapTarget> = nearby_entities
        .iter()
        .filter(|(_, x, y, z)| in_trap_radius(trap_pos.0, trap_pos.1, trap_pos.2, *x, *y, *z, trap.radius))
        .map(|(entity, _, _, _)| TrapTarget { entity: *entity, spell_id: trap.spell_id })
        .collect();

    if targets.is_empty() {
        return Ok(TrapTriggerResult { targets, depleted: false });
    }

    let one_shot = trap.cooldown == 0;
    if one_shot {
        go.state = GameObjectState::Depleted;
    } else {
        go.flags.insert(GameObjectFlags::TRIGGERED);
    }

    Ok(TrapTriggerResult { targets, depleted: one_shot })
}

/// Reset a trap after its cooldown expires (clear TRIGGERED flag).
pub fn reset_trap(go: &mut GameObject) {
    go.flags.remove(GameObjectFlags::TRIGGERED);
}

// --- Gathering nodes ---

/// An item yielded from gathering a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatherYield {
    pub item_id: u32,
    pub min_count: u16,
    pub max_count: u16,
}

/// Gathering configuration for a mining/herb node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatheringData {
    /// Minimum skill level required to gather.
    pub required_skill: u16,
    /// Skill level at which gathering is guaranteed (no fail).
    pub trivial_skill: u16,
    /// Items yielded on successful gather.
    pub yields: Vec<GatherYield>,
    /// Skill points gained on gather (0 if trivial).
    pub skill_gain: u16,
}

/// Result of a successful gather.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatherResult {
    pub items: Vec<(u32, u16)>,
    pub skill_gained: u16,
}

/// Attempt to gather from a node.
///
/// Checks skill requirement, rolls yield counts, transitions the
/// object to Depleted. `count_roll` is a 0.0–1.0 random value for
/// determining stack sizes within min/max range.
pub fn gather_node(
    go: &mut GameObject,
    data: &GatheringData,
    player_skill: u16,
    count_roll: f32,
) -> Result<GatherResult, InteractionError> {
    if !matches!(go.object_type, GameObjectType::MiningNode | GameObjectType::HerbNode) {
        return Err(InteractionError::NotInteractable);
    }
    if go.state != GameObjectState::InUse {
        return Err(InteractionError::InUse);
    }
    if player_skill < data.required_skill {
        go.cancel_use().ok();
        return Err(InteractionError::MissingSkill);
    }

    let items = roll_yields(&data.yields, count_roll);
    let skill_gained = if player_skill < data.trivial_skill { data.skill_gain } else { 0 };

    go.deplete().ok();
    Ok(GatherResult { items, skill_gained })
}

fn roll_yields(yields: &[GatherYield], count_roll: f32) -> Vec<(u32, u16)> {
    yields.iter().map(|y| {
        let range = y.max_count - y.min_count;
        let count = y.min_count + (range as f32 * count_roll) as u16;
        (y.item_id, count.min(y.max_count))
    }).collect()
}

// --- Spawn system ---

/// How a game object spawn was created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpawnOrigin {
    /// Static spawn loaded from world.db at server startup.
    Static,
    /// Dynamic spawn created by a script or event at runtime.
    Dynamic,
}

/// Definition of a game object spawn point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameObjectSpawn {
    /// Unique spawn ID.
    pub spawn_id: u64,
    /// Template ID to spawn.
    pub template_id: u32,
    /// Map ID where the object spawns.
    pub map_id: u16,
    /// World position.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Rotation quaternion.
    pub rotation: [f32; 4],
    /// How this spawn was created.
    pub origin: SpawnOrigin,
    /// Respawn time in seconds (0 = no respawn).
    pub respawn_time: u32,
}

/// Manages game object spawn points.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpawnManager {
    spawns: Vec<GameObjectSpawn>,
    next_spawn_id: u64,
}

impl SpawnManager {
    pub fn new() -> Self {
        Self { spawns: Vec::new(), next_spawn_id: 1 }
    }

    /// Load a static spawn from world data.
    pub fn add_static(&mut self, spawn: GameObjectSpawn) -> u64 {
        self.insert(spawn, SpawnOrigin::Static)
    }

    /// Create a dynamic spawn from a script or event.
    pub fn add_dynamic(&mut self, spawn: GameObjectSpawn) -> u64 {
        self.insert(spawn, SpawnOrigin::Dynamic)
    }

    fn insert(&mut self, mut spawn: GameObjectSpawn, origin: SpawnOrigin) -> u64 {
        let id = self.next_spawn_id;
        self.next_spawn_id += 1;
        spawn.spawn_id = id;
        spawn.origin = origin;
        self.spawns.push(spawn);
        id
    }

    /// Remove a spawn by ID. Only dynamic spawns can be removed.
    pub fn remove_dynamic(&mut self, spawn_id: u64) -> bool {
        let before = self.spawns.len();
        self.spawns.retain(|s| !(s.spawn_id == spawn_id && s.origin == SpawnOrigin::Dynamic));
        self.spawns.len() < before
    }

    /// Get a spawn by ID.
    pub fn get(&self, spawn_id: u64) -> Option<&GameObjectSpawn> {
        self.spawns.iter().find(|s| s.spawn_id == spawn_id)
    }

    /// All spawns for a given map.
    pub fn spawns_for_map(&self, map_id: u16) -> Vec<&GameObjectSpawn> {
        self.spawns.iter().filter(|s| s.map_id == map_id).collect()
    }

    /// All spawns of a given template.
    pub fn spawns_for_template(&self, template_id: u32) -> Vec<&GameObjectSpawn> {
        self.spawns.iter().filter(|s| s.template_id == template_id).collect()
    }

    /// Total number of spawns.
    pub fn len(&self) -> usize {
        self.spawns.len()
    }

    /// Whether there are no spawns.
    pub fn is_empty(&self) -> bool {
        self.spawns.is_empty()
    }

    /// Count of static vs dynamic spawns.
    pub fn counts(&self) -> (usize, usize) {
        let statics = self.spawns.iter().filter(|s| s.origin == SpawnOrigin::Static).count();
        (statics, self.spawns.len() - statics)
    }
}

// --- Respawn timer ---

use std::collections::HashMap;

/// Tracks despawned game objects awaiting respawn.
///
/// When an object is used/depleted, call `mark_despawned()`.
/// Each tick, call `collect_respawns()` to get spawn IDs ready to respawn.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RespawnTracker {
    /// spawn_id → timestamp when the object should respawn.
    pending: HashMap<u64, u64>,
}

impl RespawnTracker {
    pub fn new() -> Self {
        Self { pending: HashMap::new() }
    }

    /// Mark a spawn as despawned. It will respawn after `respawn_time` seconds.
    /// Does nothing if `respawn_time` is 0 (no respawn).
    pub fn mark_despawned(&mut self, spawn_id: u64, respawn_time: u32, now: u64) {
        if respawn_time == 0 {
            return;
        }
        let respawn_at = now + respawn_time as u64;
        self.pending.insert(spawn_id, respawn_at);
    }

    /// Collect all spawn IDs whose respawn timer has elapsed.
    /// Removes them from the tracker.
    pub fn collect_respawns(&mut self, now: u64) -> Vec<u64> {
        let ready: Vec<u64> = self.pending
            .iter()
            .filter(|(_, respawn_at)| now >= **respawn_at)
            .map(|(id, _)| *id)
            .collect();
        for &id in &ready {
            self.pending.remove(&id);
        }
        ready
    }

    /// Whether a spawn is currently despawned and waiting to respawn.
    pub fn is_despawned(&self, spawn_id: u64) -> bool {
        self.pending.contains_key(&spawn_id)
    }

    /// Time remaining until respawn (0 if not despawned).
    pub fn time_remaining(&self, spawn_id: u64, now: u64) -> u64 {
        self.pending
            .get(&spawn_id)
            .map(|&at| at.saturating_sub(now))
            .unwrap_or(0)
    }

    /// Number of objects awaiting respawn.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Cancel a pending respawn (e.g. spawn removed permanently).
    pub fn cancel(&mut self, spawn_id: u64) {
        self.pending.remove(&spawn_id);
    }
}

#[cfg(test)]
#[path = "game_object_tests.rs"]
mod tests;
