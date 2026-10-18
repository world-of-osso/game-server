//! Shared planner owns candidate collection and movement, never interrupt casts.
use super::Follower;
use bevy::prelude::*;

pub(super) fn read_engaged_caster_candidates(
    _world: &mut World,
    _unit: Entity,
    _follower: &Follower,
    _party: &[Entity],
) -> Vec<Entity> {
    Vec::new()
}
