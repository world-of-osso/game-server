use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use shared::components::{MovementSpeed, Player, PlayerMotion, Position, Rotation};
use shared::protocol::PlayerInput;
use shared::skyriding::{SKYRIDING, impulse_bound, max_climb, speed_after};

use super::FallTracker;
use crate::auras::AuraStatMods;
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;

pub(super) const DEFAULT_RUN_SPEED: f32 = 7.0;
pub(super) const DEFAULT_WALK_SPEED: f32 = 2.5;
pub(super) const DEFAULT_SWIM_SPEED: f32 = shared::movement::SWIM_SPEED;
pub(super) const DEFAULT_FLIGHT_SPEED: f32 = shared::movement::FLIGHT_SPEED;

pub(super) type MovableUnit = Or<(With<Player>, With<shared::components::UnitCharm>)>;

pub(super) type PlayerMovement = (
    &'static mut Position,
    &'static mut Rotation,
    &'static mut MovementSpeed,
    Option<&'static AuraStatMods>,
    Option<&'static shared::casting::CastState>,
    &'static mut FallTracker,
    &'static mut PlayerMotion,
    Option<&'static shared::components::FeatherFall>,
);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct MovementChanges {
    pub(super) position_changed: bool,
    pub(super) replicated_state_changed: bool,
    /// The latest input's report is still ahead of the player.
    pub(super) behind: bool,
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
///
/// An input whose report its grant did not reach stays `unreached`: the player keeps
/// moving to it on later ticks as the bank refills, so a client sending fewer inputs than
/// the bank needs (a slow frame rate, a lone stop input) ends where it stopped.
///
/// A skyrider's `skyriding` speed is the most its energy allows (`shared::skyriding`). It
/// takes off with its mounted run speed and the `launches` its Skyriding abilities gave it
/// on the ground (the takeoff is Skyward Ascent's flap).
#[derive(Default)]
pub(crate) struct MovementClocks {
    banks: HashMap<Entity, f32>,
    unreached: HashMap<Entity, PlayerInput>,
    skyriding: HashMap<Entity, f32>,
    launches: HashMap<Entity, f32>,
}

impl MovementClocks {
    pub(super) fn advance(&mut self, delta_secs: f32) {
        self.banks.retain(|_, bank| {
            *bank += delta_secs;
            *bank < MOVEMENT_BANK_SECS
        });
    }

    /// The movement time an input needing `requested_secs` may apply, taken from the bank.
    pub(super) fn grant(&mut self, player: Entity, requested_secs: f32) -> f32 {
        let bank = self.banks.entry(player).or_insert(MOVEMENT_BANK_SECS);
        let step = requested_secs.max(0.0).min(MAX_INPUT_STEP_SECS).min(*bank);
        *bank -= step;
        step
    }

    /// The input `player` has yet to reach, unless a teleport (new `epoch`) voided it.
    pub(super) fn unreached(&self, player: Entity, epoch: u32) -> Option<PlayerInput> {
        self.unreached
            .get(&player)
            .filter(|input| input.epoch == epoch)
            .cloned()
    }

    /// Keep `latest` to move on to while `behind` its report, else forget it.
    pub(super) fn track(&mut self, player: Entity, latest: PlayerInput, behind: bool) {
        if behind {
            self.unreached.insert(player, latest);
        } else {
            self.unreached.remove(&player);
        }
    }

    /// A Skyriding ability flapped `player` (`crate::skyriding`): its energy may be
    /// `impulse_bound` faster. A player on the ground has none yet: the flap launches its
    /// takeoff.
    pub(super) fn add_impulse(&mut self, player: Entity, speed: f32) {
        match self.skyriding.get_mut(&player) {
            Some(energy) => *energy = impulse_bound(&SKYRIDING, *energy, speed),
            None => *self.launches.entry(player).or_default() += speed,
        }
    }

    /// Forget the players `exists` no longer finds.
    pub(super) fn retain_players(&mut self, exists: impl Fn(Entity) -> bool) {
        self.unreached.retain(|player, _| exists(*player));
        self.skyriding.retain(|player, _| exists(*player));
        self.launches.retain(|player, _| exists(*player));
    }
}

impl MovementChanges {
    pub(super) fn merge(self, other: Self) -> Self {
        Self {
            position_changed: self.position_changed || other.position_changed,
            replicated_state_changed: self.replicated_state_changed
                || other.replicated_state_changed,
            behind: other.behind,
        }
    }
}

/// Apply one movement input to the specified player entity: move it to the position the
/// client reports (`HandleMovementOpcode` → `UpdatePosition(pos)`), as far as its speed
/// covers in the movement time its bank grants, at a height `terrain` allows
/// (docs/specs/player-height.md). Without `terrain` the height stays the server's.
pub(super) fn apply_movement_input<F: bevy::ecs::query::QueryFilter>(
    input: &PlayerInput,
    player_entity: Entity,
    players: &mut Query<PlayerMovement, F>,
    clocks: &mut MovementClocks,
    mut terrain: Option<(&mut GroundMap, InMap)>,
) -> MovementChanges {
    let Ok((
        mut position,
        mut rotation,
        mut speed,
        auras,
        cast_state,
        mut fall_tracker,
        mut motion,
        feather_fall,
    )) = players.get_mut(player_entity)
    else {
        return MovementChanges::default();
    };
    if let Some(changes) = stop_for_blocking_cast(cast_state, &mut speed) {
        let stopped = stop_motion(&mut motion);
        // The report waits for the cast to end.
        return MovementChanges {
            behind: true,
            ..changes.merge(stopped)
        };
    }
    // MovementHandler.cpp:128: `MOVEMENTFLAG_FLYING` from a mover that cannot fly is dropped.
    let flying = input.flying && motion.contains(PlayerMotion::CAN_FLY);
    // An input's flags are its state at the position it reports (MovementHandler.cpp:567
    // `UpdatePosition(movementInfo.pos)`): a flyer flies to its landing report and lands
    // there, falling nothing (`Player::UpdateFallInformationIfNeed` keeps the fall height
    // at every flying report).
    let flown = flying || motion.contains(PlayerMotion::FLYING);
    let skyriding = flown && motion.contains(PlayerMotion::CAN_ADV_FLY);
    let current = Vec3::new(position.x, position.y, position.z);
    let reported = Vec3::from(input.position);
    let swimming = !flown
        && input.swimming
        && terrain.as_mut().is_some_and(|(ground, map_id)| {
            swim_bounds(ground, *map_id, reported.with_y(current.y)).is_some()
        });
    let travel = if flown {
        Travel::Flight
    } else if swimming {
        Travel::Swim(DEFAULT_SWIM_SPEED * auras.map_or(1.0, |mods| mods.swim_speed))
    } else {
        Travel::Ground
    };
    let energy = skyriding.then(|| match clocks.skyriding.get(&player_entity) {
        Some(&energy) => energy,
        None => {
            let launch = clocks.launches.remove(&player_entity).unwrap_or(0.0);
            (DEFAULT_RUN_SPEED * auras.map_or(1.0, |mods| mods.movement_speed)).hypot(launch)
        }
    });
    let leg = match energy {
        Some(energy) => skyriding_leg(current, reported, energy),
        None => Leg {
            target: reported,
            speed: compute_movement_speed(input, flown, auras, direction_length(&input.direction)),
            cut: false,
        },
    };
    let (mut next, behind) = move_toward_reported(current, leg.target, leg.speed, travel, |secs| {
        clocks.grant(player_entity, secs)
    });
    let unclamped = next;
    if let Some((ground, map)) = terrain.as_ref() {
        next = ground.clamp_live_movement(*map, current, next);
    }
    let behind = behind || leg.cut || next != unclamped;
    // Short of a landing report the player still flies.
    let state = PlayerInput {
        flying: if behind { flown } else { flying },
        ..input.clone()
    };
    let mut next_motion = input_motion(&state, *motion);
    if fall_tracker.airborne && !state.flying && !swimming {
        // An input changes intent, not the origin of an existing airborne interval.
        next_motion = airborne_motion(next_motion, &fall_tracker);
    }
    let mut changes =
        update_rotation_and_speed(&state, state.flying, auras, &mut rotation, &mut speed)
            .merge(set_motion(&mut motion, next_motion));
    changes.behind = behind;
    next.y = match terrain {
        Some((ground, map_id)) if flown => flight_height(ground, map_id, current, next),
        Some((ground, map_id)) => {
            fall_tracker.report_height(input.jumping);
            let player = JudgedPlayer {
                current,
                swimming,
                jumping: input.jumping,
                airborne: fall_tracker.airborne_state(),
                feather_fall: feather_fall.is_some_and(|state| state.0),
            };
            judged_height(ground, map_id, player, next)
        }
        None => current.y,
    };
    match energy {
        Some(energy) => {
            let left = speed_after(&SKYRIDING, energy, current.y - next.y).unwrap_or(0.0);
            clocks.skyriding.insert(player_entity, left);
        }
        None => {
            clocks.skyriding.remove(&player_entity);
        }
    }
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
    feather_fall: bool,
}

/// The height the player gets at `next`: within the swim bounds there while swimming,
/// else within the land bounds.
fn judged_height(ground: &mut GroundMap, map_id: InMap, player: JudgedPlayer, next: Vec3) -> f32 {
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
        feather_fall: player.feather_fall,
    };
    land_height(ground, map_id, player.current, next, envelope)
}

