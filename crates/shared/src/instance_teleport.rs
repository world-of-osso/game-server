//! Instance teleportation: entrance portals, exit portals, and hearthstone out.
//!
//! Generates teleport orders for players entering/exiting instances.
//! Ref: AzerothCore `MapMgr::PlayerCannotEnter`, `Player::TeleportTo`.

use serde::{Deserialize, Serialize};

use crate::instance::{
    Difficulty, InstanceError, InstanceManager, InstanceTemplateRegistry, WorldPosition,
};

/// Why a teleport was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeleportReason {
    /// Player entering via dungeon entrance portal.
    EntrancePortal,
    /// Player leaving via exit portal inside the instance.
    ExitPortal,
    /// Player used hearthstone while inside an instance.
    Hearthstone,
}

/// A teleport order the server should execute.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceTeleport {
    pub player: u64,
    pub destination: WorldPosition,
    pub instance_id: Option<u32>,
    pub reason: TeleportReason,
}

/// Generate teleport orders for a group entering an instance.
///
/// Each player gets teleported to the instance's entrance position.
/// The `instance_id` is set so the server tags them with `InstanceId`.
pub fn enter_instance(
    instance_id: u32,
    manager: &InstanceManager,
    registry: &InstanceTemplateRegistry,
    players: &[u64],
) -> Result<Vec<InstanceTeleport>, InstanceError> {
    let inst = manager
        .get(instance_id)
        .ok_or(InstanceError::InstanceNotFound)?;
    let tmpl = registry.get(inst.map_id).ok_or(InstanceError::UnknownMap)?;

    let orders = players
        .iter()
        .map(|&player| InstanceTeleport {
            player,
            destination: tmpl.entrance_pos,
            instance_id: Some(instance_id),
            reason: TeleportReason::EntrancePortal,
        })
        .collect();
    Ok(orders)
}

/// Generate a teleport order for a player exiting via the exit portal.
///
/// Moves the player to the overworld exit position and clears their instance.
pub fn exit_instance(
    instance_id: u32,
    manager: &InstanceManager,
    registry: &InstanceTemplateRegistry,
    player: u64,
) -> Result<InstanceTeleport, InstanceError> {
    let inst = manager
        .get(instance_id)
        .ok_or(InstanceError::InstanceNotFound)?;
    if !inst.contains_player(player) {
        return Err(InstanceError::InstanceNotFound);
    }
    let tmpl = registry.get(inst.map_id).ok_or(InstanceError::UnknownMap)?;

    Ok(InstanceTeleport {
        player,
        destination: tmpl.exit_pos,
        instance_id: None,
        reason: TeleportReason::ExitPortal,
    })
}

/// Generate a teleport order for a player using hearthstone inside an instance.
///
/// Teleports to the player's hearth location (passed in), clearing their instance.
pub fn hearthstone_out(
    instance_id: u32,
    manager: &InstanceManager,
    player: u64,
    hearth_pos: WorldPosition,
) -> Result<InstanceTeleport, InstanceError> {
    let inst = manager
        .get(instance_id)
        .ok_or(InstanceError::InstanceNotFound)?;
    if !inst.contains_player(player) {
        return Err(InstanceError::InstanceNotFound);
    }
    Ok(InstanceTeleport {
        player,
        destination: hearth_pos,
        instance_id: None,
        reason: TeleportReason::Hearthstone,
    })
}

/// Generate teleport orders for all players when an instance is reset/destroyed.
///
/// Moves everyone to the exit position. Used on scheduled reset or leader reset
/// when the server needs to evacuate any remaining players.
pub fn evacuate_instance(
    instance_id: u32,
    manager: &InstanceManager,
    registry: &InstanceTemplateRegistry,
) -> Result<Vec<InstanceTeleport>, InstanceError> {
    let inst = manager
        .get(instance_id)
        .ok_or(InstanceError::InstanceNotFound)?;
    let tmpl = registry.get(inst.map_id).ok_or(InstanceError::UnknownMap)?;

    let orders = inst
        .players
        .iter()
        .map(|&player| InstanceTeleport {
            player,
            destination: tmpl.exit_pos,
            instance_id: None,
            reason: TeleportReason::ExitPortal,
        })
        .collect();
    Ok(orders)
}

/// Look up which instance a player should enter for a given map+difficulty.
///
/// If the player's group already has an active instance, returns that.
/// If the player has a lockout binding, returns that instance ID.
/// Otherwise returns `None` (caller should create a new instance).
pub fn resolve_instance_for_entry(
    manager: &InstanceManager,
    group_leader: u64,
    map_id: u16,
    difficulty: Difficulty,
) -> Option<u32> {
    manager.find_group_instance(group_leader, map_id, difficulty)
}

#[cfg(test)]
#[path = "instance_teleport_tests.rs"]
mod tests;
