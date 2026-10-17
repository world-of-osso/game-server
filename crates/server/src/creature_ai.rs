use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{CreatureMotion, Health, MovementSpeed, Position, Rotation};

use crate::auras::{AuraStatMods, Confused, Feared};
use crate::combat::DeathState;
use crate::grid_activity::GridDormant;
use crate::ground::{GroundMap, GroundProbe};
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};
use crate::maps::InMap;
use crate::maps::store::{InstanceType, MapStore};
use crate::navigation::{Goal, MoveRequest, Navigation, Step};
use crate::terrain_height::{bevy_to_wow, wow_to_bevy};
use crate::world_data::{WaypointMoveType, WaypointPath, WaypointPoint, WorldData};
use shared::casting::CastState;
use shared::ground::STEP_UP_HEIGHT;
use std::time::Duration;

/// Walk speed in world-units per second (base WoW walk speed).
const BASE_WALK_SPEED: f32 = 2.5;
/// Run speed in world-units per second (AzerothCore `baseMoveSpeed[MOVE_RUN]`).
pub(crate) const BASE_RUN_SPEED: f32 = 7.0;
/// Maximum chase distance from spawn before evading (yards).
const LEASH_DISTANCE: f32 = 40.0;
/// Horizontal distance a chaser closes to (yards), inside the 5 yard
/// melee range (`ATTACK_DISTANCE`) so auto-attacks land.
pub(crate) const CHASE_REACH: f32 = 2.0;
/// TrinityCore a352b1fa `CREATURE_NOPATH_EVADE_TIME` (CreatureData.h).
const NO_PATH_EVADE_TIME: Duration = Duration::from_secs(5);

/// `Creature::SetCannotReachTarget`: continuous failed reachability for one chase.
#[derive(Component)]
pub(crate) struct CannotReachTarget {
    target: u64,
    elapsed: Duration,
}
/// Idle pause range in seconds after reaching a wander destination.
const IDLE_MIN_SECS: f32 = 2.0;
const IDLE_MAX_SECS: f32 = 5.0;
/// Delay before a wanderer retries a destination it cannot walk to
/// (`RandomMovementGenerator` `_timer.Reset(100)`).
const RETRY_WANDER_SECS: f32 = 0.1;
/// `ConfusedMovementGenerator::SetTargetLocation`: a point up to this far either way
/// from where the confusion began, then a pause of `urand(800, 1500)` ms.
const CONFUSED_REACH: f32 = 2.0;
const CONFUSED_PAUSE_SECS: (f32, f32) = (0.8, 1.5);
/// Delay before a confused unit retries a point it cannot walk to (`_timer.Reset(100)`).
const RETRY_CONFUSED_SECS: f32 = 0.1;

/// Living, active creatures that are not confused or feared: their wander, waypoint
/// and formation movement runs.
type Roaming = (
    Without<crate::knockback::KnockbackFlight>,
    Without<DeathState>,
    Without<GridDormant>,
    Without<Confused>,
    Without<Feared>,
    Without<crate::scripted_ai::movement::PointMovement>,
    Without<crate::scripted_ai::movement::ScriptedPath>,
    Without<crate::scripted_ai::waypoint_path::PathMovement>,
    Without<crate::scripted_ai::jump_movement::JumpMovement>,
    Without<crate::spell_cast::charge::ChargeMovement>,
);
/// Active creatures that are not confused, feared or in flight: chase and leash run.
pub(crate) type ActiveUncontrolled = (
    Without<GridDormant>,
    Without<Confused>,
    Without<Feared>,
    Without<crate::knockback::KnockbackFlight>,
    Without<crate::scripted_ai::jump_movement::JumpMovement>,
    Without<crate::spell_cast::charge::ChargeMovement>,
);

pub struct CreatureAiPlugin;

impl Plugin for CreatureAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapStore>();
        app.add_systems(
            FixedUpdate,
            (
                wander_system,
                (waypoint_system, formation_system).chain(),
                (
                    chase_system,
                    no_path_evade_system,
                    leash_check_system,
                    return_to_spawn_system,
                )
                    .chain(),
                confused_movement_system,
                follow_owner_system,
                pet_motion_system,
            ),
        );
    }
}

/// Original spawn location and wander radius.
#[derive(Component, Debug, Clone)]
pub struct SpawnPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub wander_distance: f32,
    pub speed_walk: f32,
    /// `creature_template.speed_run`: chase and evade-home speed multiplier.
    pub speed_run: f32,
}

/// The creature's original home facing in Bevy radians, independent of current rotation.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct HomeOrientation(pub f32);

/// Explicit TC `Creature::m_homePosition`, independent of original respawn data.
/// Position and orientation use server/Bevy coordinates.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct HomePosition {
    pub position: Position,
    pub orientation: f32,
}

/// State machine for random-wander creatures (movement_type=1).
/// Coordinates are in Bevy space (X-right, Y-up, Z-back).
#[derive(Component, Debug)]
pub struct WanderState {
    pub phase: WanderPhase,
    pub target_x: f32,
    pub target_z: f32,
    pub idle_remaining: f32,
    /// LCG state, advanced on each random pick.
    pub seed: u32,
}

#[derive(Debug, PartialEq)]
pub enum WanderPhase {
    Idle,
    Walking,
}

/// State for waypoint-pathing creatures (movement_type=2).
#[derive(Component, Debug)]
pub struct WaypointState {
    /// `waypoint_data.id` from the spawn's creature addon.
    pub path_id: u32,
    /// Point walked toward, or reached and waiting out its delay.
    pub current_index: usize,
    /// `current_index` was reached; the next leg starts when the delay ends.
    pub arrived: bool,
    pub delay_remaining: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    /// Bevy (x, z) the current leg started from.
    pub leg_origin: (f32, f32),
    /// Distance covered in the last tick (0 while paused).
    pub moved: f32,
}

impl WaypointState {
    pub fn new(path_id: u32, speed_walk: f32, speed_run: f32, origin: (f32, f32)) -> Self {
        Self {
            path_id,
            current_index: 0,
            arrived: false,
            delay_remaining: 0.0,
            speed_walk,
            speed_run,
            leg_origin: origin,
            moved: 0.0,
        }
    }

    /// Walk the path again from its first point, leaving from `origin`.
    pub fn restart(&mut self, origin: (f32, f32)) {
        *self = Self::new(self.path_id, self.speed_walk, self.speed_run, origin);
    }

    fn speed(&self, move_type: WaypointMoveType) -> f32 {
        match move_type {
            WaypointMoveType::Walk => self.speed_walk * BASE_WALK_SPEED,
            WaypointMoveType::Run | WaypointMoveType::Land | WaypointMoveType::Takeoff => {
                self.speed_run * BASE_RUN_SPEED
            }
        }
    }
}

/// `creature_formations` member that moves with its patrolling leader.
#[derive(Component, Debug)]
pub struct FormationMember {
    pub leader: Entity,
    pub follow_dist: f32,
    /// Radians, relative to the leader's direction of travel.
    pub follow_angle: f32,
}