/// How an input's travel is timed.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Travel {
    /// Across the ground; the height follows the ground rules.
    Ground,
    /// Swimming: the height also moves at this vertical speed.
    Swim(f32),
    /// Flying: the whole 3D distance moves at the flight speed.
    Flight,
}

/// Where the player gets on the way from `current` to the client's `reported` point: all
/// the way when `speed` covers it (across the ground, plus the swim vertical speed, or in
/// 3D when flying) in the time `grant` allows for it, else that far along the way, still
/// behind the report. On the ground the height moves by the same fraction and is judged by
/// the ground rules after.
fn move_toward_reported(
    current: Vec3,
    reported: Vec3,
    speed: f32,
    travel: Travel,
    grant: impl FnOnce(f32) -> f32,
) -> (Vec3, bool) {
    if !reported.is_finite() || speed <= 0.0 {
        return (current, false);
    }
    let horizontal = current.xz().distance(reported.xz());
    let vertical = (reported.y - current.y).abs();
    let requested = match travel {
        Travel::Ground => horizontal / speed,
        Travel::Swim(vertical_speed) => (horizontal / speed).max(vertical / vertical_speed),
        Travel::Flight => current.distance(reported) / speed,
    };
    if requested < 0.001 / speed {
        return (current.with_y(reported.y), false);
    }
    let fraction = (grant(requested) / requested).min(1.0);
    (current + (reported - current) * fraction, fraction < 1.0)
}

