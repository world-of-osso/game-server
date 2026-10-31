//! Shared melee intent after native movement abilities, not a replacement for their movement.
//! SimC76f8118 sc_demon_hunter.cpp1559–1611 returns over distance/run_speed.
use super::super::movement;
use crate::simc_apl_native::FixtureMovement;
use bevy::prelude::*;
use server_follower_apl::literal_rotation::melee_intent::{
    DisplacementReturn, ReturningToMelee, forced_movement,
};
use shared::components::Position;

struct WalkStep {
    target: Entity,
    position: Position,
    speed: f32,
    arrived: bool,
}

pub(super) fn advance(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<FixtureMovement>>()
        .iter(world)
        .collect();
    for actor in actors {
        advance_actor(world, actor, delta);
    }
}

fn advance_actor(world: &mut World, actor: Entity, delta: f32) {
    if world
        .get::<crate::followers::retreat::RetreatSetup>(actor)
        .is_some()
    {
        return;
    }
    // A charge can finish inside melee reach facing along its short travel,
    // away from the opponent. The fixture still owns the actor's melee intent.
    // Do not override active forced movement or turn an active channel.
    if forced_movement(world, actor) {
        if world.get::<crate::combat::AutoAttack>(actor).is_some() {
            world.entity_mut(actor).insert(DisplacementReturn);
        }
        return;
    }
    clear_displacement_return_at_melee(world, actor);
    face_melee_intent(world, actor);
    let step = read_walk_step(world, actor, delta);
    if step.is_none() && world.get::<ReturningToMelee>(actor).is_none() {
        return;
    }
    let walking = step.as_ref().is_some_and(|step| !step.arrived);
    set_return_walking(world, actor, walking);
    let Some(step) = step else {
        return;
    };
    crate::creature_ai::move_follower(world, actor, step.position, step.speed);
    super::face_actor_toward_primary(world, actor, step.target);
    // The encounter's position=back contract survives moving across the pack.
    let opponents: Vec<_> = world
        .query_filtered::<Entity, With<crate::creature_scripts::TrainingDummy>>()
        .iter(world)
        .collect();
    for target in opponents {
        super::face_dummy_away_from_actor(world, actor, target);
    }
}

fn clear_displacement_return_at_melee(world: &mut World, actor: Entity) {
    let reached = world
        .get::<crate::combat::AutoAttack>(actor)
        .is_none_or(|attack| {
            if world
                .get::<shared::components::Health>(attack.victim)
                .is_none_or(|health| health.current <= 0.0)
            {
                return true;
            }
            let Some(from) = world.get::<Position>(actor) else {
                return true;
            };
            let Some(target) = world.get::<Position>(attack.victim) else {
                return true;
            };
            let Some(range) = crate::scripted_ai::model_reach::melee_range(world, actor, attack.victim) else {
                error!(?actor, target = ?attack.victim, "Melee return requires sourced combat reaches");
                return false;
            };
            (target.x - from.x).hypot(target.z - from.z) <= range
        });
    if reached {
        world.entity_mut(actor).remove::<DisplacementReturn>();
    }
}

fn set_return_walking(world: &mut World, actor: Entity, walking: bool) {
    if walking {
        world.entity_mut(actor).insert(ReturningToMelee);
    } else {
        world.entity_mut(actor).remove::<ReturningToMelee>();
    }
    if let Some(mut moving) = world.get_mut::<crate::networking::Moving>(actor) {
        moving.0 = walking;
    }
}

fn face_melee_intent(world: &mut World, actor: Entity) {
    if world.get::<shared::casting::CastState>(actor).is_some() {
        return;
    }
    let target = world
        .get::<crate::combat::AutoAttack>(actor)
        .map(|attack| attack.victim);
    if let Some(target) = target.filter(|target| world.get_entity(*target).is_ok()) {
        super::face_actor_toward_primary(world, actor, target);
    }
}

fn read_walk_step(world: &World, actor: Entity, delta: f32) -> Option<WalkStep> {
    if forced_movement(world, actor) || !movement::can_move(world, actor) {
        return None;
    }
    let target = world.get::<crate::combat::AutoAttack>(actor)?.victim;
    if world.get::<shared::components::Health>(target)?.current <= 0.0 {
        return None;
    }
    let from = *world.get::<Position>(actor)?;
    let to = *world.get::<Position>(target)?;
    let range = crate::scripted_ai::model_reach::melee_range(world, actor, target)?;
    walk_toward_melee(
        target,
        from,
        to,
        range,
        movement::read_speed(world, actor),
        delta,
    )
}

fn walk_toward_melee(
    target: Entity,
    from: Position,
    to: Position,
    range: f32,
    speed: f32,
    delta: f32,
) -> Option<WalkStep> {
    let offset = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
    let distance = offset.length();
    let remaining = distance - range;
    if remaining <= 0.0 || speed <= 0.0 {
        return None;
    }
    let step = (speed * delta).min(remaining);
    let position = Vec3::new(from.x, from.y, from.z) + offset * (step / distance);
    Some(WalkStep {
        target,
        position: Position {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        speed,
        arrived: step >= remaining,
    })
}
