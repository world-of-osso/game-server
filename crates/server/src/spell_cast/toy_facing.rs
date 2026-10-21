//! Runtime Toy FacingCasterFlags; TC a352b1fa Spell::CheckRange,
//! Position::HasInArc/IsInDist and Unit::IsWithinBoundaryRadius.
use bevy::prelude::*;
use shared::components::{ModelDisplay, Player, Rotation, UnitScale};

use super::{CastFailure, targets, toy_requirements::ToyCastingRequirements};

const MIN_MELEE_REACH: f32 = 2.0;
const PLAYER_BOUNDING_RADIUS: f32 = 0.389;
const IN_FRONT: u32 = 1;
const PET_BOUNDING_RADIUS: f32 = 1.0;

fn bounding_radius(world: &World, target: Entity) -> Result<f32, CastFailure> {
    let scale = world.get::<UnitScale>(target).map_or(1.0, |scale| scale.0);
    if world.get::<Player>(target).is_some() {
        return Ok(PLAYER_BOUNDING_RADIUS * scale);
    }
    if world.get::<crate::hunter_pet::HunterPet>(target).is_some() {
        return Ok(PET_BOUNDING_RADIUS * scale);
    }
    let display = world
        .get::<ModelDisplay>(target)
        .ok_or_else(|| CastFailure::Unsupported("facing target has no model display".into()))?;
    let radius = world
        .resource::<ToyCastingRequirements>()
        .facing
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
    let required = world
        .resource::<ToyCastingRequirements>()
        .facing
        .spells
        .get(&spell)
        .is_some_and(|flags| flags & IN_FRONT != 0);
    if !required || world.get::<Player>(caster).is_none() {
        return Ok(());
    }
    let Some(target) = target
        .filter(|&unit| unit != caster && world.get::<shared::components::Health>(unit).is_some())
    else {
        return Ok(());
    };
    let from = targets::position(world, caster).ok_or(CastFailure::InvalidTarget)?;
    let to = targets::position(world, target).ok_or(CastFailure::InvalidTarget)?;
    let rotation = world
        .get::<Rotation>(caster)
        .ok_or_else(|| CastFailure::Unsupported("facing caster has no rotation".into()))?;
    let offset = to - from;
    if in_front(rotation.y, offset) {
        return Ok(());
    }
    check_boundary(world, caster, target, offset.length_squared())
}

fn in_front(yaw: f32, offset: Vec3) -> bool {
    let orientation = yaw - std::f32::consts::FRAC_PI_2;
    let angle = (-offset.z).atan2(offset.x) - orientation;
    let relative_angle =
        (angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    relative_angle.abs() <= std::f32::consts::FRAC_PI_2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_info_radius_scales_defaults_and_preserves_authored_radius() {
        let mut world = World::new();
        world.insert_resource(super::super::toy_requirements::fixture());
        let missing = world
            .spawn((ModelDisplay { display_id: 124440 }, UnitScale(10.0)))
            .id();
        assert_eq!(bounding_radius(&world, missing).unwrap(), 0.389 * 10.0);
        let sourced = world
            .spawn((ModelDisplay { display_id: 42247 }, UnitScale(2.0)))
            .id();
        assert_eq!(bounding_radius(&world, sourced).unwrap(), 12.0);
        let caster = world.spawn(crate::maps::InMap::continent(0)).id();
        world
            .entity_mut(missing)
            .insert(crate::maps::InMap::continent(0));
        assert!(check_boundary(&world, caster, missing, 9.0).is_ok());
        assert!(matches!(
            check_boundary(&world, caster, missing, 16.0),
            Err(CastFailure::NotInFront)
        ));
    }
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
