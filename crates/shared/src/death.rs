//! Corpse and death system.
//!
//! Handles player death, corpse placement, ghost form, resurrection.
//! Ref: AzerothCore `Corpse.cpp`, `Player.cpp` (RepopAtGraveyard).

use serde::{Deserialize, Serialize};

/// Player alive/dead state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathState {
    /// Player is alive.
    Alive,
    /// Player has just died (HP reached 0). Corpse at death location.
    Dead,
    /// Player has released spirit and is a ghost at a graveyard.
    Ghost,
    /// Player is being resurrected (by spell or spirit healer).
    Resurrecting,
}

/// A player corpse left at the death location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Corpse {
    /// Player entity who died.
    pub player: u64,
    /// Map where the corpse is.
    pub map_id: u16,
    /// Death position.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Server timestamp when the player died.
    pub created_at: u64,
}

/// Result of a player dying.
#[derive(Debug, Clone, PartialEq)]
pub struct DeathResult {
    pub player: u64,
    pub corpse: Corpse,
}

/// Check if a player should die (HP reached 0).
pub fn check_death(current_hp: f32) -> bool {
    current_hp <= 0.0
}

/// Process a player death: create corpse at death location.
///
/// Returns the death result with the corpse data.
/// The server should:
/// 1. Set the player's `DeathState` to `Dead`
/// 2. Spawn the corpse entity at the death location
/// 3. Disable player interactions (combat, looting, trading, etc.)
pub fn on_death(
    player: u64,
    map_id: u16,
    x: f32,
    y: f32,
    z: f32,
    now: u64,
) -> DeathResult {
    DeathResult {
        player,
        corpse: Corpse { player, map_id, x, y, z, created_at: now },
    }
}

/// Why a death operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathError {
    /// Player is not dead.
    NotDead,
    /// Player is not a ghost.
    NotGhost,
    /// Player is already alive.
    AlreadyAlive,
    /// Player is already dead.
    AlreadyDead,
}

/// Validate and transition death state.
pub fn transition_state(
    current: DeathState,
    to: DeathState,
) -> Result<DeathState, DeathError> {
    match (current, to) {
        (DeathState::Alive, DeathState::Dead) => Ok(DeathState::Dead),
        (DeathState::Dead, DeathState::Ghost) => Ok(DeathState::Ghost),
        (DeathState::Dead, DeathState::Resurrecting) => Ok(DeathState::Resurrecting),
        (DeathState::Ghost, DeathState::Resurrecting) => Ok(DeathState::Resurrecting),
        (DeathState::Resurrecting, DeathState::Alive) => Ok(DeathState::Alive),
        (DeathState::Alive, DeathState::Alive) => Err(DeathError::AlreadyAlive),
        (DeathState::Dead, DeathState::Dead) => Err(DeathError::AlreadyDead),
        (DeathState::Alive, _) => Err(DeathError::NotDead),
        (DeathState::Dead, DeathState::Alive) => Err(DeathError::NotGhost),
        _ => Err(DeathError::NotDead),
    }
}

#[cfg(test)]
#[path = "death_tests.rs"]
mod tests;
