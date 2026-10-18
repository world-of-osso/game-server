//! Shared planner owns candidate collection and movement, never interrupt casts.
#[cfg(test)]
use super::Follower;
use bevy::prelude::*;

/// The planner's admitted movement destination; kits must not replace it.
#[derive(Component, Clone, Copy)]
pub(super) struct AdmittedPosition(pub Vec3);

#[derive(Component)]
struct AppliedMovementAt(f64);

/// Consume the same admitted point at both kit pursuit and idle regroup boundaries.
pub(super) fn apply_admitted_position(
    world: &mut World,
    unit: Entity,
    target: Option<Entity>,
) -> bool {
    let Some(goal) = world.get::<AdmittedPosition>(unit).copied() else {
        return false;
    };
    world
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert((
            crate::creature_ai::CreatureState::Idle,
            crate::networking::CombatTarget(target),
        ));
    let now = world.resource::<Time>().elapsed_secs_f64();
    if world
        .get::<AppliedMovementAt>(unit)
        .is_some_and(|at| at.0 == now)
    {
        return true;
    }
    let Some(from) = world.get::<shared::components::Position>(unit).copied() else {
        warn!(?unit, "Admitted tank movement lacks a position");
        return true;
    };
    crate::creature_ai::move_follower(world, unit, from, 0.0);
    let here = Vec3::new(from.x, from.y, from.z);
    if goal.0 == here {
        return true;
    }
    if let Err(error) = advance_admitted_once(world, unit, from, goal.0) {
        warn!(?unit, %error, "Admitted tank movement held");
    }
    true
}

fn advance_admitted_once(
    world: &mut World,
    unit: Entity,
    from: shared::components::Position,
    goal: Vec3,
) -> Result<(), String> {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let casting = world
        .get::<shared::casting::CastState>(unit)
        .is_some_and(|cast| !cast.cast_while_moving);
    if casting {
        return Ok(());
    }
    if !goal.is_finite() {
        return Err("admitted destination is non-finite".into());
    }
    let map = *world
        .get::<crate::maps::InMap>(unit)
        .ok_or("admitted movement lacks map")?;
    if world
        .get_resource::<crate::navigation::Navigation>()
        .is_none()
        || world.get_resource::<crate::ground::GroundMap>().is_none()
    {
        return Err("admitted movement requires authored navigation and ground".into());
    }
    let to = shared::components::Position {
        x: goal.x,
        y: goal.y,
        z: goal.z,
    };
    if super::hazards::point_route_crosses_hostile_field(world, unit, to) {
        return Err("admitted route crosses an active hostile field".into());
    }
    world.entity_mut(unit).insert(AppliedMovementAt(now));
    super::hazards::advance_grounded_route(
        world,
        unit,
        map,
        from,
        to,
        crate::navigation::Goal::Point,
    );
    Ok(())
}

#[cfg(test)]
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