/// `FollowMovementGenerator` toward `owner` at `PET_FOLLOW_DIST` and `PET_FOLLOW_ANGLE`
/// (`Unit::GetMotionMaster()->MoveFollow` of a pet, PetDefines.h:98-99).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct FollowOwner {
    pub owner: Entity,
    /// `UNIT_STATE_FOLLOW_MOVE`: on its way to the follow point.
    moving: bool,
}

impl FollowOwner {
    pub fn new(owner: Entity) -> Self {
        Self {
            owner,
            moving: false,
        }
    }
}

/// High-level creature behavior state. Overrides wander/waypoint when non-idle.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub enum CreatureState {
    /// Normal behavior — wander or waypoint patrol runs as usual.
    #[default]
    Idle,
    /// Chasing a threat target. Wander/waypoint AI pauses.
    /// `target` is the u64 entity ID from the ThreatTable.
    Pursuing { target: u64 },
    /// Returning to spawn point after evade. Wander/waypoint AI pauses.
    Returning,
}

impl CreatureState {
    /// Whether normal wander/waypoint AI should run.
    pub fn is_idle(&self) -> bool {
        *self == Self::Idle
    }
}

// -- Wander logic --

fn wander_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut query: Query<
        (
            Entity,
            &mut Position,
            &SpawnPoint,
            &mut WanderState,
            Option<&CreatureState>,
            Option<&InMap>,
            Bearing,
        ),
        Roaming,
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, mut state, creature_state, map, bearing) in &mut query {
        if matches!(creature_state, Some(s) if !s.is_idle()) {
            continue;
        }
        // RandomMovementGenerator rejects non-walkable paths. Unlike a routed
        // chase, this direct wander step must check walkable support itself.
        let mut probe = |feet: Vec3| match (ground.as_deref_mut(), map) {
            (Some(ground), Some(map)) => ground.ground_in(*map, feet),
            _ => GroundProbe::Unloaded,
        };
        let next_position = match state.phase {
            WanderPhase::Idle => {
                tick_idle(&mut state, dt, spawn, *pos, &mut probe);
                *pos
            }
            WanderPhase::Walking => tick_walk(*pos, &mut state, spawn, dt, &mut probe),
        };
        let speed = spawn.speed_walk * BASE_WALK_SPEED;
        let stride = Stride::moving(*pos, next_position, CreatureMotion::Walk, speed);
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

/// The ground under `feet` on the creature's map; `Unloaded` without ground data.
fn probe_ground(ground: Option<&mut GroundMap>, map: Option<&InMap>, feet: Vec3) -> GroundProbe {
    match (ground, map) {
        (Some(ground), Some(map)) => ground.creature_ground_in(*map, feet),
        _ => GroundProbe::Unloaded,
    }
}

/// Height a chasing or homing creature runs at from height `from_y`: the floor under it
/// (`UpdateAllowedPositionZ`), else its height where no floor is known. Unlike a wanderer
/// it may drop off a ledge: it has to reach its target or home.
fn run_ground(probe: GroundProbe, from_y: f32) -> f32 {
    match probe {
        GroundProbe::Supported(floor) => floor,
        GroundProbe::Unloaded | GroundProbe::Unsupported => from_y,
    }
}

/// Exact runtime support for a routed foot position (Recast contours approximate it).
fn sample_movement_ground(
    ground: Option<&mut GroundMap>,
    map: Option<&InMap>,
    mut position: Position,
) -> Position {
    let feet = Vec3::new(position.x, position.y, position.z);
    position.y = run_ground(probe_ground(ground, map, feet), position.y);
    position
}

/// `Rotation.y` of a creature facing along Bevy `(dx, dz)`: WoW orientation + π/2, as spawns
/// are placed (`creature_bundle`). `None` without a direction.
pub(crate) fn facing_along(dx: f32, dz: f32) -> Option<f32> {
    (dx.hypot(dz) > 1e-4).then(|| (-dz).atan2(dx) + std::f32::consts::FRAC_PI_2)
}

/// What a movement system did to a creature this tick.
struct Stride {
    position: Position,
    motion: CreatureMotion,
    /// `Rotation.y` to face, `None` to keep the facing.
    facing: Option<f32>,
    /// Yards per second of `motion`'s gait (walk or run speed after auras).
    speed: f32,
}

impl Stride {
    fn still(at: Position) -> Self {
        Self {
            position: at,
            motion: CreatureMotion::Still,
            facing: None,
            speed: 0.0,
        }
    }

    /// From `from` to `to` in `motion` at `speed` (move spline orientation: facing the
    /// way it goes), `Still` when it did not move.
    fn moving(from: Position, to: Position, motion: CreatureMotion, speed: f32) -> Self {
        let facing = facing_along(to.x - from.x, to.z - from.z);
        Self {
            position: to,
            motion: if facing.is_some() {
                motion
            } else {
                CreatureMotion::Still
            },
            facing,
            speed,
        }
    }
}

/// A creature's facing, motion and the speed it moves at, replicated to clients
/// (TrinityCore sends `SMSG_MOVE_SET_WALK_SPEED`/`RUN_SPEED` to the unit's viewers; the
/// client paces the walk and run clips by it).
type Bearing = (
    Option<&'static mut Rotation>,
    Option<&'static mut CreatureMotion>,
    Option<&'static mut MovementSpeed>,
);

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct MoveQueues<'w> {
    interest: Option<Res<'w, InterestDirtyQueue>>,
    replication: Option<Res<'w, ReplicationDirtyQueue>>,
}

impl MoveQueues<'_> {
    /// Write `stride` back: the position (interest), the facing and motion (replication)
    /// when they change.
    fn apply(
        &self,
        entity: Entity,
        position: &mut Mut<Position>,
        (rotation, motion, speed): (
            Option<Mut<Rotation>>,
            Option<Mut<CreatureMotion>>,
            Option<Mut<MovementSpeed>>,
        ),
        stride: Stride,
    ) {
        if **position != stride.position {
            **position = stride.position;
            mark_interest_position_changed(self.interest.as_deref(), entity);
        }
        let mut changed = false;
        if let (Some(mut rotation), Some(facing)) = (rotation, stride.facing)
            && rotation.y != facing
        {
            rotation.y = facing;
            changed = true;
        }
        if let Some(mut motion) = motion {
            changed |= motion.set_if_neq(stride.motion);
        }
        if let Some(mut speed) = speed
            && stride.motion != CreatureMotion::Still
        {
            changed |= speed.set_if_neq(MovementSpeed(stride.speed));
        }
        if changed && let Some(replication) = self.replication.as_deref() {
            replication.mark(entity);
        }
    }
}

/// Height a wanderer walking from height `from_y` stands at over a column the ground
/// `probe` judged: the floor (`UpdateAllowedPositionZ`), or `None` where the walk may not
/// go, with no support or a drop deeper than a step, a ledge `PathGenerator` does not
/// route over. Without ground data (`Unloaded`) the height is kept.
fn wander_ground(probe: GroundProbe, from_y: f32) -> Option<f32> {
    match probe {
        GroundProbe::Unloaded => Some(from_y),
        GroundProbe::Unsupported => None,
        GroundProbe::Supported(floor) if from_y - floor > STEP_UP_HEIGHT => None,
        GroundProbe::Supported(floor) => Some(floor),
    }
}

