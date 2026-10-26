//! Shared melee intent after native movement abilities, not a replacement for their movement.
//! SimC76f8118 sc_demon_hunter.cpp1559–1611 returns over distance/run_speed.
use bevy::prelude::*;
use shared::components::{MovementControl, Position};
use crate::simc_apl_native::FixtureMovement;
use super::super::movement;

struct WalkStep {
    target: Entity,
    position: Position,
    speed: f32,
    arrived: bool,
}

pub(super) fn advance(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let actors: Vec<_> = world.query_filtered::<Entity, With<FixtureMovement>>()
        .iter(world).collect();
    for actor in actors {
        advance_actor(world, actor, delta);
    }
}

fn advance_actor(world: &mut World, actor: Entity, delta: f32) {
    let step = read_walk_step(world, actor, delta);
    if let Some(mut moving) = world.get_mut::<crate::networking::Moving>(actor) {
        moving.0 = step.as_ref().is_some_and(|step| !step.arrived);
    }
    let Some(step) = step else { return; };
    crate::creature_ai::move_follower(world, actor, step.position, step.speed);
    super::face_actor_toward_primary(world, actor, step.target);
    // The encounter's position=back contract survives moving across the pack.
    let opponents: Vec<_> = world.query_filtered::<Entity, With<crate::creature_scripts::TrainingDummy>>()
        .iter(world).collect();
    for target in opponents {
        super::face_dummy_away_from_actor(world, actor, target);
    }
}

fn read_walk_step(world: &World, actor: Entity, delta: f32) -> Option<WalkStep> {
    if world.get::<MovementControl>(actor).is_some_and(|control| control.controlled)
        || !movement::can_move(world, actor) {
        return None;
    }
    let target = world.get::<crate::combat::AutoAttack>(actor)?.victim;
    if world.get::<shared::components::Health>(target)?.current <= 0.0 {
        return None;
    }
    let from = *world.get::<Position>(actor)?;
    let to = *world.get::<Position>(target)?;
    walk_toward_melee(target, from, to, movement::read_speed(world, actor), delta)
}

fn walk_toward_melee(target: Entity, from: Position, to: Position, speed: f32, delta: f32) -> Option<WalkStep> {
    let offset = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
    let distance = offset.length();
    let remaining = distance - crate::combat::MELEE_RANGE;
    if remaining <= 0.0 || speed <= 0.0 {
        return None;
    }
    let step = (speed * delta).min(remaining);
    let position = Vec3::new(from.x, from.y, from.z) + offset * (step / distance);
    Some(WalkStep { target, position: Position { x: position.x, y: position.y, z: position.z }, speed, arrived: step >= remaining })
}
