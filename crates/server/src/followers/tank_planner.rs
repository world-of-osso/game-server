//! Shared planner owns candidate collection and movement, never interrupt casts.
use super::Follower;
use bevy::prelude::*;

pub(super) fn read_engaged_caster_candidates(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    party: &[Entity],
) -> Vec<Entity> {
    super::ai::engaged_enemies(world, follower, unit, party)
        .into_iter()
        .filter(|(enemy, _)| {
            let interruptible = world
                .get::<shared::casting::CastState>(*enemy)
                .is_some_and(|cast| cast.interruptible);
            let protected = crate::auras::has_breakable_by_damage_cc(world, *enemy);
            interruptible && !protected
        })
        .map(|(enemy, _)| enemy)
        .collect()
}
