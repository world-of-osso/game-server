//! Server-owned movement permissions and aura teleport origins. Wire/engine projection is separate.
use std::collections::HashMap;

use bevy::prelude::*;
use shared::components::Position;

use crate::maps::InMap;

/// Movement permissions; kept separate from flight and feather fall.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct WorldAuraState {
    pub water_walk: bool,
    pub hover: bool,
    pub disable_gravity: bool,
    pub tracked_creatures: u32,
    pub movement_force_multiplier: f32,
}

/// The actual map copy and pose at application, keyed by the storing spell ID.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TeleportOrigin {
    pub map: InMap,
    pub position: Position,
    pub yaw: f32,
}

#[derive(Component, Debug, Default)]
pub(crate) struct AuraTeleportOrigins(pub HashMap<u32, TeleportOrigin>);
