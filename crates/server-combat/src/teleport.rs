//! Server-side repositioning of a player (graveyard release, resurrection, admin
//! `set-position`, taxi arrival). The client moves its own player from input, so
//! every such move bumps `MovementControl::epoch`, and the owning client adopts
//! the replicated `Position` when the epoch changes.

use bevy::prelude::*;
use shared::components::MovementControl;

use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};

/// `teleported` from an exclusive system.
pub(crate) fn teleported_in(world: &mut World, player: Entity) {
    mark_interest_position_changed(world.get_resource::<InterestDirtyQueue>(), player);
    if let Some(mut control) = world.get_mut::<MovementControl>(player) {
        control.epoch = control.epoch.wrapping_add(1);
    }
}

/// Call after writing a player's `Position` server-side.
pub(crate) fn teleported(
    dirty_queue: Option<&InterestDirtyQueue>,
    player: Entity,
    control: &mut MovementControl,
) {
    mark_interest_position_changed(dirty_queue, player);
    control.epoch = control.epoch.wrapping_add(1);
}
