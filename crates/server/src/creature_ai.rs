use bevy::prelude::*;
use shared::components::Position;

use crate::world_data::{WaypointPath, WorldData};

/// Walk speed in world-units per second (base WoW walk speed).
const BASE_WALK_SPEED: f32 = 2.5;
/// Idle pause range in seconds after reaching a wander destination.
const IDLE_MIN_SECS: f32 = 2.0;
const IDLE_MAX_SECS: f32 = 5.0;

pub struct CreatureAiPlugin;

impl Plugin for CreatureAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (wander_system, waypoint_system));
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
#[derive(Component, Debug)]
pub struct WanderState {
    pub phase: WanderPhase,
    pub target_x: f32,
    pub target_y: f32,
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

// -- Wander logic --

fn wander_system(
    time: Res<Time>,
    mut query: Query<(&mut Position, &SpawnPoint, &mut WanderState)>,
) {
    let dt = time.delta_secs();
    for (mut pos, spawn, mut state) in &mut query {
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
    let (tx, ty) = pick_wander_point(spawn.x, spawn.y, spawn.wander_distance, state.seed);
    state.target_x = tx;
    state.target_y = ty;
    state.phase = WanderPhase::Walking;
    state.seed = state.seed.wrapping_add(1);
}

fn tick_walk(pos: &mut Position, state: &mut WanderState, spawn: &SpawnPoint, dt: f32) {
    let speed = spawn.speed_walk * BASE_WALK_SPEED;
    let (reached, new_x, new_y) = move_toward(pos.x, pos.y, state.target_x, state.target_y, speed * dt);
    pos.x = new_x;
    pos.y = new_y;
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
    mut query: Query<(&mut Position, &mut WaypointState)>,
) {
    let dt = time.delta_secs();
    for (mut pos, mut state) in &mut query {
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
    let speed = state.speed_walk * BASE_WALK_SPEED;
    let (reached, nx, ny) = move_toward(pos.x, pos.y, point.position_x, point.position_y, speed * dt);
    pos.x = nx;
    pos.y = ny;
    if reached {
        pos.z = point.position_z;
        advance_waypoint(state, point.delay as f32, path.points.len());
    }
}

fn advance_waypoint(state: &mut WaypointState, delay_secs: f32, path_len: usize) {
    state.delay_remaining = delay_secs;
    state.current_index = (state.current_index + 1) % path_len;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_point_within_radius() {
        for seed in 0..100 {
            let (px, py) = pick_wander_point(10.0, 20.0, 5.0, seed);
            let dx = px - 10.0;
            let dy = py - 20.0;
            let dist = (dx * dx + dy * dy).sqrt();
            assert!(dist <= 5.01, "seed {seed}: dist {dist} exceeds radius 5.0");
        }
    }

    #[test]
    fn pick_point_zero_radius_stays_at_center() {
        let (px, py) = pick_wander_point(5.0, 5.0, 0.0, 42);
        assert!((px - 5.0).abs() < 0.001);
        assert!((py - 5.0).abs() < 0.001);
    }

    #[test]
    fn move_toward_reaches_close_target() {
        let (reached, x, y) = move_toward(0.0, 0.0, 0.5, 0.0, 1.0);
        assert!(reached);
        assert!((x - 0.5).abs() < 0.001);
        assert!(y.abs() < 0.001);
    }

    #[test]
    fn move_toward_partial_step() {
        let (reached, x, y) = move_toward(0.0, 0.0, 10.0, 0.0, 3.0);
        assert!(!reached);
        assert!((x - 3.0).abs() < 0.001);
        assert!(y.abs() < 0.001);
    }

    #[test]
    fn move_toward_diagonal() {
        let (reached, x, y) = move_toward(0.0, 0.0, 3.0, 4.0, 2.5);
        assert!(!reached);
        // Should move 2.5 units along the (3,4) direction (length 5)
        assert!((x - 1.5).abs() < 0.001);
        assert!((y - 2.0).abs() < 0.001);
    }

    #[test]
    fn lcg_produces_values_in_range() {
        for seed in 0..1000 {
            let v = lcg_float(seed);
            assert!(v >= 0.0 && v < 1.0, "seed {seed}: value {v} out of range");
        }
    }

    #[test]
    fn random_idle_in_range() {
        for seed in 0..100 {
            let t = random_idle_secs(seed);
            assert!(t >= IDLE_MIN_SECS && t <= IDLE_MAX_SECS, "seed {seed}: idle {t}");
        }
    }

    #[test]
    fn advance_waypoint_wraps() {
        let mut state = WaypointState {
            creature_guid: 1,
            current_index: 3,
            delay_remaining: 0.0,
            speed_walk: 1.0,
        };
        advance_waypoint(&mut state, 2.0, 4);
        assert_eq!(state.current_index, 0); // wraps from 3 -> 0
        assert_eq!(state.delay_remaining, 2.0);
    }
}
