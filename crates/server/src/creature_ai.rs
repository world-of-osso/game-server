use bevy::prelude::*;
use lightyear::prelude::ReplicationDirtyQueue;
use shared::components::{Position, Rotation};

use crate::combat::DeathState;
use crate::grid_activity::GridDormant;
use crate::interest::{InterestDirtyQueue, mark_interest_position_changed};
use crate::terrain_height::{bevy_to_wow, wow_to_bevy};
use crate::world_data::{WaypointMoveType, WaypointPath, WaypointPoint, WorldData};

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
    /// Simple counter incremented each tick, used for pseudo-random.
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
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut query: Query<
        (
            Entity,
            &mut Position,
            &SpawnPoint,
            &mut WanderState,
            Option<&CreatureState>,
        ),
        (Without<DeathState>, Without<GridDormant>),
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, mut state, creature_state) in &mut query {
        if matches!(creature_state, Some(s) if !s.is_idle()) {
            continue;
        }
        match state.phase {
            WanderPhase::Idle => tick_idle(&mut state, dt, spawn),
            WanderPhase::Walking => {
                let next_position = tick_walk(*pos, &mut state, spawn, dt);
                if *pos != next_position {
                    *pos = next_position;
                    mark_interest_position_changed(dirty_queue.as_deref(), entity);
                }
            }
        }
    }
}

fn tick_idle(state: &mut WanderState, dt: f32, spawn: &SpawnPoint) {
    state.idle_remaining -= dt;
    if state.idle_remaining > 0.0 {
        return;
    }
    // Wander in Bevy horizontal plane (x, z).
    let (tx, tz) = pick_wander_point(spawn.x, spawn.z, spawn.wander_distance, state.seed);
    state.target_x = tx;
    state.target_z = tz;
    state.phase = WanderPhase::Walking;
    state.seed = state.seed.wrapping_add(1);
}

fn tick_walk(
    mut position: Position,
    state: &mut WanderState,
    spawn: &SpawnPoint,
    dt: f32,
) -> Position {
    let speed = spawn.speed_walk * BASE_WALK_SPEED;
    let (reached, new_x, new_z) = move_toward(
        position.x,
        position.z,
        state.target_x,
        state.target_z,
        speed * dt,
    );
    position.x = new_x;
    position.z = new_z;
    if reached {
        state.phase = WanderPhase::Idle;
        state.idle_remaining = random_idle_secs(state.seed);
        state.seed = state.seed.wrapping_add(1);
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
    let next = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (next >> 8) as f32 / 16777216.0 // 2^24
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
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut query: Query<
        (
            Entity,
            &mut Position,
            &mut WaypointState,
            Option<&mut Rotation>,
            Option<&CreatureState>,
        ),
        (Without<DeathState>, Without<GridDormant>),
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, mut state, rotation, creature_state) in &mut query {
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
        let (next_position, facing) = tick_waypoint(*pos, &mut state, path, dt);
        if *pos != next_position {
            *pos = next_position;
            mark_interest_position_changed(dirty_queue.as_deref(), entity);
        }
        if let (Some(orientation), Some(mut rotation)) = (facing, rotation) {
            rotation.y = orientation + std::f32::consts::FRAC_PI_2;
            if let Some(replication_dirty) = replication_dirty.as_deref() {
                replication_dirty.mark(entity);
            }
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
    dirty_queue: Option<Res<InterestDirtyQueue>>,
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
        ),
        (Without<DeathState>, Without<GridDormant>),
    >,
) {
    for (entity, mut pos, member, member_state) in &mut members {
        if matches!(member_state, Some(s) if !s.is_idle()) {
            continue;
        }
        let Ok((leader_pos, leg, leader_state)) = leaders.get(member.leader) else {
            continue;
        };
        if leg.moved <= 0.0 || matches!(leader_state, Some(s) if !s.is_idle()) {
            continue;
        }
        let Some(point) = world_data
            .waypoints
            .get(&leg.path_id)
            .and_then(|path| path.points.get(leg.current_index))
        else {
            continue;
        };
        let slot = formation_slot(leg.leg_origin, point, member);
        let (px, _, pz) = wow_to_bevy(point.position_x, point.position_y, point.position_z);
        let leader_remaining = (px - leader_pos.x).hypot(pz - leader_pos.z);
        let next_position = follow_leader_step(*pos, slot, leader_remaining, leg.moved);
        if *pos != next_position {
            *pos = next_position;
            mark_interest_position_changed(dirty_queue.as_deref(), entity);
        }
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
/// (`ChaseMovementGenerator`, run speed) until within `CHASE_REACH`,
/// following the target's height so the 3D melee range check holds.
pub(crate) fn chase_system(
    time: Res<Time>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut chasers: Query<(Entity, &mut Position, &SpawnPoint, &CreatureState), Without<GridDormant>>,
    targets: Query<&Position, Without<SpawnPoint>>,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, state) in &mut chasers {
        let CreatureState::Pursuing { target } = *state else {
            continue;
        };
        let target_entity = Entity::from_bits(target);
        let Ok(target_pos) = targets.get(target_entity) else {
            continue; // target despawned or invalid
        };
        let next_position = chase_step(*pos, target_pos, spawn.speed_run * BASE_RUN_SPEED * dt);
        if *pos != next_position {
            *pos = next_position;
            mark_interest_position_changed(dirty_queue.as_deref(), entity);
        }
    }
}

/// Move up to `step` toward the point `CHASE_REACH` short of `target`.
fn chase_step(position: Position, target: &Position, step: f32) -> Position {
    let gap = (target.x - position.x).hypot(target.z - position.z);
    let travel = (gap - CHASE_REACH).clamp(0.0, step);
    if travel <= 0.0 {
        return position;
    }
    let ratio = travel / gap;
    Position {
        x: position.x + (target.x - position.x) * ratio,
        y: position.y + (target.y - position.y) * ratio,
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
/// run speed), then transition to Idle.
fn return_to_spawn_system(
    time: Res<Time>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    mut query: Query<
        (Entity, &mut Position, &SpawnPoint, &mut CreatureState),
        Without<GridDormant>,
    >,
) {
    let dt = time.delta_secs();
    for (entity, mut pos, spawn, mut state) in &mut query {
        if *state != CreatureState::Returning {
            continue;
        }
        let speed = spawn.speed_run * BASE_RUN_SPEED;
        let (reached, new_x, new_z) = move_toward(pos.x, pos.z, spawn.x, spawn.z, speed * dt);
        let next_position = Position {
            x: new_x,
            y: if reached { spawn.y } else { pos.y },
            z: new_z,
        };
        if *pos != next_position {
            *pos = next_position;
            mark_interest_position_changed(dirty_queue.as_deref(), entity);
        }
        if reached {
            *state = CreatureState::Idle;
        }
    }
}

#[cfg(test)]
#[path = "creature_ai_tests.rs"]
mod tests;