/// Where and how fast one input's skyriding moves.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Leg {
    target: Vec3,
    speed: f32,
    /// The report climbs higher than the energy reaches: `target` stops short of it.
    cut: bool,
}

/// A skyrider with `energy` yards a second heading for `reported`: no higher than its
/// speed climbs (`max_climb`), at the most it moves at along the way (energy is conserved
/// at best, so a straight leg is fastest at one of its ends).
fn skyriding_leg(current: Vec3, reported: Vec3, energy: f32) -> Leg {
    let climb = reported.y - current.y;
    let reach = max_climb(energy);
    let (target, cut) = if climb > reach {
        (current + (reported - current) * (reach / climb), true)
    } else {
        (reported, false)
    };
    let end = speed_after(&SKYRIDING, energy, current.y - target.y).unwrap_or(0.0);
    Leg {
        target,
        speed: energy.max(end),
        cut,
    }
}

/// A flyer's height: anywhere on or over the ground under it (probed from the server's
/// height, so no report passes through a floor); unloaded ground takes the report.
fn flight_height(ground: &mut GroundMap, map_id: InMap, current: Vec3, next: Vec3) -> f32 {
    match ground.ground_in(map_id, next.with_y(current.y)) {
        GroundProbe::Supported(floor) => next.y.max(floor),
        GroundProbe::Unsupported | GroundProbe::Unloaded => next.y,
    }
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
pub(super) fn swim_bounds(ground: &mut GroundMap, map_id: InMap, feet: Vec3) -> Option<SwimBounds> {
    let liquid = ground.liquid_at(map_id.map_id, feet)?;
    let floor = match ground.ground_in(map_id, feet) {
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
    pub(super) feather_fall: bool,
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
    map_id: InMap,
    current: Vec3,
    next: Vec3,
    envelope: LandEnvelope,
) -> f32 {
    let floor = match ground.ground_in(map_id, next.with_y(current.y)) {
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
            airborne.takeoff_y
                + highest_arc(airborne.impulse, airborne.secs, envelope.feather_fall)
                + HEIGHT_TOLERANCE
        }
        (None, true) => {
            current_y
                + highest_arc(shared::movement::JUMP_IMPULSE, 0.0, envelope.feather_fall)
                + HEIGHT_TOLERANCE
        }
        (None, false) => (floor + super::GROUND_SNAP_THRESHOLD).max(current_y),
    }
}

/// Highest point, over the takeoff, of a launch at `impulse` under `GRAVITY` between
/// `secs - MOVEMENT_BANK_SECS` and `secs + MOVEMENT_BANK_SECS` after it.
fn highest_arc(impulse: f32, secs: f32, feather_fall: bool) -> f32 {
    let gravity = shared::movement::GRAVITY;
    let arc = |t: f32| {
        if feather_fall {
            shared::movement::feather_fall_displacement(impulse, t)
        } else {
            impulse * t - 0.5 * gravity * t * t
        }
    };
    let earliest = (secs - MOVEMENT_BANK_SECS).max(0.0);
    let latest = secs + MOVEMENT_BANK_SECS;
    arc((impulse / gravity).clamp(earliest, latest))
}

/// Retail `MovementFlags` of an input, as the client's `CMSG_MOVE_*` carries them: the
/// wire direction (its animation direction, `movement_to_direction`) against the facing
/// sets FORWARD/BACKWARD/STRAFE_LEFT/STRAFE_RIGHT, the run toggle WALKING, the swim
/// SWIMMING, a jump FALLING with the project JUMP_STARTED marker, and flight FLYING
/// (and ADV_FLYING for a skyrider). Gravity also sets FALLING for unjumped falls.
/// CAN_FLY and CAN_ADV_FLY are the server's (`Unit::SetCanFly`, `Unit::SetCanAdvFly`), kept
/// from `current`; a flyer does not swim (`SetCanFly` drops SWIMMING).
fn input_motion(input: &PlayerInput, current: PlayerMotion) -> PlayerMotion {
    let can_fly = current.contains(PlayerMotion::CAN_FLY);
    let can_adv_fly = current.contains(PlayerMotion::CAN_ADV_FLY);
    let flying = input.flying && can_fly;
    let length = direction_length(&input.direction);
    // Input replaces movement directions, not the server-owned aura409 ability.
    let mut flags = current.0 & crate::auras::handlers::CAN_TURN_WHILE_FALLING_MOTION;
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
        (input.swimming && !flying, PlayerMotion::SWIMMING),
        (
            input.jumping && !flying,
            PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED,
        ),
        (can_fly, PlayerMotion::CAN_FLY),
        (flying, PlayerMotion::FLYING),
        (can_adv_fly, PlayerMotion::CAN_ADV_FLY),
        (flying && can_adv_fly, PlayerMotion::ADV_FLYING),
    ] {
        if set {
            flags |= flag;
        }
    }
    PlayerMotion(flags)
}

