//! `MotionMaster::MovePoint` (PointMovementGenerator): a creature moves in a
//! straight line to a point at its walk or run speed (`Unit::SetWalk`), then its
//! AI gets `MovementInform(POINT_MOTION_TYPE, id)`.

use bevy::prelude::*;
use shared::components::Position;

use super::{Hook, MotionType};
use crate::combat::DeathState;
use crate::creature_ai::SpawnPoint;
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};

/// Base walk and run speeds, yards per second (`baseMoveSpeed`).
const BASE_WALK_SPEED: f32 = 2.5;
const BASE_RUN_SPEED: f32 = 7.0;

/// `MOVEMENTFLAG_WALKING`.
#[derive(Component, Debug, Clone, Copy)]
pub struct Walking;

/// A running `PointMovementGenerator`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PointMovement {
    pub id: u32,
    /// Bevy space.
    pub target: Position,
}

pub(crate) fn move_point(world: &mut World, unit: Entity, id: u32, target: Position) {
    if let Ok(mut entity) = world.get_entity_mut(unit) {
        entity.insert(PointMovement { id, target });
    }
}

pub(crate) fn set_walk(world: &mut World, unit: Entity, walk: bool) {
    let Ok(mut entity) = world.get_entity_mut(unit) else {
        return;
    };
    if walk {
        entity.insert(Walking);
    } else {
        entity.remove::<Walking>();
    }
}

/// Advance every point movement; arrivals inform the creature's AI.
pub(crate) fn point_movement_system(world: &mut World) {
    let dt = world.resource::<Time>().delta_secs();
    let mut arrived = Vec::new();
    let mut moved = Vec::new();
    let mut movers = world.query_filtered::<(
        Entity,
        &mut Position,
        &PointMovement,
        &SpawnPoint,
        Has<Walking>,
    ), Without<DeathState>>();
    for (entity, mut position, movement, spawn, walking) in movers.iter_mut(world) {
        let speed = if walking {
            spawn.speed_walk * BASE_WALK_SPEED
        } else {
            spawn.speed_run * BASE_RUN_SPEED
        };
        let (next, reached) = step_toward(*position, movement.target, speed * dt);
        if *position != next {
            *position = next;
            moved.push(entity);
        }
        if reached {
            arrived.push((entity, movement.id));
        }
    }
    if let Some(queue) = world.get_resource::<InterestDirtyQueue>() {
        for entity in moved {
            mark_interest_position_changed(Some(queue), entity);
        }
    }
    for (entity, id) in arrived {
        world.entity_mut(entity).remove::<PointMovement>();
        super::dispatch(world, entity, Hook::MovementInform(MotionType::Point, id));
    }
}

/// Move up to `step` yards toward `target`; whether it is reached.
fn step_toward(from: Position, target: Position, step: f32) -> (Position, bool) {
    let delta = Vec3::new(target.x - from.x, target.y - from.y, target.z - from.z);
    let distance = delta.length();
    if distance <= step || distance < 0.01 {
        return (target, true);
    }
    let moved = delta / distance * step;
    (
        Position {
            x: from.x + moved.x,
            y: from.y + moved.y,
            z: from.z + moved.z,
        },
        false,
    )
}
