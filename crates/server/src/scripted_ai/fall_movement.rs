//! TC a352b1fa MotionMaster1001–1046 and MoveSpline114–127: creature MoveFall.
//! Represented subset: static dry map floor, no transport or hover movement flag.
use bevy::prelude::*;
use shared::components::Position;

use super::{Hook, MotionType};
use crate::combat::DeathState;
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;

const TERMINAL_SPEED: f32 = 60.148_003;
const GRAVITY: f32 = shared::skyriding::GRAVITY;

#[derive(Component)]
pub(crate) struct FallMovement {
    id: u32,
    from: Position,
    floor: f32,
    elapsed_ms: u32,
    duration_ms: u32,
    started: std::time::Duration,
}

pub(crate) fn move_fall(world: &mut World, unit: Entity, id: u32) {
    let Some((position, map)) = world
        .get::<Position>(unit)
        .copied()
        .zip(world.get::<InMap>(unit).copied())
    else {
        warn!(?unit, "MoveFall requires position and map");
        return;
    };
    if world
        .get::<crate::auras::ActiveAuras>(unit)
        .is_some_and(|auras| auras.is_stunned() || auras.has_type(crate::auras::handlers::MOD_ROOT))
    {
        return;
    }
    let Some(mut ground) = world.get_resource_mut::<GroundMap>() else {
        warn!(?unit, "MoveFall requires map collision");
        return;
    };
    let GroundProbe::Supported(floor) =
        ground.creature_ground_in(map, Vec3::new(position.x, position.y, position.z))
    else {
        warn!(?unit, "MoveFall has no supported floor");
        return;
    };
    if (position.y - floor).abs() < 0.1 {
        return;
    }
    let duration_ms = (fall_seconds(position.y - floor) * 1000.0) as u32;
    if duration_ms == 0 {
        return;
    }
    let started = world.resource::<Time>().elapsed();
    super::movement::clear_point_movement(world, unit);
    world.entity_mut(unit).insert(FallMovement {
        id,
        from: position,
        floor,
        elapsed_ms: 0,
        duration_ms,
        started,
    });
}

fn fall_seconds(drop: f32) -> f32 {
    if drop < 0.0 {
        return 0.0;
    }
    let terminal_time = TERMINAL_SPEED / GRAVITY;
    let terminal_drop = TERMINAL_SPEED * TERMINAL_SPEED / (2.0 * GRAVITY);
    if drop >= terminal_drop {
        (drop - terminal_drop) / TERMINAL_SPEED + terminal_time
    } else {
        (2.0 * drop / GRAVITY).sqrt()
    }
}

fn fall_elevation(seconds: f32) -> f32 {
    let terminal_time = TERMINAL_SPEED / GRAVITY;
    if seconds > terminal_time {
        TERMINAL_SPEED * (seconds - terminal_time) + GRAVITY * terminal_time * terminal_time * 0.5
    } else {
        seconds * (seconds * GRAVITY * 0.5)
    }
}

pub(crate) fn release_finished_falls(
    mut commands: Commands,
    falls: Query<(Entity, &FallMovement, Has<DeathState>)>,
) {
    for (unit, fall, dead) in &falls {
        if dead || fall.elapsed_ms == fall.duration_ms {
            commands.entity(unit).remove::<FallMovement>();
        }
    }
}

pub(crate) fn advance_falls(world: &mut World) {
    let now = world.resource::<Time>().elapsed();
    let delta_ms = world.resource::<Time>().delta().as_millis() as u32;
    let mut updates = Vec::new();
    let mut query =
        world.query_filtered::<(Entity, &mut FallMovement, &mut Position), Without<DeathState>>();
    for (unit, mut fall, mut position) in query.iter_mut(world) {
        // Launch initializes at t=0, never reuses the point-arrival tick's delta.
        if fall.started == now || fall.elapsed_ms == fall.duration_ms {
            continue;
        }
        fall.elapsed_ms = (fall.elapsed_ms + delta_ms).min(fall.duration_ms);
        let arrived = fall.elapsed_ms == fall.duration_ms;
        *position = Position {
            y: if arrived {
                fall.floor
            } else {
                (fall.from.y - fall_elevation(fall.elapsed_ms as f32 / 1000.0)).max(fall.floor)
            },
            ..fall.from
        };
        updates.push((unit, arrived.then_some(fall.id)));
    }
    for (unit, arrival) in updates {
        crate::interest::mark_interest_position_changed(
            world.get_resource::<crate::interest::InterestDirtyQueue>(),
            unit,
        );
        if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
            queue.mark(unit);
        }
        if let Some(id) = arrival {
            super::dispatch(world, unit, Hook::MovementInform(MotionType::Effect, id));
        }
    }
}
