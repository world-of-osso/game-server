//! TC a352b1fa Spell::CheckCast, GameObjectFocusCheck and WorldObject::_IsWithinDist.
//! Only the authenticated, readable Retail69933 cooking-focus requirements are here.
#[path = "cooking_focus_spells.rs"]
mod source;

use bevy::prelude::*;
use shared::components::{Player, Position, UnitScale};
use shared::protocol::GameObjectInfo;

use crate::maps::InMap;
use crate::spell_cast::CastFailure;

/// Player.cpp:1577 sets combat reach to scale * DEFAULT_PLAYER_COMBAT_REACH.
const PLAYER_COMBAT_REACH: f32 = 1.5;
pub(super) const COOKING_FIRE: u32 = 4;

pub(crate) fn check_cast(world: &World, caster: Entity, spell: u32) -> Result<(), CastFailure> {
    if source::COOKING_SPELLS.binary_search(&spell).is_err() {
        return Ok(());
    }
    if world.get::<Player>(caster).is_none() {
        return Err(CastFailure::Unsupported(
            "non-player cooking focus is unavailable".into(),
        ));
    }
    let position = world
        .get::<Position>(caster)
        .ok_or(CastFailure::InvalidTarget)?;
    let map = world
        .get::<InMap>(caster)
        .ok_or(CastFailure::InvalidTarget)?;
    let scale = world
        .get::<UnitScale>(caster)
        .ok_or_else(|| CastFailure::Unsupported("cooking caster scale is unavailable".into()))?;
    let origin = Vec3::new(position.x, position.y, position.z);
    let reach = PLAYER_COMBAT_REACH * scale.0;
    if has_live_cooking_focus(world, origin, *map, reach) {
        Ok(())
    } else {
        Err(CastFailure::Unsupported("Requires Cooking Fire".into()))
    }
}

fn has_live_cooking_focus(world: &World, origin: Vec3, map: InMap, reach: f32) -> bool {
    let Some(mut objects) = world.try_query::<(&GameObjectInfo, &Position, &InMap)>() else {
        return false;
    };
    objects.iter(world).any(|(info, position, object_map)| {
        if *object_map != map {
            return false;
        }
        let Some(radius) = super::summoned::cooking_focus_radius(world, info.entry) else {
            return false;
        };
        let location = Vec3::new(position.x, position.y, position.z);
        origin.distance_squared(location) < (radius + reach).powi(2)
    })
}
