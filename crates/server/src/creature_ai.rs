use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{CreatureMotion, Position, Rotation};

use crate::auras::AuraStatMods;
use crate::combat::DeathState;
use crate::grid_activity::GridDormant;
use crate::ground::{GroundMap, GroundProbe};
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};
use crate::maps::InMap;
use crate::terrain_height::{bevy_to_wow, wow_to_bevy};
use crate::world_data::{WaypointMoveType, WaypointPath, WaypointPoint, WorldData};
use shared::casting::CastState;
use shared::ground::STEP_UP_HEIGHT;

/// Walk speed in world-units per second (base WoW walk speed).
const BASE_WALK_SPEED: f32 = 2.5;
/// Run speed in world-units per second (AzerothCore `baseMoveSpeed[MOVE_RUN]`).
const BASE_RUN_SPEED: f32 = 7.0;
/// Maximum chase distance from spawn before evading (yards).
const LEASH_DISTANCE: f32 = 40.0;
/// Horizontal distance a chaser closes to (yards), inside the 5 yard
/// melee range (`ATTACK_DISTANCE`) so auto-attacks land.
const CHASE_REACH: f32 = 2.0;
/// Idle pause range in seconds after reaching a wander destination.
const IDLE_MIN_SECS: f32 = 2.0;
const IDLE_MAX_SECS: f32 = 5.0;
/// Delay before a wanderer retries a destination it cannot walk to
/// (`RandomMovementGenerator` `_timer.Reset(100)`).
const RETRY_WANDER_SECS: f32 = 0.1;

pub struct CreatureAiPlugin;

impl Plugin for CreatureAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                wander_system,
                (waypoint_system, formation_system).chain(),
                (chase_system, leash_check_system, return_to_spawn_system).chain(),
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

