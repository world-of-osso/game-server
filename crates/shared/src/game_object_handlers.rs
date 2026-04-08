//! Type-specific interaction handlers for game objects.
//!
//! Doors, traps, quest objects, and gathering nodes.

use serde::{Deserialize, Serialize};

use crate::game_object::{
    GameObject, GameObjectFlags, GameObjectState, GameObjectType, InteractionError, StateError,
};

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
    pub is_open: bool,
    pub auto_close_secs: u32,
}

/// Toggle a door open or closed.
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

    if go.state == GameObjectState::InUse {
        go.state = GameObjectState::Ready;
        return Ok(DoorToggleResult {
            is_open: false,
            auto_close_secs: 0,
        });
    }

    if go.is_locked() {
        let can_unlock =
            has_key || (door.lockpick_skill > 0 && lockpick_skill >= door.lockpick_skill);
        if !can_unlock {
            return Err(InteractionError::Locked);
        }
        go.flags.remove(GameObjectFlags::LOCKED);
    }

    go.state = GameObjectState::InUse;
    Ok(DoorToggleResult {
        is_open: true,
        auto_close_secs: door.auto_close_secs,
    })
}

/// Auto-close a door (called by server timer).
pub fn close_door(go: &mut GameObject) {
    if go.object_type == GameObjectType::Door && go.state == GameObjectState::InUse {
        go.state = GameObjectState::Ready;
    }
}

// --- Traps ---

/// Configuration for a trap game object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrapData {
    pub spell_id: u32,
    pub radius: f32,
    /// Cooldown seconds (0 = one-shot, depletes).
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
    pub depleted: bool,
}

/// Check if a position is within a trap's trigger radius.
pub fn in_trap_radius(
    trap_x: f32,
    trap_y: f32,
    trap_z: f32,
    target_x: f32,
    target_y: f32,
    target_z: f32,
    radius: f32,
) -> bool {
    let dx = trap_x - target_x;
    let dy = trap_y - target_y;
    let dz = trap_z - target_z;
    dx * dx + dy * dy + dz * dz <= radius * radius
}

/// Trigger a trap, applying its spell to nearby entities.
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
        .filter(|(_, x, y, z)| {
            in_trap_radius(trap_pos.0, trap_pos.1, trap_pos.2, *x, *y, *z, trap.radius)
        })
        .map(|(entity, _, _, _)| TrapTarget {
            entity: *entity,
            spell_id: trap.spell_id,
        })
        .collect();

    if targets.is_empty() {
        return Ok(TrapTriggerResult {
            targets,
            depleted: false,
        });
    }

    let one_shot = trap.cooldown == 0;
    if one_shot {
        go.state = GameObjectState::Depleted;
    } else {
        go.flags.insert(GameObjectFlags::TRIGGERED);
    }

    Ok(TrapTriggerResult {
        targets,
        depleted: one_shot,
    })
}

/// Reset a trap after its cooldown expires.
pub fn reset_trap(go: &mut GameObject) {
    go.flags.remove(GameObjectFlags::TRIGGERED);
}

// --- Quest objects ---

/// Configuration for a quest-interactive game object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestObjectData {
    pub quest_id: u32,
    pub objective_index: u8,
}

/// Result of interacting with a quest object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestObjectResult {
    pub quest_id: u32,
    pub objective_index: u8,
}

/// Interact with a quest object to advance an objective.
pub fn use_quest_object(
    go: &mut GameObject,
    data: &QuestObjectData,
    player_has_quest: bool,
) -> Result<QuestObjectResult, InteractionError> {
    if go.object_type != GameObjectType::QuestObject {
        return Err(InteractionError::NotInteractable);
    }
    if go.state != GameObjectState::InUse {
        return Err(InteractionError::InUse);
    }
    if !player_has_quest {
        go.cancel_use().ok();
        return Err(InteractionError::NotInteractable);
    }
    go.deplete().ok();
    Ok(QuestObjectResult {
        quest_id: data.quest_id,
        objective_index: data.objective_index,
    })
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
    pub required_skill: u16,
    pub trivial_skill: u16,
    pub yields: Vec<GatherYield>,
    pub skill_gain: u16,
}

/// Result of a successful gather.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatherResult {
    pub items: Vec<(u32, u16)>,
    pub skill_gained: u16,
}

/// Attempt to gather from a node.
pub fn gather_node(
    go: &mut GameObject,
    data: &GatheringData,
    player_skill: u16,
    count_roll: f32,
) -> Result<GatherResult, InteractionError> {
    if !matches!(
        go.object_type,
        GameObjectType::MiningNode | GameObjectType::HerbNode
    ) {
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
    let skill_gained = if player_skill < data.trivial_skill {
        data.skill_gain
    } else {
        0
    };

    go.deplete().ok();
    Ok(GatherResult {
        items,
        skill_gained,
    })
}

fn roll_yields(yields: &[GatherYield], count_roll: f32) -> Vec<(u32, u16)> {
    yields
        .iter()
        .map(|y| {
            let range = y.max_count - y.min_count;
            let count = y.min_count + (range as f32 * count_roll) as u16;
            (y.item_id, count.min(y.max_count))
        })
        .collect()
}

/// Complete interaction: InUse → Depleted.
pub fn deplete_object(go: &mut GameObject) -> Result<(), StateError> {
    go.deplete()
}

/// Cancel interaction: InUse → Ready.
pub fn cancel_interaction(go: &mut GameObject) -> Result<(), StateError> {
    go.cancel_use()
}
