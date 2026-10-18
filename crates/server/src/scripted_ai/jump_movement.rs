//! TC a352b1fa MotionMaster::MoveJump / MoveSpline's two-point parabolic spline.
//! Gordoth retains its explicit-speed subset; effect254 also supplies source time,
//! speed-cap and height options. Facing target/transport/curves are not implemented.
//! Native jumping animation is unproved.
use bevy::prelude::*;
use shared::components::{Position, Rotation};

use super::{Hook, MotionType};
use crate::combat::DeathState;
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};

const MAX_JUMP_SPEED: f32 = 50.0;
const MAX_JUMP_HEIGHT: f32 = 1000.0;
const MIN_DURATION_MS: u32 = 1;

#[derive(Clone, Copy)]
pub(crate) struct JumpOptions {
    pub speed: f32,
    pub time_mode: bool,
    pub unlimited_speed: bool,
    pub min_height: f32,
    pub max_height: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct JumpMovement {
    id: u32,
    from: Vec3,
    to: Vec3,
    height: f32,
    duration_ms: u32,
    elapsed_ms: u32,
}

type Jumpers = (
    Entity,
    &'static mut JumpMovement,
    &'static mut Position,
    Option<&'static mut Rotation>,
);

pub(crate) fn move_jump(
    world: &mut World,
    unit: Entity,
    id: u32,
    target: Position,
    speed: f32,
    min_height: f32,
) {
    move_jump_with_options(
        world,
        unit,
        id,
        target,
        JumpOptions {
            speed,
            time_mode: false,
            unlimited_speed: false,
            min_height,
            max_height: MAX_JUMP_HEIGHT,
        },
    );
}

pub(crate) fn move_jump_with_options(
    world: &mut World,
    unit: Entity,
    id: u32,
    target: Position,
    options: JumpOptions,
) {
    let Some(from) = world.get::<Position>(unit).copied() else {
        warn!("MoveJump {unit}: missing source position");
        return;
    };
    let from = Vec3::new(from.x, from.y, from.z);
    let to = Vec3::new(target.x, target.y, target.z);
    let Some(jump) = calculate_jump(id, from, to, options) else {
        warn!("MoveJump {unit}: speed below source minimum");
        return;
    };
    world.entity_mut(unit).insert(jump);
}

/// Source speed cap/height calculation and CommonInitializer's integer duration,
/// whose linear segment starts at minimal_duration1.
fn calculate_jump(id: u32, from: Vec3, to: Vec3, options: JumpOptions) -> Option<JumpMovement> {
    let distance = from.distance(to);
    let requested_speed = if options.time_mode {
        // EffectJumpCharge converts FloatSeconds to integer Milliseconds first.
        let time_ms = (options.speed * 1000.0).trunc();
        if time_ms < 1.0 {
            return None;
        }
        distance / (time_ms / 1000.0)
    } else {
        options.speed
    };
    let speed = if options.unlimited_speed {
        requested_speed
    } else {
        requested_speed.min(MAX_JUMP_SPEED)
    };
    if speed < 0.01 {
        return None;
    }
    let seconds = distance / speed;
    let height = (shared::skyriding::GRAVITY * seconds * seconds / 8.0)
        .clamp(options.min_height, options.max_height);
    let duration_ms = MIN_DURATION_MS + (distance * (1000.0 / speed)) as u32;
    Some(JumpMovement {
        id,
        from,
        to,
        height,
        duration_ms,
        elapsed_ms: 0,
    })
}

/// Retain this tick's motion ownership through landing callbacks, then release at
/// the next unit tick. Lower movement drivers must not consume the same tick twice.
pub(crate) fn release_finished_jumps(
    mut commands: Commands,
    jumps: Query<(Entity, &JumpMovement, Has<DeathState>)>,
) {
    for (entity, jump, dead) in &jumps {
        if dead || jump.elapsed_ms == jump.duration_ms {
            commands.entity(entity).remove::<JumpMovement>();
        }
    }
}

fn spline_position(jump: &JumpMovement) -> Position {
    let t = jump.elapsed_ms as f32 / jump.duration_ms as f32;
    let mut position = jump.from.lerp(jump.to, t);
    position.y += jump.height * 4.0 * t * (1.0 - t);
    Position {
        x: position.x,
        y: position.y,
        z: position.z,
    }
}

pub(crate) fn advance_jumps(world: &mut World) {
    let dt_ms = world.resource::<Time>().delta().as_millis() as u32;
    for (entity, arrival) in update_jump_positions(world, dt_ms) {
        publish_jump_position(world, entity);
        if let Some(id) = arrival {
            super::dispatch(world, entity, Hook::MovementInform(MotionType::Effect, id));
        }
    }
}

fn update_jump_positions(world: &mut World, dt_ms: u32) -> Vec<(Entity, Option<u32>)> {
    let mut updates = Vec::new();
    let mut jumps = world.query_filtered::<Jumpers, (
        Without<DeathState>,
        Without<crate::spell_cast::charge::ChargeMovement>,
    )>();
    for (entity, mut jump, mut position, rotation) in jumps.iter_mut(world) {
        if jump.elapsed_ms == jump.duration_ms {
            continue;
        }
        step_jump(
            &mut jump,
            &mut position,
            rotation.map(|rotation| rotation.into_inner()),
            dt_ms,
        );
        let arrival = (jump.elapsed_ms == jump.duration_ms).then_some(jump.id);
        updates.push((entity, arrival));
    }
    updates
}

fn step_jump(
    jump: &mut JumpMovement,
    position: &mut Position,
    rotation: Option<&mut Rotation>,
    dt_ms: u32,
) {
    jump.elapsed_ms = (jump.elapsed_ms + dt_ms).min(jump.duration_ms);
    *position = spline_position(jump);
    let delta = jump.to - jump.from;
    if let (Some(rotation), Some(facing)) =
        (rotation, crate::creature_ai::facing_along(delta.x, delta.z))
    {
        rotation.y = facing;
    }
}

fn publish_jump_position(world: &World, entity: Entity) {
    mark_interest_position_changed(world.get_resource::<InterestDirtyQueue>(), entity);
    if let Some(queue) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        queue.mark(entity);
    }
}