/// `RandomMovementGenerator::SetRandomLocation`: a destination without ground is
/// retried 100 ms later with a new random point.
fn tick_idle(
    state: &mut WanderState,
    dt: f32,
    spawn: &SpawnPoint,
    position: Position,
    probe: &mut impl FnMut(Vec3) -> GroundProbe,
) {
    state.idle_remaining -= dt;
    if state.idle_remaining > 0.0 {
        return;
    }
    // Wander in Bevy horizontal plane (x, z).
    let (tx, tz) = pick_wander_point(spawn.x, spawn.z, spawn.wander_distance, state.seed);
    state.seed = next_seed(state.seed);
    if wander_ground(probe(Vec3::new(tx, position.y, tz)), position.y).is_none() {
        state.idle_remaining = RETRY_WANDER_SECS;
        return;
    }
    state.target_x = tx;
    state.target_z = tz;
    state.phase = WanderPhase::Walking;
}

fn tick_walk(
    mut position: Position,
    state: &mut WanderState,
    spawn: &SpawnPoint,
    dt: f32,
    probe: &mut impl FnMut(Vec3) -> GroundProbe,
) -> Position {
    let speed = spawn.speed_walk * BASE_WALK_SPEED;
    let (reached, new_x, new_z) = move_toward(
        position.x,
        position.z,
        state.target_x,
        state.target_z,
        speed * dt,
    );
    let next_y = wander_ground(probe(Vec3::new(new_x, position.y, new_z)), position.y);
    if let Some(y) = next_y {
        position.x = new_x;
        position.y = y;
        position.z = new_z;
    }
    if reached || next_y.is_none() {
        state.phase = WanderPhase::Idle;
        state.idle_remaining = random_idle_secs(state.seed);
        state.seed = next_seed(state.seed);
    }
    position
}

/// Pick a random point within `radius` of (cx, cy) using a simple LCG.
pub fn pick_wander_point(cx: f32, cy: f32, radius: f32, seed: u32) -> (f32, f32) {
    let angle = lcg_float(seed) * std::f32::consts::TAU;
    let dist = lcg_float(seed.wrapping_add(7)) * radius;
    (cx + dist * angle.cos(), cy + dist * angle.sin())
}

/// Generate a pseudo-random idle duration in [IDLE_MIN_SECS, IDLE_MAX_SECS].
fn random_idle_secs(seed: u32) -> f32 {
    IDLE_MIN_SECS + lcg_float(seed.wrapping_add(13)) * (IDLE_MAX_SECS - IDLE_MIN_SECS)
}

/// Simple LCG producing a float in [0, 1).
fn lcg_float(seed: u32) -> f32 {
    (next_seed(seed) >> 8) as f32 / 16777216.0 // 2^24
}

/// The LCG's next state. Seeds one apart give nearly the same float, so a wanderer
/// advancing its seed by one picked the same point every time.
fn next_seed(seed: u32) -> u32 {
    seed.wrapping_mul(1664525).wrapping_add(1013904223)
}

/// Move (x, y) toward (tx, ty) by at most `step`. Returns (reached, new_x, new_y).
pub fn move_toward(x: f32, y: f32, tx: f32, ty: f32, step: f32) -> (bool, f32, f32) {
    let dx = tx - x;
    let dy = ty - y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= step || dist < 0.01 {
        return (true, tx, ty);
    }
    let ratio = step / dist;
    (false, x + dx * ratio, y + dy * ratio)
}

// -- Waypoint logic --

