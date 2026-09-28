use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use shared::components::{MovementSpeed, Player, Position, Rotation, VerticalVelocity};
use shared::protocol::PlayerInput;

use crate::auras::AuraStatMods;

pub(super) const DEFAULT_RUN_SPEED: f32 = 7.0;
pub(super) const DEFAULT_WALK_SPEED: f32 = 2.5;
pub(super) const DEFAULT_SWIM_SPEED: f32 = shared::movement::SWIM_SPEED;

pub(super) type PlayerMovement = (
    &'static mut Position,
    &'static mut Rotation,
    &'static mut MovementSpeed,
    &'static mut VerticalVelocity,
    Option<&'static AuraStatMods>,
    Option<&'static shared::casting::CastState>,
);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct MovementChanges {
    pub(super) position_changed: bool,
    pub(super) replicated_state_changed: bool,
}

/// Longest movement one input may apply (a client frame of 4 fps).
pub(super) const MAX_INPUT_STEP_SECS: f32 = 0.25;
/// Movement time a player may bank ahead of server time, absorbing network jitter and
/// client frame hitches.
pub(super) const MOVEMENT_BANK_SECS: f32 = 0.5;

/// Seconds of movement each player may still apply. Banks refill at server time up to
/// `MOVEMENT_BANK_SECS`, so however many inputs arrive and however far they report, a
/// player moves no faster than its speed over real time. A player without an entry has
/// a full bank.
#[derive(Default)]
pub(crate) struct MovementClocks(HashMap<Entity, f32>);

impl MovementClocks {
    pub(super) fn advance(&mut self, delta_secs: f32) {
        self.0.retain(|_, bank| {
            *bank += delta_secs;
            *bank < MOVEMENT_BANK_SECS
        });
    }

    /// The movement time an input needing `requested_secs` may apply, taken from the bank.
    pub(super) fn grant(&mut self, player: Entity, requested_secs: f32) -> f32 {
        let bank = self.0.entry(player).or_insert(MOVEMENT_BANK_SECS);
        let step = requested_secs.max(0.0).min(MAX_INPUT_STEP_SECS).min(*bank);
        *bank -= step;
        step
    }
}

impl MovementChanges {
    pub(super) fn merge(self, other: Self) -> Self {
        Self {
            position_changed: self.position_changed || other.position_changed,
            replicated_state_changed: self.replicated_state_changed
                || other.replicated_state_changed,
        }
    }
}

/// Apply one movement input to the specified player entity: move it across the ground to
/// the position the client reports (`HandleMovementOpcode` → `UpdatePosition(pos)`), as far
/// as its speed covers in the movement time its bank grants. Height stays the server's
/// (gravity and the ground clamp).
pub(super) fn apply_movement_input(
    input: &PlayerInput,
    player_entity: Entity,
    players: &mut Query<PlayerMovement, With<Player>>,
    clocks: &mut MovementClocks,
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
    let mut changes = update_rotation_and_speed(input, auras, &mut rotation, &mut speed);
    let target = Vec2::new(input.position[0], input.position[2]);
    let current = Vec2::new(position.x, position.z);
    let next = move_toward_reported(current, target, speed.0, |secs| {
        clocks.grant(player_entity, secs)
    });
    if next != current {
        position.x = next.x;
        position.z = next.y;
        changes.position_changed = true;
        changes.replicated_state_changed = true;
    }
    changes
}

/// Where the player gets on the way from `current` to the client's `reported` point: all
/// the way when `speed` covers it in the time `grant` allows for it, else that far.
fn move_toward_reported(
    current: Vec2,
    reported: Vec2,
    speed: f32,
    grant: impl FnOnce(f32) -> f32,
) -> Vec2 {
    let distance = current.distance(reported);
    if !reported.is_finite() || distance < 0.001 || speed <= 0.0 {
        return current;
    }
    let travel = speed * grant(distance / speed);
    current + (reported - current) * (travel / distance).min(1.0)
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
    auras: Option<&AuraStatMods>,
    rotation: &mut Mut<'_, Rotation>,
    speed: &mut Mut<'_, MovementSpeed>,
) -> MovementChanges {
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
    MovementChanges {
        position_changed: false,
        replicated_state_changed: rotation_changed || speed_changed,
    }
}

/// Compute effective movement speed from input, auras, and direction.
pub(super) fn compute_movement_speed(
    input: &PlayerInput,
    auras: Option<&AuraStatMods>,
    direction_length: f32,
) -> f32 {
    let base = if input.swimming {
        DEFAULT_SWIM_SPEED
    } else if input.running {
        DEFAULT_RUN_SPEED
    } else {
        DEFAULT_WALK_SPEED
    };
    let aura_multiplier = auras.map_or(1.0, |mods| mods.movement_speed);
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

pub(super) fn direction_length(direction: &[f32; 3]) -> f32 {
    (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt()
}
