use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use shared::components::{MovementSpeed, Player, PlayerMotion, Position, Rotation};
use shared::protocol::PlayerInput;

use super::FallTracker;
use crate::auras::AuraStatMods;
use crate::ground::{GroundMap, GroundProbe};

pub(super) const DEFAULT_RUN_SPEED: f32 = 7.0;
pub(super) const DEFAULT_WALK_SPEED: f32 = 2.5;
pub(super) const DEFAULT_SWIM_SPEED: f32 = shared::movement::SWIM_SPEED;

pub(super) type PlayerMovement = (
    &'static mut Position,
    &'static mut Rotation,
    &'static mut MovementSpeed,
    Option<&'static AuraStatMods>,
    Option<&'static shared::casting::CastState>,
    &'static mut FallTracker,
    &'static mut PlayerMotion,
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

/// Apply one movement input to the specified player entity: move it to the position the
/// client reports (`HandleMovementOpcode` → `UpdatePosition(pos)`), as far as its speed
/// covers in the movement time its bank grants, at a height `terrain` allows
/// (docs/specs/player-height.md). Without `terrain` the height stays the server's.
pub(super) fn apply_movement_input(
    input: &PlayerInput,
    player_entity: Entity,
    players: &mut Query<PlayerMovement, With<Player>>,
    clocks: &mut MovementClocks,
    mut terrain: Option<(&mut GroundMap, u32)>,
) -> MovementChanges {
    let Ok((
        mut position,
        mut rotation,
        mut speed,
        auras,
        cast_state,
        mut fall_tracker,
        mut motion,
    )) = players.get_mut(player_entity)
    else {
        return MovementChanges::default();
    };
    if let Some(changes) = stop_for_blocking_cast(cast_state, &mut speed) {
        let stopped = stop_motion(&mut motion);
        return changes.merge(stopped);
    }
    let mut changes = update_rotation_and_speed(input, auras, &mut rotation, &mut speed)
        .merge(set_motion(&mut motion, input_motion(input)));
    let current = Vec3::new(position.x, position.y, position.z);
    let reported = Vec3::from(input.position);
    let swimming = input.swimming
        && terrain.as_mut().is_some_and(|(ground, map_id)| {
            swim_bounds(ground, *map_id, reported.with_y(current.y)).is_some()
        });
    let vertical_speed =
        swimming.then(|| DEFAULT_SWIM_SPEED * auras.map_or(1.0, |mods| mods.movement_speed));
    let mut next = move_toward_reported(current, reported, speed.0, vertical_speed, |secs| {
        clocks.grant(player_entity, secs)
    });
    next.y = match terrain {
        Some((ground, map_id)) => {
            fall_tracker.report_height(input.jumping);
            let player = JudgedPlayer {
                current,
                swimming,
                jumping: input.jumping,
                airborne: fall_tracker.airborne_state(),
            };
            judged_height(ground, map_id, player, next)
        }
        None => current.y,
    };
    if next != current {
        position.x = next.x;
        position.y = next.y;
        position.z = next.z;
        changes.position_changed = true;
        changes.replicated_state_changed = true;
    }
    changes
}

/// What the height rules know of the player an input moves.
#[derive(Clone, Copy, Debug)]
struct JudgedPlayer {
    current: Vec3,
    swimming: bool,
    jumping: bool,
    airborne: Option<Airborne>,
}

/// The height the player gets at `next`: within the swim bounds there while swimming,
/// else within the land bounds.
fn judged_height(ground: &mut GroundMap, map_id: u32, player: JudgedPlayer, next: Vec3) -> f32 {
    let bounds = player
        .swimming
        .then(|| swim_bounds(ground, map_id, next.with_y(player.current.y)))
        .flatten();
    if let Some(bounds) = bounds {
        return next.y.min(bounds.top).max(bounds.floor);
    }
    let envelope = LandEnvelope {
        airborne: player.airborne,
        jumping: player.jumping,
    };
    land_height(ground, map_id, player.current, next, envelope)
}

/// Where the player gets on the way from `current` to the client's `reported` point: all
/// the way when `speed` (across the ground) and `vertical_speed` (swimming) cover it in the
/// time `grant` allows for it, else that far along the way. Without a vertical speed, the
/// height moves by the same fraction and is judged by the ground rules after.
fn move_toward_reported(
    current: Vec3,
    reported: Vec3,
    speed: f32,
    vertical_speed: Option<f32>,
    grant: impl FnOnce(f32) -> f32,
) -> Vec3 {
    if !reported.is_finite() || speed <= 0.0 {
        return current;
    }
    let horizontal = current.xz().distance(reported.xz());
    let vertical = (reported.y - current.y).abs();
    let horizontal_secs = horizontal / speed;
    let vertical_secs = vertical_speed.map_or(0.0, |vertical_speed| vertical / vertical_speed);
    let requested = horizontal_secs.max(vertical_secs);
    if requested < 0.001 / speed {
        return current.with_y(reported.y);
    }
    let fraction = (grant(requested) / requested).min(1.0);
    current + (reported - current) * fraction
}

/// Heights a swimmer may take: on or over the ground, no higher than floating under the
/// surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SwimBounds {
    pub(super) floor: f32,
    pub(super) top: f32,
}

/// The swim bounds at `feet`, where the terrain water there stands `SWIM_DEPTH` over the
/// ground; `None` where the player cannot swim.
pub(super) fn swim_bounds(ground: &mut GroundMap, map_id: u32, feet: Vec3) -> Option<SwimBounds> {
    let liquid = ground.liquid_at(map_id, feet)?;
    let floor = match ground.ground_at(map_id, feet) {
        GroundProbe::Supported(height) => height,
        _ => liquid.depth_level,
    };
    let top = shared::movement::swim_top(liquid.level);
    shared::movement::is_swimming(floor, floor, Some(liquid.level))
        .then_some(SwimBounds { floor, top })
}