/// High-level creature behavior state. Overrides wander/waypoint when non-idle.
#[derive(Component, Debug, Clone, PartialEq, Default)]
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
        (Without<DeathState>, Without<GridDormant>),
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, mut state, creature_state, map, bearing) in &mut query {
        if matches!(creature_state, Some(s) if !s.is_idle()) {
            continue;
        }
        let mut probe = |feet: Vec3| probe_ground(ground.as_deref_mut(), map, feet);
        let next_position = match state.phase {
            WanderPhase::Idle => {
                tick_idle(&mut state, dt, spawn, *pos, &mut probe);
                *pos
            }
            WanderPhase::Walking => tick_walk(*pos, &mut state, spawn, dt, &mut probe),
        };
        let stride = Stride::moving(*pos, next_position, CreatureMotion::Walk);
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

/// The ground under `feet` on the creature's map; `Unloaded` without ground data.
fn probe_ground(ground: Option<&mut GroundMap>, map: Option<&InMap>, feet: Vec3) -> GroundProbe {
    match (ground, map) {
        (Some(ground), Some(map)) => ground.ground_at(map.map_id, feet),
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
}

impl Stride {
    fn still(at: Position) -> Self {
        Self {
            position: at,
            motion: CreatureMotion::Still,
            facing: None,
        }
    }

    /// From `from` to `to` in `motion` (move spline orientation: facing the way it goes),
    /// `Still` when it did not move.
    fn moving(from: Position, to: Position, motion: CreatureMotion) -> Self {
        let facing = facing_along(to.x - from.x, to.z - from.z);
        Self {
            position: to,
            motion: if facing.is_some() {
                motion
            } else {
                CreatureMotion::Still
            },
            facing,
        }
    }
}

/// A creature's facing and motion, replicated to clients.
type Bearing = (
    Option<&'static mut Rotation>,
    Option<&'static mut CreatureMotion>,
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
        (rotation, motion): (Option<Mut<Rotation>>, Option<Mut<CreatureMotion>>),
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
    mut query: Query<
        (
            Entity,
            &mut Position,
            &mut WaypointState,
            Option<&CreatureState>,
            Bearing,
        ),
        (Without<DeathState>, Without<GridDormant>),
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, mut state, creature_state, bearing) in &mut query {
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
        let (next_position, orientation) = tick_waypoint(*pos, &mut state, path, dt);
        let leg = &path.points[state.current_index % path.points.len()];
        let mut stride = Stride::moving(*pos, next_position, leg_motion(leg.move_type));
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
    mut position: Position,
    state: &mut WaypointState,
    path: &WaypointPath,
    dt: f32,
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
    let (reached, nx, nz) = move_toward(position.x, position.z, tx, tz, step);
    state.moved = (nx - position.x).hypot(nz - position.z);
    position.x = nx;
    position.z = nz;
    if !reached {
        return (position, None);
    }
    position.y = ty;
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
        (Without<DeathState>, Without<GridDormant>),
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
                Stride::moving(*pos, next_position, leg_motion(point.move_type))
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

/// Run pursuing creatures toward their target each tick
/// (`ChaseMovementGenerator`, run speed × the aura speed multiplier) until within
/// `CHASE_REACH`, on the floor under it, facing the way it runs and, in reach, the target.
/// A stunned or rooted creature does not move, nor does one casting a spell that
/// movement would interrupt (`Unit::IsMovementPreventedByCasting`).
pub(crate) fn chase_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut chasers: Query<
        (
            Entity,
            &mut Position,
            &SpawnPoint,
            &CreatureState,
            Option<&CastState>,
            Option<&AuraStatMods>,
            Option<&InMap>,
            Bearing,
        ),
        Without<GridDormant>,
    >,
    targets: Query<&Position, Without<SpawnPoint>>,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, state, cast, mods, map, bearing) in &mut chasers {
        let CreatureState::Pursuing { target } = *state else {
            continue;
        };
        let speed = mods.map_or(1.0, |mods| mods.movement_speed);
        if speed <= 0.0 || cast.is_some_and(|cast| !cast.cast_while_moving) {
            let stride = Stride::still(*pos);
            queues.apply(entity, &mut pos, bearing, stride);
            continue;
        }
        let target_entity = Entity::from_bits(target);
        let Ok(target_pos) = targets.get(target_entity) else {
            continue; // target despawned or invalid
        };
        let mut next_position = chase_step(
            *pos,
            target_pos,
            spawn.speed_run * BASE_RUN_SPEED * speed * dt,
        );
        if next_position != *pos {
            let feet = Vec3::new(next_position.x, pos.y, next_position.z);
            next_position.y = run_ground(probe_ground(ground.as_deref_mut(), map, feet), pos.y);
        }
        let mut stride = Stride::moving(*pos, next_position, CreatureMotion::Run);
        if stride.facing.is_none() {
            // In reach: `ChaseMovementGenerator` turns it to face the target (`SetInFront`).
            stride.facing = facing_along(target_pos.x - pos.x, target_pos.z - pos.z);
        }
        queues.apply(entity, &mut pos, bearing, stride);
    }
}

/// Move up to `step` across the ground toward the point `CHASE_REACH` short of `target`;
/// the height is the caller's (the floor there).
fn chase_step(position: Position, target: &Position, step: f32) -> Position {
    let gap = (target.x - position.x).hypot(target.z - position.z);
    let travel = (gap - CHASE_REACH).clamp(0.0, step);
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

/// Check if position exceeds leash distance from spawn (2D, ignoring height).
fn beyond_leash(pos: &Position, spawn: &SpawnPoint) -> bool {
    let dx = pos.x - spawn.x;
    let dz = pos.z - spawn.z;
    let dist_sq = dx * dx + dz * dz;
    dist_sq > LEASH_DISTANCE * LEASH_DISTANCE
}

/// Evade pursuing creatures that exceed leash distance from spawn
/// (`enter_evade_mode`: threat cleared, combat stopped, health restored,
/// Returning).
fn leash_check_system(
    query: Query<(Entity, &Position, &SpawnPoint, &CreatureState), Without<GridDormant>>,
    mut commands: Commands,
) {
    for (entity, pos, spawn, state) in &query {
        if matches!(*state, CreatureState::Pursuing { .. }) && beyond_leash(pos, spawn) {
            commands.queue(move |world: &mut World| {
                crate::creature_aggro::enter_evade_mode(world, entity);
            });
        }
    }
}

/// Run evading creatures back to their spawn point (`MoveTargetedHome`,
/// run speed), then transition to Idle. At home the creature's mana is full and
/// its addon auras return (`HomeMovementGenerator::DoFinalize` → `SetSpawnHealth`,
/// `LoadCreaturesAddon`).
fn return_to_spawn_system(
    time: Res<Time>,
    queues: MoveQueues,
    mut ground: Option<ResMut<GroundMap>>,
    mut query: Query<
        (
            Entity,
            &mut Position,
            &SpawnPoint,
            &mut CreatureState,
            Option<&mut shared::components::Mana>,
            Option<&crate::zones::SpawnAddonAuras>,
            Option<&InMap>,
            Bearing,
        ),
        Without<GridDormant>,
    >,
    mut commands: Commands,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, mut state, mana, addon_auras, map, bearing) in &mut query {
        if *state != CreatureState::Returning {
            continue;
        }
        let speed = spawn.speed_run * BASE_RUN_SPEED;
        let (reached, new_x, new_z) = move_toward(pos.x, pos.z, spawn.x, spawn.z, speed * dt);
        let y = if reached {
            spawn.y
        } else {
            let feet = Vec3::new(new_x, pos.y, new_z);
            run_ground(probe_ground(ground.as_deref_mut(), map, feet), pos.y)
        };
        let next_position = Position {
            x: new_x,
            y,
            z: new_z,
        };
        let mut stride = Stride::moving(*pos, next_position, CreatureMotion::Run);
        if reached {
            stride.motion = CreatureMotion::Still;
        }
        queues.apply(entity, &mut pos, bearing, stride);
        if reached {
            *state = CreatureState::Idle;
            if let Some(mut mana) = mana
                && mana.current != mana.max
            {
                mana.current = mana.max;
            }
            if let Some(auras) = addon_auras {
                commands
                    .entity(entity)
                    .insert(crate::zones::AddonAuras(auras.0.clone()));
            }
        }
    }
}

#[cfg(test)]
#[path = "creature_ai_tests.rs"]
mod tests;
