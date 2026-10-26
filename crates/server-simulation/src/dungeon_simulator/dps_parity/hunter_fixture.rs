//! Native default-pet record for explicit Hunter actors.
use bevy::prelude::*;

pub(super) fn provision_pet(world: &mut World, actor: Entity) -> Result<(), String> {
    // SimC76f8118 sc_hunter.cpp1052: reuse the existing native default-duck
    // record policy; ordinary Call Pet still supplies the actual summon.
    crate::hunter_pet::provision_follower_duck(world, actor).map_err(str::to_owned)
}
