//! Stable storage for explicit headless Hunter actors, without a fake connection.
use bevy::prelude::*;

pub(super) fn provision_pet(world: &mut World, actor: Entity) -> Result<(), String> {
    // Real login restores PetStable from a character session. The explicit parity
    // Player has no PlayerLink/InWorld, so retain its identity with runtime-only
    // storage rather than inventing an authenticated character ID or follower.
    if world.get::<shared::components::Player>(actor).is_some()
        && world.get::<crate::hunter_pet::PetStable>(actor).is_none()
    {
        world.entity_mut(actor).insert(crate::hunter_pet::PetStable {
            character_id: None,
            stored: Default::default(),
            summoned: None,
        });
    }
    // SimC76f8118 sc_hunter.cpp1052: reuse the existing native default-duck
    // record policy; ordinary Call Pet still supplies the actual summon.
    crate::hunter_pet::provision_follower_duck(world, actor).map_err(str::to_owned)
}
