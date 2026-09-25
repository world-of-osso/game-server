//! Server-side repositioning of a player (graveyard release, resurrection, admin
//! `set-position`, taxi arrival). The client moves its own player from input, so
//! every such move bumps `MovementControl::epoch`, and the owning client adopts
//! the replicated `Position` when the epoch changes.

use bevy::prelude::*;
use shared::components::MovementControl;

use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};

/// Call after writing a player's `Position` server-side.
pub(crate) fn teleported(
    dirty_queue: Option<&InterestDirtyQueue>,
    player: Entity,
    control: &mut MovementControl,
) {
    mark_interest_position_changed(dirty_queue, player);
    control.epoch = control.epoch.wrapping_add(1);
}
