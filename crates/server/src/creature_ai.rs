use bevy::prelude::*;
use shared::components::{Health, Position, ThreatTable};

use crate::world_data::{WaypointPath, WorldData};

/// Walk speed in world-units per second (base WoW walk speed).
const BASE_WALK_SPEED: f32 = 2.5;
/// Maximum chase distance from spawn before evading (yards).
const LEASH_DISTANCE: f32 = 40.0;
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
                waypoint_system,
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
    pub creature_guid: u32,
    pub current_index: usize,
    pub delay_remaining: f32,
    pub speed_walk: f32,
}

/// High-level creature behavior state. Overrides wander/waypoint when non-idle.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub enum CreatureState {
    /// Normal behavior — wander or waypoint patrol runs as usual.
    #[default]
    Idle,
    /// Chasing a threat target. Wander/waypoint AI pauses.
    /// `target` is the u64 entity ID from the ThreatTable.
    #[cfg_attr(not(test), expect(dead_code, reason = "constructed by combat system (upcoming PLAN item)"))]
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
    mut query: Query<(
        &mut Position,
        &SpawnPoint,
        &mut WanderState,
        Option<&CreatureState>,
    )>,
) {
    let dt = time.delta_secs();
    for (mut pos, spawn, mut state, creature_state) in &mut query {
        if matches!(creature_state, Some(s) if !s.is_idle()) {
            continue;
        }
        match state.phase {
            WanderPhase::Idle => tick_idle(&mut state, dt, spawn),
            WanderPhase::Walking => tick_walk(&mut pos, &mut state, spawn, dt),
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

fn tick_walk(pos: &mut Position, state: &mut WanderState, spawn: &SpawnPoint, dt: f32) {
    let speed = spawn.speed_walk * BASE_WALK_SPEED;
    let (reached, new_x, new_z) =
        move_toward(pos.x, pos.z, state.target_x, state.target_z, speed * dt);
    pos.x = new_x;
    pos.z = new_z;
    if reached {
        state.phase = WanderPhase::Idle;
        state.idle_remaining = random_idle_secs(state.seed);
        state.seed = state.seed.wrapping_add(1);
    }
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
    mut query: Query<(&mut Position, &mut WaypointState, Option<&CreatureState>)>,
) {
    let dt = time.delta_secs();
    for (mut pos, mut state, creature_state) in &mut query {
        if matches!(creature_state, Some(s) if !s.is_idle()) {
            continue;
        }
        let Some(path) = world_data.waypoints.get(&state.creature_guid) else {
            continue;
        };
        if path.points.is_empty() {
            continue;
        }
        tick_waypoint(&mut pos, &mut state, path, dt);
    }
}

fn tick_waypoint(pos: &mut Position, state: &mut WaypointState, path: &WaypointPath, dt: f32) {
    if state.delay_remaining > 0.0 {
        state.delay_remaining -= dt;
        return;
    }
    let point = &path.points[state.current_index % path.points.len()];
    // Waypoint data is WoW coords; convert target to Bevy for movement.
    let (tx, ty, tz) =
        crate::terrain_height::wow_to_bevy(point.position_x, point.position_y, point.position_z);
    let speed = state.speed_walk * BASE_WALK_SPEED;
    let (reached, nx, nz) = move_toward(pos.x, pos.z, tx, tz, speed * dt);
    pos.x = nx;
    pos.z = nz;
    if reached {
        pos.y = ty;
        advance_waypoint(state, point.delay as f32, path.points.len());
    }
}

fn advance_waypoint(state: &mut WaypointState, delay_secs: f32, path_len: usize) {
    state.delay_remaining = delay_secs;
    state.current_index = (state.current_index + 1) % path_len;
}

// -- Chase logic --

/// Move pursuing creatures toward their target each tick.
fn chase_system(
    time: Res<Time>,
    mut chasers: Query<(&mut Position, &SpawnPoint, &CreatureState)>,
    targets: Query<&Position, Without<SpawnPoint>>,
) {
    let dt = time.delta_secs();
    for (mut pos, spawn, state) in &mut chasers {
        let CreatureState::Pursuing { target } = *state else {
            continue;
        };
        let target_entity = Entity::from_bits(target);
        let Ok(target_pos) = targets.get(target_entity) else {
            continue; // target despawned or invalid
        };
        let speed = spawn.speed_walk * BASE_WALK_SPEED;
        let (_reached, new_x, new_z) =
            move_toward(pos.x, pos.z, target_pos.x, target_pos.z, speed * dt);
        pos.x = new_x;
        pos.z = new_z;
    }
}

/// Check if position exceeds leash distance from spawn (2D, ignoring height).
fn beyond_leash(pos: &Position, spawn: &SpawnPoint) -> bool {
    let dx = pos.x - spawn.x;
    let dz = pos.z - spawn.z;
    let dist_sq = dx * dx + dz * dz;
    dist_sq > LEASH_DISTANCE * LEASH_DISTANCE
}

type LeashQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static SpawnPoint,
        &'static mut CreatureState,
        Option<&'static mut ThreatTable>,
        Option<&'static mut Health>,
    ),
>;

/// Evade pursuing creatures that exceed leash distance from spawn.
/// Clears threat, restores health, transitions to Returning.
fn leash_check_system(mut query: LeashQuery) {
    for (pos, spawn, mut state, threat, health) in &mut query {
        if !matches!(*state, CreatureState::Pursuing { .. }) {
            continue;
        }
        if !beyond_leash(pos, spawn) {
            continue;
        }
        *state = CreatureState::Returning;
        if let Some(mut threat) = threat {
            threat.reset();
        }
        if let Some(mut health) = health {
            health.current = health.max;
        }
    }
}

/// Walk returning creatures back to their spawn point, then transition to Idle.
fn return_to_spawn_system(
    time: Res<Time>,
    mut query: Query<(&mut Position, &SpawnPoint, &mut CreatureState)>,
) {
    let dt = time.delta_secs();
    for (mut pos, spawn, mut state) in &mut query {
        if *state != CreatureState::Returning {
            continue;
        }
        let speed = spawn.speed_walk * BASE_WALK_SPEED;
        let (reached, new_x, new_z) =
            move_toward(pos.x, pos.z, spawn.x, spawn.z, speed * dt);
        pos.x = new_x;
        pos.z = new_z;
        if reached {
            pos.y = spawn.y;
            *state = CreatureState::Idle;
        }
    }
}


#[cfg(test)]
#[path = "creature_ai_tests.rs"]
mod tests;
