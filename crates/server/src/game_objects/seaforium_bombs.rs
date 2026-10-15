//! TC a352b1fa GameObject::Update charges2: ten-second fuse, one cast, deactivation.
//! Ownership stays in SummonedObject; no raw faction/template overrides.
use bevy::prelude::*;

use super::summoned::SummonedObject;
use crate::maps::InMap;
use crate::spell_cast::{now, try_cast_triggered};

pub(super) const SCRIPT_NAME: &str = "go_massive_seaforium_charge";
const ARMING_SECONDS: f64 = 10.0;

#[derive(Component)]
struct SeaforiumFuse {
    spell_id: u32,
    detonate_at: f64,
}

pub(super) fn arm(world: &mut World, object: Entity, spell_id: u32) {
    let detonate_at = now(world) + ARMING_SECONDS;
    world.entity_mut(object).insert(SeaforiumFuse {
        spell_id,
        detonate_at,
    });
}

/// TC WorldObject::GetAffectingPlayer / GetReactionTo uses the live in-map owner.
/// The spell still originates at the physical game object, not at that player.
pub(crate) fn find_caster_controller(world: &World, caster: Entity) -> Option<Entity> {
    world.get::<SeaforiumFuse>(caster)?;
    let owner = world.get::<SummonedObject>(caster)?.owner?;
    (world.get::<InMap>(owner) == world.get::<InMap>(caster)).then_some(owner)
}

pub(super) fn update(world: &mut World) {
    let time = now(world);
    let mut bombs = world.query::<(Entity, &SeaforiumFuse)>();
    let due: Vec<_> = bombs
        .iter(world)
        .filter(|(_, fuse)| time >= fuse.detonate_at)
        .map(|(entity, fuse)| (entity, fuse.spell_id))
        .collect();
    for (object, spell_id) in due {
        if !try_cast_triggered(world, object, spell_id, None, 0) {
            error!(
                ?object,
                spell_id, "Source Seaforium detonation cast rejected"
            );
        }
        // TC deactivates after the cast attempt, including a rejected spell.
        world.despawn(object);
    }
}
