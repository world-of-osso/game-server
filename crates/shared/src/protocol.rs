use serde::{Deserialize, Serialize};

use crate::types::*;

/// Player inputs sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub enum PlayerInput {
    Move { dx: f32, dy: f32, dz: f32 },
    CastSpell { spell_id: SpellId, target_entity: u64 },
    Interact { target_entity: u64 },
    Jump,
    StopMove,
}

/// Channel identifiers for lightyear networking.
#[derive(Debug)]
pub enum GameChannel {
    /// Position updates — unreliable, latest-wins.
    Movement,
    /// Combat events — reliable, ordered.
    Combat,
    /// Chat messages — reliable, ordered.
    Chat,
}
