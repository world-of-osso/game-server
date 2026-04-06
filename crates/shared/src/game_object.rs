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
        }
    }

    /// Whether the object can be interacted with.
    pub fn is_interactable(&self) -> bool {
        self.flags.contains(GameObjectFlags::INTERACTABLE)
    }

    /// Whether the object is locked (requires a key).
    pub fn is_locked(&self) -> bool {
        self.flags.contains(GameObjectFlags::LOCKED)
    }

    /// Whether the object is currently in use by a player.
    pub fn is_in_use(&self) -> bool {
        self.flags.contains(GameObjectFlags::IN_USE)
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

#[cfg(test)]
#[path = "game_object_tests.rs"]
mod tests;
