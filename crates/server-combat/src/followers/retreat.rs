//! Position Havoc actors for a real Vengeful Retreat through their target.
use bevy::prelude::*;
use shared::components::{MovementControl, Position, Rotation};

#[derive(Component)]
pub struct RetreatSetup;

/// Walk to the edge of melee, then face away so Retreat moves through the target.
/// Returns false while the actor is still walking or cannot move.
pub fn prepare_havoc_retreat(
    world: &mut World,
    actor: Entity,
    target: Entity,
) -> Result<bool, String> {
    if world
        .get::<MovementControl>(actor)
        .is_some_and(|control| control.controlled)
        || world
            .get::<shared::casting::CastState>(actor)
            .is_some_and(|cast| !cast.cast_while_moving)
        || world.get::<crate::auras::Confused>(actor).is_some()
        || world.get::<crate::auras::Feared>(actor).is_some()
    {
        return Ok(false);
    }
    let from = *world
        .get::<Position>(actor)
        .ok_or("Retreat actor lacks position")?;
    let to = *world
        .get::<Position>(target)
        .ok_or("Retreat target lacks position")?;
    let offset = Vec3::new(from.x - to.x, 0.0, from.z - to.z);
    let distance = offset.length();
    if distance <= f32::EPSILON {
        return Err("Retreat actor and target share a position".into());
    }
    let range = crate::scripted_ai::model_reach::melee_range(world, actor, target)
        .ok_or("Retreat setup requires native melee reach")?;
    let desired = (range - 0.02).max(0.0);
    let difference = desired - distance;
    if difference.abs() > 0.001 {
        world.entity_mut(actor).insert(RetreatSetup);
        let speed = world
            .get::<crate::creature_ai::SpawnPoint>(actor)
            .map_or(1.0, |spawn| spawn.speed_run)
            * crate::creature_ai::BASE_RUN_SPEED
            * world
                .get::<crate::auras::AuraStatMods>(actor)
                .map_or(1.0, |mods| mods.movement_speed);
        if speed <= 0.0 {
            return Ok(false);
        }
        let stride = speed * world.resource::<Time>().delta_secs();
        let step = difference.clamp(-stride, stride);
        let point = Vec3::new(from.x, from.y, from.z) + offset * (step / distance);
        crate::creature_ai::move_follower(
            world,
            actor,
            Position {
                x: point.x,
                y: point.y,
                z: point.z,
            },
            speed,
        );
        return Ok(false);
    }
    world.entity_mut(actor).remove::<RetreatSetup>();
    let mut rotation = world
        .get_mut::<Rotation>(actor)
        .ok_or("Retreat actor lacks rotation")?;
    rotation.y = offset.x.atan2(offset.z);
    Ok(true)
}
