//! Toy FacingCasterFlags from local69933; TC a352b1fa Spell::CheckRange,
//! Position::HasInArc/IsInDist and Unit::IsWithinBoundaryRadius.
use std::{collections::HashMap, sync::OnceLock};

use bevy::prelude::*;
use serde::Deserialize;
use shared::components::{ModelDisplay, Player, Rotation, UnitScale};

use super::{CastFailure, targets};

const MIN_MELEE_REACH: f32 = 2.0;
const PLAYER_BOUNDING_RADIUS: f32 = 0.389;
const IN_FRONT: u32 = 1;

#[derive(Deserialize)]
struct Requirements {
    spells: HashMap<u32, u32>,
    model_radii: HashMap<u32, f32>,
}

fn requirements() -> &'static Requirements {
    static DATA: OnceLock<Requirements> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("toy_facing_requirements.json"))
            .expect("valid generated Toy facing and TDB model-radius manifest")
    })
}

fn bounding_radius(world: &World, target: Entity) -> Result<f32, CastFailure> {
    let scale = world.get::<UnitScale>(target).map_or(1.0, |scale| scale.0);
    if world.get::<Player>(target).is_some() {
        return Ok(PLAYER_BOUNDING_RADIUS * scale);
    }
    let display = world
        .get::<ModelDisplay>(target)
        .ok_or_else(|| CastFailure::Unsupported("facing target has no model display".into()))?;
    let radius = requirements()
        .model_radii
        .get(&display.display_id)
        .ok_or_else(|| {
            CastFailure::Unsupported(format!(
                "facing target display{} has no TDB bounding radius",
                display.display_id
            ))
        })?;
    Ok(radius * scale)
}

pub(super) fn check_cast(
    world: &World,
    caster: Entity,
    spell: u32,
    target: Option<Entity>,
) -> Result<(), CastFailure> {
    let required = requirements()
        .spells
        .get(&spell)
        .is_some_and(|flags| flags & IN_FRONT != 0);
    if !required || world.get::<Player>(caster).is_none() {
        return Ok(());
    }
    let Some(target) = target.filter(|&unit| unit != caster) else {
        return Ok(());
    };
    let from = targets::position(world, caster).ok_or(CastFailure::InvalidTarget)?;
    let to = targets::position(world, target).ok_or(CastFailure::InvalidTarget)?;
    let rotation = world
        .get::<Rotation>(caster)
        .ok_or_else(|| CastFailure::Unsupported("facing caster has no rotation".into()))?;
    let orientation = rotation.y - std::f32::consts::FRAC_PI_2;
    let offset = to - from;
    let angle = (-offset.z).atan2(offset.x) - orientation;
    let relative_angle =
        (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    if relative_angle.abs() <= std::f32::consts::FRAC_PI_2 {
        return Ok(());
    }
    check_boundary(world, caster, target, offset.length_squared())
}

fn check_boundary(
    world: &World,
    caster: Entity,
    target: Entity,
    distance_sq: f32,
) -> Result<(), CastFailure> {
    let radius = bounding_radius(world, target)?.max(MIN_MELEE_REACH);
    // Position::IsInDist is strict exact3D; WorldObject::GetDistance and its
    // CombatReach subtraction are deliberately not involved.
    if targets::same_map(world, caster, target) && distance_sq < radius * radius {
        Ok(())
    } else {
        Err(CastFailure::NotInFront)
    }
}
