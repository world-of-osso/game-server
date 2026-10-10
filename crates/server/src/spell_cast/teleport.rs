//! Data-driven displacement. TC a352b1fa Spell.cpp SelectImplicitCasterDestTargets
//! and SpellEffects.cpp EffectLeap / EffectTeleportUnits. Retail Blink (1953) is
//! effect 29, not the legacy teleport effect 15; current teleport is effect 252.
use super::{effects::EffectCtx, targets::Dest};
use crate::maps::InMap;
use bevy::prelude::*;

const WALL_CLEARANCE: f32 = 0.5;
const COLLISION_SEARCH_STEPS: usize = 16;

/// Stop before the first blocking triangle using the same baked LOS query as casts.
/// Prefix queries are monotonic even when the endpoint is beyond a thin wall.
fn clamp_to_wall(
    los: &crate::line_of_sight::LineOfSight,
    map: InMap,
    start: Vec3,
    end: Vec3,
) -> Vec3 {
    let offset = Vec3::Y;
    if los.clear(map, start + offset, end + offset) {
        return end;
    }
    let mut clear_fraction = 0.0;
    let mut blocked_fraction = 1.0;
    for _ in 0..COLLISION_SEARCH_STEPS {
        let fraction = (clear_fraction + blocked_fraction) * 0.5;
        if los.clear(map, start + offset, start.lerp(end, fraction) + offset) {
            clear_fraction = fraction;
        } else {
            blocked_fraction = fraction;
        }
    }
    let distance = start.distance(end);
    let safe_fraction = (clear_fraction - WALL_CLEARANCE / distance).max(0.0);
    start.lerp(end, safe_fraction)
}

pub(super) fn forward_destination(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
) -> Option<Dest> {
    let map = *world.get::<InMap>(target)?;
    let start = super::targets::position(world, target)?;
    let dest = ctx.dest?;
    // Production loads the LOS bake and ground. Missing resources are a failed move,
    // never an unchecked teleport through geometry.
    let Some(los) = world.get_resource::<crate::line_of_sight::LineOfSight>() else {
        error!("Spell {}: cannot displace without LOS data", ctx.spell.id);
        return None;
    };
    let mut end = clamp_to_wall(los, map, start, dest.position);
    let ground = world
        .get_resource_mut::<crate::ground::GroundMap>()?
        .ground_at(map.map_id, end);
    let crate::ground::GroundProbe::Supported(height) = ground else {
        error!(
            "Spell {}: destination has no supported ground",
            ctx.spell.id
        );
        return None;
    };
    end.y = height;
    Some(Dest {
        position: end,
        ..dest
    })
}
