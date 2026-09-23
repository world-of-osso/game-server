use bevy::prelude::*;
use shared::aura::Auras;
use shared::components::{MovementSpeed, Player, Position, Rotation, VerticalVelocity};
use shared::protocol::PlayerInput;

pub(super) const DEFAULT_RUN_SPEED: f32 = 7.0;
pub(super) const DEFAULT_WALK_SPEED: f32 = 2.5;
pub(super) const DEFAULT_SWIM_SPEED: f32 = shared::movement::SWIM_SPEED;

pub(super) type PlayerMovement = (
    &'static mut Position,
    &'static mut Rotation,
    &'static mut MovementSpeed,
    &'static mut VerticalVelocity,
    Option<&'static Auras>,
    Option<&'static shared::casting::CastState>,
);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct MovementChanges {
    pub(super) position_changed: bool,
    pub(super) replicated_state_changed: bool,
}

/// Apply a single movement input to the specified player entity.
pub(super) fn apply_movement_input(
    input: &PlayerInput,
    player_entity: Entity,
    players: &mut Query<PlayerMovement, With<Player>>,
    time: &Time,
) -> MovementChanges {
    let Ok((mut position, mut rotation, mut speed, mut vertical_velocity, auras, cast_state)) =
        players.get_mut(player_entity)
    else {
        return MovementChanges::default();
    };
    if let Some(changes) = stop_for_blocking_cast(cast_state, &mut speed) {
        return changes;
    }
    if input.jumping && vertical_velocity.0 == 0.0 {
        vertical_velocity.0 = shared::movement::JUMP_IMPULSE;
    }
    let (direction_length, mut changes) =
        update_rotation_and_speed(input, auras, &mut rotation, &mut speed);
    if apply_input_translation(
        input,
        &mut position,
        speed.0,
        direction_length,
        time.delta_secs(),
    ) {
        changes.position_changed = true;
        changes.replicated_state_changed = true;
    }
    changes
}

fn stop_for_blocking_cast(
    cast_state: Option<&shared::casting::CastState>,
    speed: &mut Mut<'_, MovementSpeed>,
) -> Option<MovementChanges> {
    let blocks_movement = cast_state.is_some_and(|cast| !cast.cast_while_moving);
    if !blocks_movement {
        return None;
    }
    let speed_changed = speed.0 != 0.0;
    if speed_changed {
        speed.0 = 0.0;
    }
    Some(MovementChanges {
        position_changed: false,
        replicated_state_changed: speed_changed,
    })
}

fn update_rotation_and_speed(
    input: &PlayerInput,
    auras: Option<&Auras>,
    rotation: &mut Mut<'_, Rotation>,
    speed: &mut Mut<'_, MovementSpeed>,
) -> (f32, MovementChanges) {
    let rotation_changed = rotation.y != input.facing_yaw;
    if rotation_changed {
        rotation.y = input.facing_yaw;
    }
    let direction_length = direction_length(&input.direction);
    let desired_speed = compute_movement_speed(input, auras, direction_length);
    let speed_changed = speed.0 != desired_speed;
    if speed_changed {
        speed.0 = desired_speed;
    }
    (
        direction_length,
        MovementChanges {
            position_changed: false,
            replicated_state_changed: rotation_changed || speed_changed,
        },
    )
}

fn apply_input_translation(
    input: &PlayerInput,
    position: &mut Mut<'_, Position>,
    speed: f32,
    direction_length: f32,
    delta_seconds: f32,
) -> bool {
    if direction_length < 0.001 {
        return false;
    }
    let scale = speed * delta_seconds / direction_length;
    if !validate_move_distance(scale * direction_length, speed, delta_seconds) {
        return false;
    }
    let next_position = Position {
        x: position.x + input.direction[0] * scale,
        y: position.y + input.direction[1] * scale,
        z: position.z + input.direction[2] * scale,
    };
    if **position == next_position {
        return false;
    }
    **position = next_position;
    true
}

/// Compute effective movement speed from input, auras, and direction.
pub(super) fn compute_movement_speed(
    input: &PlayerInput,
    auras: Option<&Auras>,
    direction_length: f32,
) -> f32 {
    let base = if input.swimming {
        DEFAULT_SWIM_SPEED
    } else if input.running {
        DEFAULT_RUN_SPEED
    } else {
        DEFAULT_WALK_SPEED
    };
    let aura_multiplier = auras.map_or(1.0, Auras::movement_speed_multiplier);
    let direction_multiplier =
        movement_speed_multiplier(input.facing_yaw, &input.direction, direction_length);
    base * aura_multiplier * direction_multiplier
}

pub(super) fn movement_speed_multiplier(
    facing_yaw: f32,
    direction: &[f32; 3],
    direction_length: f32,
) -> f32 {
    if direction_length <= 0.001 {
        return 1.0;
    }
    let alignment = facing_alignment(facing_yaw, direction, direction_length);
    if alignment < -0.5 {
        shared::movement::BACKPEDAL_MULTIPLIER
    } else if alignment.abs() < 0.5 {
        shared::movement::STRAFE_MULTIPLIER
    } else {
        1.0
    }
}

fn facing_alignment(facing_yaw: f32, direction: &[f32; 3], direction_length: f32) -> f32 {
    let forward_x = facing_yaw.sin();
    let forward_z = facing_yaw.cos();
    (direction[0] * forward_x + direction[2] * forward_z) / direction_length
}

/// Reject movement if distance exceeds max speed with tolerance.
pub(super) fn validate_move_distance(distance: f32, max_speed: f32, delta_seconds: f32) -> bool {
    let max_distance = max_speed * delta_seconds * 1.5;
    distance <= max_distance
}

pub(super) fn direction_length(direction: &[f32; 3]) -> f32 {
    (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt()
}