fn waypoint_system(
    time: Res<Time>,
    world_data: Res<WorldData>,
    queues: MoveQueues,
    mut nav: Option<ResMut<Navigation>>,
    mut ground: Option<ResMut<GroundMap>>,
    mut query: Query<
        (
            Entity,
            &mut Position,
            &mut WaypointState,
            Option<&CreatureState>,
            Option<&InMap>,
            Bearing,
        ),
        Roaming,
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, mut state, creature_state, map, bearing) in &mut query {
        state.moved = 0.0;
        if matches!(creature_state, Some(s) if !s.is_idle()) {
            continue;
        }
        let Some(path) = world_data.waypoints.get(&state.path_id) else {
            continue;
        };
        if path.points.is_empty() {
            continue;
        }
        let (next_position, orientation) = match (nav.as_deref_mut(), map) {
            (Some(nav), Some(map)) => {
                let path_id = state.path_id;
                tick_waypoint_with_travel(
                    *pos,
                    &mut state,
                    path,
                    dt,
                    |from, to, distance, index| {
                        let request = MoveRequest {
                            map_id: map.map_id,
                            goal: Goal::Waypoint(path_id, index),
                            from,
                            to,
                            distance,
                            reach: 0.0,
                        };
                        match nav.advance(entity, request) {
                            Step::Unbaked => direct_leg(from, to, distance),
                            Step::Moved { position, reached } => (
                                sample_movement_ground(ground.as_deref_mut(), Some(map), position),
                                reached,
                            ),
                            Step::Blocked => (from, false),
                        }
                    },
                )
            }
            _ => tick_waypoint(*pos, &mut state, path, dt),
        };
        let leg = &path.points[state.current_index % path.points.len()];
        let speed = state.speed(leg.move_type);
        let mut stride = Stride::moving(*pos, next_position, leg_motion(leg.move_type), speed);
        if let Some(orientation) = orientation {
            stride.facing = Some(orientation + std::f32::consts::FRAC_PI_2);
        }
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

/// `waypoint_data.move_type` 0 walks; run, land and takeoff run (`WaypointState::speed`).
fn leg_motion(move_type: WaypointMoveType) -> CreatureMotion {
    match move_type {
        WaypointMoveType::Walk => CreatureMotion::Walk,
        WaypointMoveType::Run | WaypointMoveType::Land | WaypointMoveType::Takeoff => {
            CreatureMotion::Run
        }
    }
}

/// Advance one tick along the path (WaypointMovementGenerator::StartMove /
/// OnArrived): walk to the current point, wait its `delay` ms, then start the
/// next leg, wrapping after the last point. Returns the new position and the
/// point's orientation when it is to be faced (arrival with a delay).
fn tick_waypoint(
    position: Position,
    state: &mut WaypointState,
    path: &WaypointPath,
    dt: f32,
) -> (Position, Option<f32>) {
    tick_waypoint_with_travel(position, state, path, dt, |from, to, distance, _| {
        direct_leg(from, to, distance)
    })
}

fn direct_leg(mut from: Position, to: Position, distance: f32) -> (Position, bool) {
    let (reached, x, z) = move_toward(from.x, from.z, to.x, to.z, distance);
    from.x = x;
    from.z = z;
    if reached {
        from.y = to.y;
    }
    (from, reached)
}

fn tick_waypoint_with_travel(
    mut position: Position,
    state: &mut WaypointState,
    path: &WaypointPath,
    dt: f32,
    mut travel: impl FnMut(Position, Position, f32, usize) -> (Position, bool),
) -> (Position, Option<f32>) {
    if state.delay_remaining > 0.0 {
        state.delay_remaining -= dt;
        if state.delay_remaining > 0.0 {
            return (position, None);
        }
    }
    if state.arrived {
        start_next_leg(state, &position, path.points.len());
    }
    let point = &path.points[state.current_index % path.points.len()];
    // Waypoint data is WoW coords; convert target to Bevy for movement.
    let (tx, ty, tz) = wow_to_bevy(point.position_x, point.position_y, point.position_z);
    let step = state.speed(point.move_type) * dt;
    let (next, reached) = travel(
        position,
        Position {
            x: tx,
            y: ty,
            z: tz,
        },
        step,
        state.current_index,
    );
    state.moved = (next.x - position.x).hypot(next.z - position.z);
    position = next;
    if !reached {
        return (position, None);
    }
    // Travel owns height: direct legs keep authored arrival Z, routed legs use
    // actual ground support instead of overwriting it with approximate waypoint Z.
    state.arrived = true;
    state.delay_remaining = point.delay as f32 / 1000.0;
    (position, point.orientation.filter(|_| point.delay > 0))
}

fn start_next_leg(state: &mut WaypointState, position: &Position, path_len: usize) {
    state.arrived = false;
    state.leg_origin = (position.x, position.z);
    state.current_index = (state.current_index + 1) % path_len;
}

// -- Formation logic --

/// Move formation members with their patrolling leader (CreatureGroup::LeaderMoveTo):
/// each member heads for the leader's current point offset by `follow_dist` at
/// `follow_angle` from the leg's direction, at the speed that lands it there
/// when the leader arrives. Dead members and members of a dead leader hold
/// still: LeaderMoveTo skips `!member->IsAlive()`, a dead leader never reaches
/// StartMove, and its death dismisses the group (`Creature::setDeathState` →
/// `FormationReset(true, false)`, which leaves members' motion untouched).
fn formation_system(
    world_data: Res<WorldData>,
    queues: MoveQueues,
    leaders: Query<
        (&Position, &WaypointState, Option<&CreatureState>),
        (
            Without<FormationMember>,
            Without<DeathState>,
            Without<GridDormant>,
        ),
    >,
    mut members: Query<
        (
            Entity,
            &mut Position,
            &FormationMember,
            Option<&CreatureState>,
            Bearing,
        ),
        Roaming,
    >,
) {
    for (entity, mut pos, member, member_state, bearing) in &mut members {
        if matches!(member_state, Some(s) if !s.is_idle()) {
            continue;
        }
        let Ok((leader_pos, leg, leader_state)) = leaders.get(member.leader) else {
            continue;
        };
        let point = world_data
            .waypoints
            .get(&leg.path_id)
            .and_then(|path| path.points.get(leg.current_index));
        let leading = leg.moved > 0.0 && !matches!(leader_state, Some(s) if !s.is_idle());
        let stride = match point.filter(|_| leading) {
            Some(point) => {
                let slot = formation_slot(leg.leg_origin, point, member);
                let (px, _, pz) = wow_to_bevy(point.position_x, point.position_y, point.position_z);
                let leader_remaining = (px - leader_pos.x).hypot(pz - leader_pos.z);
                let next_position = follow_leader_step(*pos, slot, leader_remaining, leg.moved);
                let speed = leg.speed(point.move_type);
                Stride::moving(*pos, next_position, leg_motion(point.move_type), speed)
            }
            None => Stride::still(*pos),
        };
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

/// Bevy position of a member's slot at `point`: AzerothCore offsets
/// `follow_angle` from the leg's direction (its `pathAngle` points back at the
/// leader, then flips by pi whenever the leader faces its travel direction).
fn formation_slot(origin: (f32, f32), point: &WaypointPoint, member: &FormationMember) -> Position {
    let (ox, oy, _) = bevy_to_wow(origin.0, 0.0, origin.1);
    let heading = (point.position_y - oy).atan2(point.position_x - ox);
    let angle = heading + member.follow_angle;
    let (x, y, z) = wow_to_bevy(
        point.position_x + angle.cos() * member.follow_dist,
        point.position_y + angle.sin() * member.follow_dist,
        point.position_z,
    );
    Position { x, y, z }
}

/// Close the member's distance to `slot` in the same proportion the leader
/// closed its own distance this tick (`leader_moved` of `leader_remaining +
/// leader_moved`), so both arrive together.
fn follow_leader_step(
    mut position: Position,
    slot: Position,
    leader_remaining: f32,
    leader_moved: f32,
) -> Position {
    let keep = leader_remaining / (leader_remaining + leader_moved);
    if keep <= 0.0 {
        return slot;
    }
    position.x = slot.x + (position.x - slot.x) * keep;
    position.z = slot.z + (position.z - slot.z) * keep;
    position
}

// -- Chase logic --

type ChasingUnits<'w, 's> = Query<
    'w,
    's,
    Chaser,
    (
        ActiveUncontrolled,
        Without<crate::scripted_ai::caster_chase::CasterChase>,
    ),
>;

/// Run pursuing creatures toward their target each tick
/// (`ChaseMovementGenerator`, run speed × the aura speed multiplier) until within
/// `CHASE_REACH`, on the floor under it, facing the way it runs and, in reach, the target.
/// A stunned or rooted creature does not move, nor does one casting a spell that
/// movement would interrupt (`Unit::IsMovementPreventedByCasting`).
pub(crate) fn chase_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut nav: Option<ResMut<Navigation>>,
    mut commands: Commands,
    mut units: ParamSet<(ChasingUnits, Query<&Position>)>,
) {
    let dt = time.delta_secs();
    let targets = victim_positions(&mut units);
    for (
        entity,
        mut pos,
        spawn,
        state,
        cast,
        mods,
        map,
        bearing,
        combat_move_disabled,
        cannot_reach,
        follower_reach,
        default_waypoint,
    ) in &mut units.p0()
    {
        let CreatureState::Pursuing { target } = *state else {
            continue;
        };
        // MotionMaster initializes a newly-preempting active generator with
        // diff0. The default spline may already have moved before sight/aggro.
        let initializing =
            default_waypoint.is_some_and(|mut path| path.preempt_for(Goal::Chase(target)));
        if initializing {
            continue;
        }
        let speed = mods.map_or(1.0, |mods| mods.movement_speed);
        if speed <= 0.0
            || combat_move_disabled.is_some()
            || cast.is_some_and(|cast| !cast.cast_while_moving)
        {
            let stride = Stride::still(*pos);
            queues.apply(entity, &mut pos, bearing, stride);
            continue;
        }
        let Some(target_pos) = targets.get(&Entity::from_bits(target)) else {
            continue; // target despawned or invalid
        };
        let run_speed = spawn.speed_run * BASE_RUN_SPEED * speed;
        let reach = follower_reach.map_or(CHASE_REACH, |reach| reach.0);
        let mut next_position = match (nav.as_deref_mut(), map) {
            (Some(nav), Some(map)) => match nav.advance(
                entity,
                MoveRequest {
                    map_id: map.map_id,
                    goal: Goal::Chase(target),
                    from: *pos,
                    to: *target_pos,
                    distance: run_speed * dt,
                    reach,
                },
            ) {
                Step::Unbaked if follower_reach.is_some() => *pos,
                Step::Unbaked => chase_step(*pos, target_pos, run_speed * dt, reach),
                Step::Moved { position, .. } => position,
                Step::Blocked => {
                    let stride = Stride::still(*pos);
                    queues.apply(entity, &mut pos, bearing, stride);
                    if cannot_reach.is_none_or(|blocked| blocked.target != target) {
                        commands.entity(entity).insert(CannotReachTarget {
                            target,
                            elapsed: Duration::ZERO,
                        });
                    }
                    continue;
                }
            },
            _ if follower_reach.is_some() => {
                error!("Follower {entity} cannot chase: navigation resource or map absent");
                *pos
            }
            _ => chase_step(*pos, target_pos, run_speed * dt, reach),
        };
        if cannot_reach.is_some() {
            commands.entity(entity).remove::<CannotReachTarget>();
        }
        if next_position != *pos {
            let feet = Vec3::new(next_position.x, next_position.y, next_position.z);
            next_position.y = run_ground(
                probe_ground(ground.as_deref_mut(), map, feet),
                next_position.y,
            );
        }
        let mut stride = Stride::moving(*pos, next_position, CreatureMotion::Run, run_speed);
        if stride.facing.is_none() {
            // In reach: `ChaseMovementGenerator` turns it to face the target (`SetInFront`).
            stride.facing = facing_along(target_pos.x - pos.x, target_pos.z - pos.z);
        }
        bevy::log::debug!(?entity, from = ?*pos, to = ?next_position, ?state, driver = "chase", "combat movement");
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

type Chaser = (
    Entity,
    &'static mut Position,
    &'static SpawnPoint,
    &'static CreatureState,
    Option<&'static CastState>,
    Option<&'static AuraStatMods>,
    Option<&'static InMap>,
    Bearing,
    Option<&'static crate::scripted_ai::smart_ai::CombatMovementDisabled>,
    Option<&'static CannotReachTarget>,
    Option<&'static crate::followers::FollowerReach>,
    Option<&'static mut crate::scripted_ai::waypoint_path::PathMovement>,
);

/// `Creature::Update`: no-path evade after five seconds, never on raid maps.
/// Reachable paths remove the flag; a new chase records a fresh timeout.
fn no_path_evade_system(
    time: Res<Time>,
    maps: Res<MapStore>,
    mut blocked: Query<
        (
            Entity,
            &mut CannotReachTarget,
            &CreatureState,
            Option<&InMap>,
        ),
        ActiveUncontrolled,
    >,
    mut commands: Commands,
) {
    for (entity, mut blocked, state, map) in &mut blocked {
        if *state
            != (CreatureState::Pursuing {
                target: blocked.target,
            })
        {
            commands.entity(entity).remove::<CannotReachTarget>();
            continue;
        }
        if map
            .and_then(|map| maps.map(map.map_id))
            .is_some_and(|map| map.instance_type == InstanceType::Raid)
        {
            continue;
        }
        blocked.elapsed += time.delta();
        if blocked.elapsed >= NO_PATH_EVADE_TIME {
            commands.queue(move |world: &mut World| {
                crate::creature_aggro::enter_evade_mode(world, entity);
            });
        }
    }
}

/// Where each chased unit stands at the start of the chase step. A victim may be a
/// creature that moves in the same step (a soldier and the worg it fights).
fn victim_positions(
    units: &mut ParamSet<(ChasingUnits, Query<&Position>)>,
) -> EntityHashMap<Position> {
    let victims: Vec<Entity> = units
        .p0()
        .iter()
        .filter_map(|(_, _, _, state, ..)| match *state {
            CreatureState::Pursuing { target } => Entity::try_from_bits(target),
            _ => None,
        })
        .collect();
    let positions = units.p1();
    victims
        .into_iter()
        .filter_map(|victim| {
            positions
                .get(victim)
                .ok()
                .map(|position| (victim, *position))
        })
        .collect()
}

/// Move up to `step` across the ground toward the point `CHASE_REACH` short of `target`;
/// the height is the caller's (the floor there).
fn chase_step(position: Position, target: &Position, step: f32, reach: f32) -> Position {
    let gap = (target.x - position.x).hypot(target.z - position.z);
    let travel = (gap - reach).clamp(0.0, step);
    if travel <= 0.0 {
        return position;
    }
    let ratio = travel / gap;
    Position {
        x: position.x + (target.x - position.x) * ratio,
        y: position.y,
        z: position.z + (target.z - position.z) * ratio,
    }
}

// -- Follow logic --

/// `PET_FOLLOW_DIST`.
const PET_FOLLOW_DIST: f32 = 1.0;
/// `PET_FOLLOW_ANGLE`: behind the owner, within `ChaseAngle`'s default tolerance of π/4.
const PET_FOLLOW_ANGLE: f32 = std::f32::consts::PI;
const PET_FOLLOW_ANGLE_TOLERANCE: f32 = std::f32::consts::FRAC_PI_4;
/// `DEFAULT_PLAYER_COMBAT_REACH` of the owner and, without creature model data, of the pet.
const FOLLOW_COMBAT_REACH: f32 = 1.5;
/// `FOLLOW_RANGE_TOLERANCE` (FollowMovementGenerator.cpp).
const FOLLOW_RANGE_TOLERANCE: f32 = 1.0;

type Follower = (
    Entity,
    &'static mut Position,
    &'static mut FollowOwner,
    &'static Health,
    Option<&'static CastState>,
    Option<&'static AuraStatMods>,
    Option<&'static InMap>,
    Bearing,
    Option<&'static crate::followers::Follower>,
    Option<&'static crate::followers::FollowerReach>,
);

/// Where an owner stands and how fast it runs.
#[derive(Clone, Copy)]
struct Leader {
    position: Position,
    rotation_y: f32,
    speed: f32,
}

/// `FollowMovementGenerator::Update` for a pet: once its owner is farther than both combat
/// reaches plus `PET_FOLLOW_DIST` and `FOLLOW_RANGE_TOLERANCE`, it runs to the point that
/// far from the owner at its current angle around it, clamped to `PET_FOLLOW_ANGLE` ±
/// π/4, on the floor, and on arrival faces the owner's way (`SetFacing`). It runs at the
/// owner's speed scaled by `Unit::UpdateSpeed`'s catch-up factor: 0.75 + 0.05 per yard
/// beyond `PET_FOLLOW_DIST`, within 1-1.3. It holds still while it cannot move, casts
/// a spell movement would interrupt, or is dead. Dungeon followers use baked routes;
/// other FollowOwner consumers retain their existing straight movement.
#[cfg(test)]
pub(crate) fn install_follow_fixture(app: &mut App) {
    app.add_systems(Update, follow_owner_system);
}

#[cfg(test)]
pub(crate) fn install_follower_follow_fixture(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            crate::followers::update_ai,
            chase_system,
            follow_owner_system,
        )
            .chain(),
    );
}

fn follow_owner_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut nav: Option<ResMut<Navigation>>,
    mut units: ParamSet<(
        Query<Follower, ActiveUncontrolled>,
        Query<(&Position, Option<&Rotation>, Option<&MovementSpeed>)>,
    )>,
) {
    let dt = time.delta_secs();
    let owners: Vec<Entity> = units
        .p0()
        .iter()
        .map(|(_, _, follow, ..)| follow.owner)
        .collect();
    let owner_query = units.p1();
    let leaders: EntityHashMap<Leader> = owners
        .into_iter()
        .filter_map(|owner| {
            let (position, rotation, speed) = owner_query.get(owner).ok()?;
            Some((
                owner,
                Leader {
                    position: *position,
                    rotation_y: rotation.map_or(0.0, |rotation| rotation.y),
                    speed: speed.map_or(BASE_RUN_SPEED, |speed| speed.0),
                },
            ))
        })
        .collect();
    for (entity, mut pos, mut follow, health, cast, mods, map, bearing, companion, companion_pet) in
        &mut units.p0()
    {
        let Some(leader) = leaders.get(&follow.owner) else {
            continue;
        };
        let speed_mod = mods.map_or(1.0, |mods| mods.movement_speed);
        if health.current <= 0.0
            || speed_mod <= 0.0
            || cast.is_some_and(|cast| !cast.cast_while_moving)
        {
            follow.moving = false;
            let here = *pos;
            queues.apply(entity, &mut pos, bearing, Stride::still(here));
            continue;
        }
        let reach = 2.0 * FOLLOW_COMBAT_REACH + PET_FOLLOW_DIST;
        let gap = (leader.position.x - pos.x).hypot(leader.position.z - pos.z);
        if !follow.moving && gap <= reach + FOLLOW_RANGE_TOLERANCE {
            let here = *pos;
            queues.apply(entity, &mut pos, bearing, Stride::still(here));
            continue;
        }
        let point = follow_point(&leader, *pos, reach);
        let run_speed = leader.speed * speed_mod * catch_up_factor(gap);
        let movement = if companion.is_some() || companion_pet.is_some() {
            let request = map.map(|map| MoveRequest {
                map_id: map.map_id,
                goal: Goal::Follow(follow.owner.to_bits()),
                from: *pos,
                to: Position {
                    x: point.0,
                    y: leader.position.y,
                    z: point.1,
                },
                distance: run_speed * dt,
                reach: 0.0,
            });
            advance_companion_route(nav.as_deref_mut(), entity, request)
        } else {
            let (reached, x, z) = move_toward(pos.x, pos.z, point.0, point.1, run_speed * dt);
            Some((Position { x, y: pos.y, z }, reached))
        };
        let Some((mut next_position, reached)) = movement else {
            follow.moving = false;
            let here = *pos;
            queues.apply(entity, &mut pos, bearing, Stride::still(here));
            continue;
        };
        let (x, z) = (next_position.x, next_position.z);
        if next_position != *pos {
            let feet = Vec3::new(x, pos.y, z);
            next_position.y = run_ground(probe_ground(ground.as_deref_mut(), map, feet), pos.y);
        }
        let mut stride = Stride::moving(*pos, next_position, CreatureMotion::Run, run_speed);
        follow.moving = !reached;
        if reached {
            stride.motion = CreatureMotion::Still;
            stride.facing = Some(leader.rotation_y);
        }
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

/// Follower dungeons require an authored bake; unavailable or blocked routes never
/// become direct movement. Other existing FollowOwner consumers keep their behavior.
fn advance_companion_route(
    nav: Option<&mut Navigation>,
    entity: Entity,
    request: Option<MoveRequest>,
) -> Option<(Position, bool)> {
    let (Some(nav), Some(request)) = (nav, request) else {
        error!("Follower {entity} cannot follow: navigation resource or map absent");
        return None;
    };
    match nav.advance(entity, request) {
        Step::Moved { position, reached } => Some((position, reached)),
        Step::Unbaked | Step::Blocked => None,
    }
}

/// Follower hazard escape uses the same native movement/interest/replication writer.
pub(crate) fn move_follower(world: &mut World, entity: Entity, to: Position, speed: f32) {
    let mut state = bevy::ecs::system::SystemState::<(
        MoveQueues,
        Query<
            (
                &mut Position,
                Option<&mut Rotation>,
                Option<&mut CreatureMotion>,
                Option<&mut MovementSpeed>,
            ),
            ActiveUncontrolled,
        >,
    )>::new(world);
    let (queues, mut units) = state
        .get_mut(world)
        .expect("follower native movement system parameters");
    let Ok((mut position, rotation, motion, movement_speed)) = units.get_mut(entity) else {
        debug!("Follower {entity} escape movement held while externally controlled");
        return;
    };
    let stride = Stride::moving(*position, to, CreatureMotion::Run, speed);
    queues.apply(
        entity,
        &mut position,
        (rotation, motion, movement_speed),
        stride,
    );
    state.apply(world);
}

/// Bevy (x, z) `reach` yards from the owner at the follower's angle around it, clamped to
/// `PET_FOLLOW_ANGLE` ± `PET_FOLLOW_ANGLE_TOLERANCE` (`ChaseAngle::IsAngleOkay`).
fn follow_point(leader: &Leader, follower: Position, reach: f32) -> (f32, f32) {
    use std::f32::consts::{PI, TAU};
    let orientation = leader.rotation_y - std::f32::consts::FRAC_PI_2;
    // WoW (x, y) offsets are Bevy (x, -z).
    let (dx, dy) = (
        follower.x - leader.position.x,
        -(follower.z - leader.position.z),
    );
    let relative = (dy.atan2(dx) - orientation).rem_euclid(TAU);
    let off = (relative - PET_FOLLOW_ANGLE + PI).rem_euclid(TAU) - PI;
    let angle =
        PET_FOLLOW_ANGLE + off.clamp(-PET_FOLLOW_ANGLE_TOLERANCE, PET_FOLLOW_ANGLE_TOLERANCE);
    let absolute = orientation + angle;
    (
        leader.position.x + reach * absolute.cos(),
        leader.position.z - reach * absolute.sin(),
    )
}

/// `Unit::UpdateSpeed` for a minion following its owner out of combat (Unit.cpp:8912-8924).
fn catch_up_factor(gap: f32) -> f32 {
    (0.75 + (gap - PET_FOLLOW_DIST) * 0.05).clamp(1.0, 1.3)
}

// -- Pet motion --

/// A pet's movement while it does not follow its owner (`PetAI`; a following pet has
/// `FollowOwner` instead, never both): chasing its victim (`MoveChase`), running to a
/// point (`MovePoint`: Move To's ground point, or back to where Stay held it), or holding
/// still there (`MoveIdle`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum PetMotion {
    Chase(Entity),
    Point(Position),
    Idle,
}

type PetMover = (
    Entity,
    &'static mut Position,
    &'static mut PetMotion,
    &'static Health,
    Option<&'static CastState>,
    Option<&'static AuraStatMods>,
    Option<&'static InMap>,
    Bearing,
    Option<&'static crate::followers::FollowerReach>,
);

/// `PetAI` movement at the base run speed × the aura speed multiplier, on the floor: a
/// chasing pet runs to `CHASE_REACH` of its victim and, in reach, faces it; a pet sent to
/// a point runs there, then holds still (`PetAI::MovementInform` → `MoveIdle`). It holds
/// still while dead, unable to move, or casting a spell movement would interrupt. Paths are
/// straight, and the chase does not seek `PetAI::DoAttack`'s angle behind the victim.
fn pet_motion_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut nav: Option<ResMut<Navigation>>,
    mut units: ParamSet<(Query<PetMover, ActiveUncontrolled>, Query<&Position>)>,
) {
    let dt = time.delta_secs();
    let victims: Vec<Entity> = units
        .p0()
        .iter()
        .filter_map(|(_, _, motion, ..)| match *motion {
            PetMotion::Chase(victim) => Some(victim),
            _ => None,
        })
        .collect();
    let victim_positions: EntityHashMap<Position> = {
        let positions = units.p1();
        victims
            .into_iter()
            .filter_map(|victim| positions.get(victim).ok().map(|at| (victim, *at)))
            .collect()
    };
    for (entity, mut pos, mut motion, health, cast, mods, map, bearing, companion_pet) in
        &mut units.p0()
    {
        let speed = mods.map_or(1.0, |mods| mods.movement_speed);
        let held = health.current <= 0.0
            || speed <= 0.0
            || cast.is_some_and(|cast| !cast.cast_while_moving);
        let run_speed = BASE_RUN_SPEED * speed;
        let here = *pos;
        let (next_position, face) = match *motion {
            _ if held => (here, None),
            PetMotion::Idle => (here, None),
            PetMotion::Chase(victim) => {
                let Some(target) = victim_positions.get(&victim) else {
                    continue;
                };
                let next = if companion_pet.is_some() {
                    let request = map.map(|map| MoveRequest {
                        map_id: map.map_id,
                        goal: Goal::Chase(victim.to_bits()),
                        from: here,
                        to: *target,
                        distance: run_speed * dt,
                        reach: CHASE_REACH,
                    });
                    advance_companion_route(nav.as_deref_mut(), entity, request)
                        .map_or(here, |(position, _)| position)
                } else {
                    chase_step(here, target, run_speed * dt, CHASE_REACH)
                };
                let face = (next == here)
                    .then(|| facing_along(target.x - here.x, target.z - here.z))
                    .flatten();
                (next, face)
            }
            PetMotion::Point(point) => {
                let movement = if companion_pet.is_some() {
                    let request = map.map(|map| MoveRequest {
                        map_id: map.map_id,
                        goal: Goal::Point,
                        from: here,
                        to: point,
                        distance: run_speed * dt,
                        reach: 0.0,
                    });
                    advance_companion_route(nav.as_deref_mut(), entity, request)
                } else {
                    let (reached, x, z) =
                        move_toward(here.x, here.z, point.x, point.z, run_speed * dt);
                    Some((Position { x, y: here.y, z }, reached))
                };
                let (position, reached) = movement.unwrap_or((here, false));
                if reached {
                    *motion = PetMotion::Idle;
                }
                (position, None)
            }
        };
        let mut next = next_position;
        if next != here {
            let feet = Vec3::new(next.x, next.y, next.z);
            next.y = run_ground(probe_ground(ground.as_deref_mut(), map, feet), next.y);
        }
        let mut stride = if next == here {
            Stride::still(here)
        } else {
            Stride::moving(here, next, CreatureMotion::Run, run_speed)
        };
        if face.is_some() {
            stride.facing = face;
        }
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

// -- Confused movement --

type ConfusedMover = (
    Entity,
    &'static mut Position,
    &'static SpawnPoint,
    &'static mut Confused,
    Option<&'static AuraStatMods>,
    Option<&'static InMap>,
    Bearing,
);

/// `ConfusedMovementGenerator`: a confused creature walks (walk speed × the aura speed
/// multiplier) to a random point up to `CONFUSED_REACH` either way from where its
/// confusion began, pauses `CONFUSED_PAUSE_SECS`, and picks the next; it holds still
/// while it cannot move (`UNIT_STATE_NOT_MOVE`). It overrides chase, wander and
/// waypoints (`MOTION_SLOT_CONTROLLED`). A point without ground under it is retried.
fn confused_movement_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut query: Query<
        ConfusedMover,
        (
            Without<DeathState>,
            Without<GridDormant>,
            Without<crate::knockback::KnockbackFlight>,
        ),
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, mut confused, mods, map, bearing) in &mut query {
        let speed =
            spawn.speed_walk * BASE_WALK_SPEED * mods.map_or(1.0, |mods| mods.movement_speed);
        let mut probe = |feet: Vec3| probe_ground(ground.as_deref_mut(), map, feet);
        let next_position = if speed <= 0.0 {
            confused.destination = None;
            *pos
        } else {
            confused_step(*pos, &mut confused, speed * dt, dt, &mut probe)
        };
        let stride = Stride::moving(*pos, next_position, CreatureMotion::Walk, speed);
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

/// One tick of confused movement: pause, pick a point (`SetTargetLocation`), or walk
/// toward it.
fn confused_step(
    mut position: Position,
    confused: &mut Confused,
    step: f32,
    dt: f32,
    probe: &mut impl FnMut(Vec3) -> GroundProbe,
) -> Position {
    let Some((tx, tz)) = confused.destination else {
        confused.wait_secs -= dt;
        if confused.wait_secs > 0.0 {
            return position;
        }
        let (tx, tz) = confused_point(confused.reference);
        if wander_ground(probe(Vec3::new(tx, position.y, tz)), position.y).is_none() {
            confused.wait_secs = RETRY_CONFUSED_SECS;
            return position;
        }
        confused.destination = Some((tx, tz));
        return position;
    };
    let (reached, nx, nz) = move_toward(position.x, position.z, tx, tz, step);
    let next_y = wander_ground(probe(Vec3::new(nx, position.y, nz)), position.y);
    if let Some(y) = next_y {
        position.x = nx;
        position.y = y;
        position.z = nz;
    }
    if reached || next_y.is_none() {
        confused.destination = None;
        confused.wait_secs = rand::Rng::gen_range(
            &mut rand::thread_rng(),
            CONFUSED_PAUSE_SECS.0..=CONFUSED_PAUSE_SECS.1,
        );
    }
    position
}

/// `4 * frand(0, 1) - 2` yards at a random angle from `reference`.
fn confused_point(reference: (f32, f32)) -> (f32, f32) {
    let mut rng = rand::thread_rng();
    let distance = rand::Rng::gen_range(&mut rng, -CONFUSED_REACH..=CONFUSED_REACH);
    let angle = rand::Rng::gen_range(&mut rng, 0.0..std::f32::consts::TAU);
    (
        reference.0 + distance * angle.cos(),
        reference.1 + distance * angle.sin(),
    )
}

/// Check if position exceeds leash distance from spawn (2D, ignoring height).
fn beyond_leash(pos: &Position, spawn: &SpawnPoint) -> bool {
    let dx = pos.x - spawn.x;
    let dz = pos.z - spawn.z;
    let dist_sq = dx * dx + dz * dz;
    dist_sq > LEASH_DISTANCE * LEASH_DISTANCE
}

/// Evade pursuing creatures that exceed leash distance from spawn
/// (`enter_evade_mode`: threat cleared, combat stopped, health restored,
/// Returning). Not in a dungeon: `Creature::CanCreatureAttack` (Creature.cpp:2731)
/// keeps any target there regardless of home distance.
fn leash_check_system(
    query: Query<
        (
            Entity,
            &Position,
            &SpawnPoint,
            &CreatureState,
            Option<&crate::scripted_ai::movement::ScriptedPath>,
            Option<&HomePosition>,
            Option<&InMap>,
        ),
        (ActiveUncontrolled, Without<crate::followers::Follower>),
    >,
    maps: Option<Res<MapStore>>,
    mut commands: Commands,
) {
    for (entity, pos, spawn, state, escort, explicit_home, map) in &query {
        let dungeon = map
            .zip(maps.as_deref())
            .and_then(|(map, maps)| maps.map(map.map_id))
            .is_some_and(|entry| entry.is_dungeon());
        if dungeon {
            continue;
        }
        let beyond_home = match explicit_home {
            Some(home) => (pos.x - home.position.x).hypot(pos.z - home.position.z) > LEASH_DISTANCE,
            None => match escort {
                Some(escort) => {
                    (pos.x - escort.home.x).hypot(pos.z - escort.home.z) > LEASH_DISTANCE
                }
                None => beyond_leash(pos, spawn),
            },
        };
        if matches!(*state, CreatureState::Pursuing { .. }) && beyond_home {
            bevy::log::debug!(
                ?entity,
                ?pos,
                ?spawn,
                ?state,
                driver = "spawn_leash",
                "enter evade"
            );
            commands.queue(move |world: &mut World| {
                crate::creature_aggro::enter_evade_mode(world, entity);
            });
        }
    }
}

/// Run evading creatures back to their spawn point (`MoveTargetedHome`,
/// run speed), then transition to Idle. At home the creature's mana is full and
/// its addon auras and pose return (`HomeMovementGenerator::DoFinalize` →
/// `SetSpawnHealth`, `LoadCreaturesAddon`). `SetSpawnHealth` restores health only
/// for a creature without `RegenHealth` ("otherwise it would immediately go back to
/// full health anyway"): full, as `curHealthPct` is not loaded. A regenerating
/// creature heals by `Creature::RegenerateHealth` once it is no longer evading.
pub(crate) fn return_to_spawn_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut nav: Option<ResMut<Navigation>>,
    mut query: Query<
        (
            Entity,
            &mut Position,
            &SpawnPoint,
            &mut CreatureState,
            Option<&mut shared::components::Mana>,
            Option<&crate::zones::SpawnAddonAuras>,
            Option<(&crate::zones::SpawnPose, &mut shared::components::UnitPose)>,
            Option<&InMap>,
            Bearing,
            Option<&mut shared::components::Health>,
            Has<crate::spell_cast::powers::CreatureHealthRegenTimer>,
            Option<&crate::scripted_ai::movement::ScriptedPath>,
            Option<&HomePosition>,
            Option<&mut crate::scripted_ai::waypoint_path::PathMovement>,
        ),
        (
            Without<GridDormant>,
            Without<crate::scripted_ai::jump_movement::JumpMovement>,
            Without<crate::spell_cast::charge::ChargeMovement>,
        ),
    >,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (
        entity,
        mut pos,
        spawn,
        mut state,
        mana,
        addon_auras,
        pose,
        map,
        bearing,
        health,
        regen,
        escort,
        explicit_home,
        default_waypoint,
    ) in &mut query
    {
        if *state != CreatureState::Returning {
            continue;
        }
        let initializing = default_waypoint.is_some_and(|mut path| path.preempt_for(Goal::Home));
        if initializing {
            continue;
        }
        bevy::log::debug!(?entity, from = ?*pos, ?spawn, driver = "return_home", "return movement");
        let speed = spawn.speed_run * BASE_RUN_SPEED;
        let home = explicit_home.map_or_else(
            || {
                escort.map_or(
                    Position {
                        x: spawn.x,
                        y: spawn.y,
                        z: spawn.z,
                    },
                    |escort| escort.home,
                )
            },
            |home| home.position,
        );
        let (mut next_position, reached) = match (nav.as_deref_mut(), map) {
            (Some(nav), Some(map)) => match nav.advance(
                entity,
                MoveRequest {
                    map_id: map.map_id,
                    goal: Goal::Home,
                    from: *pos,
                    to: home,
                    distance: speed * dt,
                    reach: 0.0,
                },
            ) {
                Step::Unbaked => direct_leg(*pos, home, speed * dt),
                Step::Moved {
                    mut position,
                    reached,
                } => {
                    if reached {
                        position =
                            sample_movement_ground(ground.as_deref_mut(), Some(map), position);
                    }
                    (position, reached)
                }
                Step::Blocked => {
                    let stride = Stride::still(*pos);
                    queues.apply(entity, &mut pos, bearing, stride);
                    continue;
                }
            },
            _ => direct_leg(*pos, home, speed * dt),
        };
        if !reached {
            let feet = Vec3::new(next_position.x, next_position.y, next_position.z);
            next_position.y = run_ground(
                probe_ground(ground.as_deref_mut(), map, feet),
                next_position.y,
            );
        }
        let mut stride = Stride::moving(*pos, next_position, CreatureMotion::Run, speed);
        if reached {
            stride.motion = CreatureMotion::Still;
            if let Some(home) = explicit_home {
                stride.facing = Some(home.orientation);
            }
        }
        queues.apply(entity, &mut pos, bearing, stride);
        if reached {
            *state = CreatureState::Idle;
            if let Some(mut mana) = mana
                && mana.current != mana.max
            {
                mana.current = mana.max;
            }
            if let Some(mut health) = health.filter(|_| !regen)
                && health.current != health.max
            {
                health.current = health.max;
                if let Some(replication) = queues.replication.as_deref() {
                    replication.mark(entity);
                }
            }
            if let Some(auras) = addon_auras {
                commands
                    .entity(entity)
                    .insert(crate::zones::AddonAuras(auras.0.clone()));
            }
            if let Some((spawn_pose, mut pose)) = pose
                && pose.set_if_neq(spawn_pose.0)
                && let Some(replication) = queues.replication.as_deref()
            {
                replication.mark(entity);
            }
        }
    }
}

#[cfg(test)]
#[path = "creature_ai_tests.rs"]
mod tests;