/// The authoritative airborne interval. Retail's jump opcode identifies its origin;
/// FALLING_FAR describes falling phase, so it cannot stand in for jump origin.
/// The existing takeoff impulse preserves that origin until gravity ends the interval.
pub(super) fn airborne_motion(motion: PlayerMotion, tracker: &FallTracker) -> PlayerMotion {
    let mut flags = motion.0 & !(PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED);
    if tracker.airborne {
        flags |= PlayerMotion::FALLING;
        if tracker.jump_impulse > 0.0 {
            flags |= PlayerMotion::JUMP_STARTED;
        }
    }
    PlayerMotion(flags)
}

/// The flags without movement: what a stop input leaves (walk and swim mode).
fn stopped_motion(motion: PlayerMotion) -> PlayerMotion {
    const MOVING: u64 = PlayerMotion::FORWARD
        | PlayerMotion::BACKWARD
        | PlayerMotion::STRAFE_LEFT
        | PlayerMotion::STRAFE_RIGHT
        | PlayerMotion::FALLING
        | PlayerMotion::JUMP_STARTED;
    PlayerMotion(motion.0 & !MOVING)
}

/// Write `next` only when it differs, so unchanged flags do not replicate.
fn set_motion(motion: &mut Mut<'_, PlayerMotion>, next: PlayerMotion) -> MovementChanges {
    MovementChanges {
        position_changed: false,
        replicated_state_changed: motion.set_if_neq(next),
        behind: false,
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
        behind: false,
    })
}

fn update_rotation_and_speed(
    input: &PlayerInput,
    flying: bool,
    auras: Option<&AuraStatMods>,
    rotation: &mut Mut<'_, Rotation>,
    speed: &mut Mut<'_, MovementSpeed>,
) -> MovementChanges {
    let rotation_changed = rotation.y != input.facing_yaw;
    if rotation_changed {
        rotation.y = input.facing_yaw;
    }
    let direction_length = direction_length(&input.direction);
    let desired_speed = compute_movement_speed(input, flying, auras, direction_length);
    let speed_changed = speed.0 != desired_speed;
    if speed_changed {
        speed.0 = desired_speed;
    }
    MovementChanges {
        position_changed: false,
        replicated_state_changed: rotation_changed || speed_changed,
        behind: false,
    }
}

/// Compute effective movement speed from input, auras, and direction: flight speed × the
/// flight auras while `flying`, else swim, run or walk speed × the run auras.
pub(super) fn compute_movement_speed(
    input: &PlayerInput,
    flying: bool,
    auras: Option<&AuraStatMods>,
    direction_length: f32,
) -> f32 {
    let base = if flying {
        DEFAULT_FLIGHT_SPEED
    } else if input.swimming {
        DEFAULT_SWIM_SPEED
    } else if input.running {
        DEFAULT_RUN_SPEED
    } else {
        DEFAULT_WALK_SPEED
    };
    let aura_multiplier = auras.map_or(1.0, |mods| {
        if flying {
            mods.flight_speed
        } else {
            mods.movement_speed
        }
    });
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
