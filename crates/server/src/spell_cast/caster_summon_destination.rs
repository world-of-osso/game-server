//! Target32: TC Spell.cpp1613–1641/9286–9292, Object.cpp698–766/2787–2906.
//! Uses existing geometry; no new collision or unit-movement projection.
use bevy::prelude::*;
use shared::components::PlayerMotion;

use super::super::{CastFailure, attributes};
use super::Dest;
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;
use crate::spell_info::SpellInfo;

const FORCE_DEST_LOCATION: u32 = 0x1;
const FORCE_HEIGHT_CHANGE: f32 = 6.0;
const FORCE_BACKOFF_STEPS: usize = 10;

pub(super) fn read_destination(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    destination: Option<Dest>,
) -> Result<Option<Dest>, CastFailure> {
    let mut destination = destination.ok_or_else(|| {
        CastFailure::Unsupported("target32 requires caster position and source combat reach".into())
    })?;
    let map = world
        .get::<InMap>(caster)
        .copied()
        .ok_or(CastFailure::InvalidTarget)?;
    let origin = super::position(world, caster).ok_or(CastFailure::InvalidTarget)?;
    let flying = caster_can_fly(world, caster);
    let force = attributes(spell, 9) & FORCE_DEST_LOCATION != 0;
    let mut ground = world
        .get_resource_mut::<GroundMap>()
        .ok_or_else(|| CastFailure::Unsupported("target32 requires ground geometry".into()))?;
    destination.position = if force {
        read_forced_position(&mut ground, map, origin, destination.position)?
    } else {
        read_collision_position(&mut ground, map, origin, destination.position, flying)?
    };
    Ok(Some(destination))
}

fn caster_can_fly(world: &World, caster: Entity) -> bool {
    let gravity_disabled = world
        .get::<crate::scripted_ai::waypoint_path::GravityDisabled>(caster)
        .is_some();
    let flying_flags = world
        .get::<PlayerMotion>(caster)
        .is_some_and(|motion| motion.0 & (PlayerMotion::CAN_FLY | PlayerMotion::FLYING) != 0);
    gravity_disabled || flying_flags
}

fn read_collision_position(
    ground: &mut GroundMap,
    map: InMap,
    origin: Vec3,
    wanted: Vec3,
    flying: bool,
) -> Result<Vec3, CastFailure> {
    // TARGET32_FIRST_COLLISION_PARITY_UNIMPLEMENTED: this helper clips live GO
    // hulls at its 1yd ray/0.5yd clearance, not TC nav/static/half-height traces.
    let mut position = ground.clamp_live_movement(map, origin, wanted);
    let floor = read_floor(ground, map, position)?;
    // TARGET32_HOVER_SWIM_STATE_UNIMPLEMENTED: no invented offset or new projection.
    position.y = if flying { position.y.max(floor) } else { floor };
    Ok(position)
}

fn read_forced_position(
    ground: &mut GroundMap,
    map: InMap,
    origin: Vec3,
    wanted: Vec3,
) -> Result<Vec3, CastFailure> {
    let step = (wanted - origin) / FORCE_BACKOFF_STEPS as f32;
    let mut candidate = wanted;
    let mut position = origin;
    for _ in 0..FORCE_BACKOFF_STEPS {
        let floor = read_floor(ground, map, candidate)?;
        if (origin.y - floor).abs() <= FORCE_HEIGHT_CHANGE {
            position = candidate;
            break;
        }
        candidate -= step;
    }
    // TARGET32_HEIGHT_PROBE_PARITY_UNIMPLEMENTED: existing sampling is not TC's
    // MAX_HEIGHT-versus-near-height choice. No alternate geometry reader.
    position.y = read_floor(ground, map, position)?;
    Ok(position)
}

fn read_floor(ground: &mut GroundMap, map: InMap, position: Vec3) -> Result<f32, CastFailure> {
    match ground.creature_ground_in(map, position) {
        GroundProbe::Supported(height) => Ok(height),
        GroundProbe::Unloaded | GroundProbe::Unsupported => Err(CastFailure::InvalidTarget),
    }
}