/// What bounds a height on land: how long the player has been off the ground and from
/// where, and whether the input jumps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct LandEnvelope {
    pub(super) airborne: Option<Airborne>,
    pub(super) jumping: bool,
}

/// A player off the ground since `takeoff_y`, `secs` ago, with `impulse` upward speed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Airborne {
    pub(super) takeoff_y: f32,
    pub(super) impulse: f32,
    pub(super) secs: f32,
}

/// Slack over the highest height a jump or fall allows.
const HEIGHT_TOLERANCE: f32 = 0.25;

/// `next.y` held to the ground under `next` (probed from the server's height, so no report
/// passes through a floor) and to the highest height the envelope allows. Unloaded ground
/// keeps the server's height.
fn land_height(
    ground: &mut GroundMap,
    map_id: u32,
    current: Vec3,
    next: Vec3,
    envelope: LandEnvelope,
) -> f32 {
    let floor = match ground.ground_at(map_id, next.with_y(current.y)) {
        GroundProbe::Supported(height) => height,
        GroundProbe::Unsupported => f32::NEG_INFINITY,
        GroundProbe::Unloaded => return current.y,
    };
    let ceiling = land_ceiling(current.y, floor, envelope);
    next.y.min(ceiling).max(floor)
}

/// The highest height on land: without a jump the ground or where the player already is
/// (no rise off the ground); a jump's or fall's arc from where it took off, over the
/// movement bank's window of input jitter.
pub(super) fn land_ceiling(current_y: f32, floor: f32, envelope: LandEnvelope) -> f32 {
    match (envelope.airborne, envelope.jumping) {
        (Some(airborne), _) => {
            airborne.takeoff_y + highest_arc(airborne.impulse, airborne.secs) + HEIGHT_TOLERANCE
        }
        (None, true) => {
            current_y + highest_arc(shared::movement::JUMP_IMPULSE, 0.0) + HEIGHT_TOLERANCE
        }
        (None, false) => (floor + super::GROUND_SNAP_THRESHOLD).max(current_y),
    }
}

/// Highest point, over the takeoff, of a launch at `impulse` under `GRAVITY` between
/// `secs - MOVEMENT_BANK_SECS` and `secs + MOVEMENT_BANK_SECS` after it.
fn highest_arc(impulse: f32, secs: f32) -> f32 {
    let gravity = shared::movement::GRAVITY;
    let arc = |t: f32| impulse * t - 0.5 * gravity * t * t;
    let earliest = (secs - MOVEMENT_BANK_SECS).max(0.0);
    let latest = secs + MOVEMENT_BANK_SECS;
    arc((impulse / gravity).clamp(earliest, latest))
}

/// Retail `MovementFlags` of an input, as the client's `CMSG_MOVE_*` carries them: the
/// wire direction (its animation direction, `movement_to_direction`) against the facing
/// sets FORWARD/BACKWARD/STRAFE_LEFT/STRAFE_RIGHT, the run toggle WALKING, the swim
/// SWIMMING and a jump in the air FALLING.
fn input_motion(input: &PlayerInput) -> PlayerMotion {
    let length = direction_length(&input.direction);
    let mut flags = 0;
    if length > 0.001 {
        let forward = facing_alignment(input.facing_yaw, &input.direction, length);
        let right = right_alignment(input.facing_yaw, &input.direction, length);
        for (alignment, positive, negative) in [
            (forward, PlayerMotion::FORWARD, PlayerMotion::BACKWARD),
            (right, PlayerMotion::STRAFE_RIGHT, PlayerMotion::STRAFE_LEFT),
        ] {
            if alignment > 0.5 {
                flags |= positive;
            } else if alignment < -0.5 {
                flags |= negative;
            }
        }
    }
    for (set, flag) in [
        (!input.running, PlayerMotion::WALKING),
        (input.swimming, PlayerMotion::SWIMMING),
        (input.jumping, PlayerMotion::FALLING),
    ] {
        if set {
            flags |= flag;
        }
    }
    PlayerMotion(flags)
}

/// The flags without movement: what a stop input leaves (walk and swim mode).
fn stopped_motion(motion: PlayerMotion) -> PlayerMotion {
    const MOVING: u32 = PlayerMotion::FORWARD
        | PlayerMotion::BACKWARD
        | PlayerMotion::STRAFE_LEFT
        | PlayerMotion::STRAFE_RIGHT
        | PlayerMotion::FALLING;
    PlayerMotion(motion.0 & !MOVING)
}

/// Write `next` only when it differs, so unchanged flags do not replicate.
fn set_motion(motion: &mut Mut<'_, PlayerMotion>, next: PlayerMotion) -> MovementChanges {
    MovementChanges {
        position_changed: false,
        replicated_state_changed: motion.set_if_neq(next),
    }
}

/// Stop a player the server holds still (a blocking cast, a corpse).
pub(super) fn stop_motion(motion: &mut Mut<'_, PlayerMotion>) -> MovementChanges {
    let stopped = stopped_motion(**motion);
    set_motion(motion, stopped)
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

/// How far `direction` points to the facing's right, `(-forward.z, forward.x)` (the
/// client's strafe right).
fn right_alignment(facing_yaw: f32, direction: &[f32; 3], direction_length: f32) -> f32 {
    (-direction[0] * facing_yaw.cos() + direction[2] * facing_yaw.sin()) / direction_length
}

pub(super) fn direction_length(direction: &[f32; 3]) -> f32 {
    (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt()
}
