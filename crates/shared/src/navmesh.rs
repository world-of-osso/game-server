//! Navigation mesh types for server-side pathfinding.
//!
//! The navmesh is pre-generated from ADT terrain heightmaps and WMO building
//! collision. At runtime, the server loads tile-based navmesh data and uses it
//! for pathfinding (A* on polygons), line-of-sight checks, and movement
//! validation.
//!
//! Navmesh generation is an offline tool (similar to MaNGOS mmaps_generator).
//! This module defines the runtime data model.
//!
//! Ref: AzerothCore `MoveMap/`, Recast/Detour navmesh format.

use serde::{Deserialize, Serialize};

/// A 3D position in world space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NavPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl NavPoint {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &NavPoint) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn distance_2d(&self, other: &NavPoint) -> f32 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        (dx * dx + dz * dz).sqrt()
    }
}

/// A convex polygon in the navmesh. Entities can walk anywhere within it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavPolygon {
    /// Unique polygon ID within the tile.
    pub id: u32,
    /// Vertex indices into the tile's vertex list.
    pub vertices: Vec<u16>,
    /// Indices of adjacent polygons (for graph traversal). One per edge.
    /// `usize::MAX` = no neighbor (boundary edge).
    pub neighbors: Vec<usize>,
    /// Surface flags (walkable, water, steep, etc.).
    pub flags: NavFlags,
}

/// Surface type flags for navmesh polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NavFlags {
    /// Standard walkable ground.
    pub walkable: bool,
    /// Water surface (swimming).
    pub water: bool,
    /// Too steep to walk (blocked).
    pub steep: bool,
    /// Indoor area (WMO interior).
    pub indoor: bool,
}

/// A single navmesh tile covering a fixed-size area of the world.
///
/// The world is divided into a grid of tiles (typically 533.33y × 533.33y,
/// matching WoW's ADT tile size). Each tile contains polygons and vertices
/// from both terrain heightmap and WMO collision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavTile {
    /// Map ID (0 = Eastern Kingdoms, 1 = Kalimdor, etc.).
    pub map_id: u16,
    /// Tile X coordinate in the grid.
    pub tile_x: u16,
    /// Tile Y coordinate in the grid.
    pub tile_y: u16,
    /// Vertices (positions) used by polygons in this tile.
    pub vertices: Vec<NavPoint>,
    /// Convex polygons that make up the walkable surface.
    pub polygons: Vec<NavPolygon>,
}

/// Tile size in world units (yards). Matches WoW's ADT tile size.
pub const TILE_SIZE: f32 = 533.333_3;

impl NavTile {
    /// Find the polygon containing a point (2D projection, ignoring Y).
    /// Returns the polygon ID or None if the point is outside all polygons.
    ///
    /// This is a simplified point-in-polygon check; a real implementation
    /// would use a spatial index (grid or BVH).
    /// Find the polygon index containing a point (2D projection, ignoring Y).
    /// Returns the array index (not polygon ID) or None.
    pub fn find_polygon(&self, point: &NavPoint) -> Option<usize> {
        self.polygons
            .iter()
            .position(|poly| poly.flags.walkable && self.point_in_polygon(poly, point))
    }

    /// Simple 2D point-in-convex-polygon test using cross products.
    fn point_in_polygon(&self, poly: &NavPolygon, point: &NavPoint) -> bool {
        let n = poly.vertices.len();
        if n < 3 {
            return false;
        }
        let mut positive = 0u32;
        let mut negative = 0u32;
        for i in 0..n {
            let v0 = &self.vertices[poly.vertices[i] as usize];
            let v1 = &self.vertices[poly.vertices[(i + 1) % n] as usize];
            let cross = (v1.x - v0.x) * (point.z - v0.z) - (v1.z - v0.z) * (point.x - v0.x);
            if cross > 0.0 {
                positive += 1;
            } else if cross < 0.0 {
                negative += 1;
            }
        }
        positive == 0 || negative == 0
    }

    /// Get the height at a 2D position by interpolating within the containing polygon.
    /// Returns None if outside all polygons.
    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        let point = NavPoint::new(x, 0.0, z);
        let idx = self.find_polygon(&point)?;
        let poly = &self.polygons[idx];
        if poly.vertices.is_empty() {
            return None;
        }
        // Average Y of polygon vertices as a simple height estimate
        let sum_y: f32 = poly
            .vertices
            .iter()
            .map(|&vi| self.vertices[vi as usize].y)
            .sum();
        Some(sum_y / poly.vertices.len() as f32)
    }
}

/// World-to-tile coordinate conversion.
pub fn world_to_tile(x: f32, z: f32) -> (u16, u16) {
    // WoW coordinates: center of the world is (32, 32) in tile space
    let tile_x = (32.0 - x / TILE_SIZE) as u16;
    let tile_y = (32.0 - z / TILE_SIZE) as u16;
    (tile_x, tile_y)
}

// --- A* pathfinding ---

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// A path result: sequence of world-space waypoints.
#[derive(Debug, Clone, PartialEq)]
pub struct NavPath {
    pub waypoints: Vec<NavPoint>,
}

/// No-neighbor sentinel for navmesh polygon edges.
pub const NO_NEIGHBOR: usize = usize::MAX;

/// A* open-set entry.
#[derive(Debug, Clone, Copy)]
struct AStarNode {
    index: usize,
    f_cost: f32,
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl Eq for AStarNode {}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl NavTile {
    /// Compute the centroid (center point) of a polygon.
    pub fn polygon_center(&self, poly: &NavPolygon) -> NavPoint {
        let n = poly.vertices.len() as f32;
        let (sx, sy, sz) = poly
            .vertices
            .iter()
            .fold((0.0, 0.0, 0.0), |(x, y, z), &vi| {
                let v = &self.vertices[vi as usize];
                (x + v.x, y + v.y, z + v.z)
            });
        NavPoint::new(sx / n, sy / n, sz / n)
    }

    /// Find a path between two points using A* on the polygon graph.
    ///
    /// Returns a sequence of waypoints (polygon centers) from start to end.
    /// Returns `None` if no path exists (disconnected polygons or points
    /// outside the navmesh).
    pub fn find_path(&self, start: &NavPoint, end: &NavPoint) -> Option<NavPath> {
        let start_idx = self.find_polygon(start)?;
        let end_idx = self.find_polygon(end)?;

        if start_idx == end_idx {
            return Some(NavPath {
                waypoints: vec![*start, *end],
            });
        }

        let came_from = self.astar_search(start_idx, end_idx, start)?;
        Some(self.reconstruct_path(start, end, &came_from, start_idx, end_idx))
    }

    fn astar_search(&self, start: usize, end: usize, start_pos: &NavPoint) -> Option<Vec<usize>> {
        let n = self.polygons.len();
        let mut g = vec![f32::MAX; n];
        let mut from = vec![NO_NEIGHBOR; n];
        let mut closed = vec![false; n];
        let goal = self.polygon_center(&self.polygons[end]);

        g[start] = 0.0;
        let mut open = BinaryHeap::new();
        open.push(AStarNode {
            index: start,
            f_cost: start_pos.distance_2d(&goal),
        });

        while let Some(cur) = open.pop() {
            if cur.index == end {
                return Some(from);
            }
            if closed[cur.index] {
                continue;
            }
            closed[cur.index] = true;
            self.expand_neighbors(cur.index, &mut g, &mut from, &closed, &mut open, &goal);
        }
        None
    }

    fn expand_neighbors(
        &self,
        ci: usize,
        g: &mut [f32],
        from: &mut [usize],
        closed: &[bool],
        open: &mut BinaryHeap<AStarNode>,
        goal: &NavPoint,
    ) {
        let cc = self.polygon_center(&self.polygons[ci]);
        for &ni in &self.polygons[ci].neighbors {
            if ni == NO_NEIGHBOR || ni >= g.len() || closed[ni] || !self.polygons[ni].flags.walkable
            {
                continue;
            }
            let nc = self.polygon_center(&self.polygons[ni]);
            let tg = g[ci] + cc.distance_2d(&nc);
            if tg < g[ni] {
                g[ni] = tg;
                from[ni] = ci;
                open.push(AStarNode {
                    index: ni,
                    f_cost: tg + nc.distance_2d(goal),
                });
            }
        }
    }

    fn reconstruct_path(
        &self,
        start: &NavPoint,
        end: &NavPoint,
        from: &[usize],
        start_idx: usize,
        end_idx: usize,
    ) -> NavPath {
        let mut chain = vec![end_idx];
        let mut cur = end_idx;
        while cur != start_idx {
            cur = from[cur];
            if cur == NO_NEIGHBOR {
                break;
            }
            chain.push(cur);
        }
        chain.reverse();

        let mut waypoints = vec![*start];
        for &idx in &chain[1..chain.len().saturating_sub(1)] {
            waypoints.push(self.polygon_center(&self.polygons[idx]));
        }
        waypoints.push(*end);
        NavPath { waypoints }
    }
}

// --- Chase behavior ---

/// How often a chasing mob recalculates its path (seconds).
const CHASE_REPATH_INTERVAL: f32 = 1.0;
/// Distance threshold: if target moved less than this, skip repath.
const CHASE_REPATH_THRESHOLD: f32 = 3.0;

/// Chase state for a mob following a target.
///
/// The mob moves along waypoints toward the target. When the target moves
/// significantly or the repath timer expires, the path is recalculated.
#[derive(Debug, Clone, PartialEq)]
pub struct ChaseState {
    /// Target entity bits.
    pub target: u64,
    /// Current path to follow.
    pub path: Option<NavPath>,
    /// Index of the next waypoint to move toward.
    pub waypoint_index: usize,
    /// Time until next path recalculation.
    pub repath_timer: f32,
    /// Last known target position (for movement detection).
    pub last_target_pos: NavPoint,
    /// Movement speed in yards/second.
    pub speed: f32,
}

/// Result of a chase tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChaseTickResult {
    /// Move toward this position this frame.
    MoveTo(NavPoint),
    /// Target reached (within melee range).
    Reached,
    /// No path to target (stuck or unreachable).
    NoPath,
    /// Need to recalculate path (target moved or timer expired).
    NeedRepath,
    /// Mob exceeded leash distance — should evade (return to spawn, reset).
    Leashed,
}

impl ChaseState {
    pub fn new(target: u64, target_pos: NavPoint, speed: f32) -> Self {
        Self {
            target,
            path: None,
            waypoint_index: 0,
            repath_timer: 0.0,
            last_target_pos: target_pos,
            speed,
        }
    }

    /// Set a new path to follow.
    pub fn set_path(&mut self, path: NavPath, target_pos: NavPoint) {
        self.path = Some(path);
        self.waypoint_index = 0;
        self.repath_timer = CHASE_REPATH_INTERVAL;
        self.last_target_pos = target_pos;
    }

    /// Tick the chase state. Returns what the mob should do this frame.
    ///
    /// `current_pos`: mob's current position.
    /// `target_pos`: target's current position.
    /// `melee_range`: distance at which the mob considers itself "reached".
    pub fn tick(
        &mut self,
        dt: f32,
        current_pos: &NavPoint,
        target_pos: &NavPoint,
        melee_range: f32,
    ) -> ChaseTickResult {
        // Already in range?
        if current_pos.distance_2d(target_pos) <= melee_range {
            return ChaseTickResult::Reached;
        }

        // Check if repath needed
        self.repath_timer -= dt;
        let target_moved = target_pos.distance_2d(&self.last_target_pos) > CHASE_REPATH_THRESHOLD;
        if self.path.is_none() || self.repath_timer <= 0.0 || target_moved {
            return ChaseTickResult::NeedRepath;
        }

        // Follow current path
        let path = self.path.as_ref().unwrap();
        if self.waypoint_index >= path.waypoints.len() {
            return ChaseTickResult::NeedRepath;
        }

        let wp = path.waypoints[self.waypoint_index];
        let dist = current_pos.distance_2d(&wp);
        if dist < self.speed * dt {
            self.waypoint_index += 1;
            if self.waypoint_index >= path.waypoints.len() {
                return ChaseTickResult::Reached;
            }
            return ChaseTickResult::MoveTo(path.waypoints[self.waypoint_index]);
        }

        ChaseTickResult::MoveTo(wp)
    }

    /// Tick with leash check: if the mob is too far from spawn, return Leashed.
    ///
    /// `spawn_origin`: the mob's spawn point.
    /// `max_leash_distance`: maximum distance from spawn before evading.
    ///
    /// On `Leashed`, the server should: clear threat, restore HP, return to spawn.
    pub fn tick_with_leash(
        &mut self,
        dt: f32,
        current_pos: &NavPoint,
        target_pos: &NavPoint,
        melee_range: f32,
        spawn_origin: &NavPoint,
        max_leash_distance: f32,
    ) -> ChaseTickResult {
        if current_pos.distance_2d(spawn_origin) > max_leash_distance {
            return ChaseTickResult::Leashed;
        }
        self.tick(dt, current_pos, target_pos, melee_range)
    }
}

/// State for a mob returning to its spawn point after evading.
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnToSpawn {
    pub path: Option<NavPath>,
    pub waypoint_index: usize,
    pub speed: f32,
}

impl ReturnToSpawn {
    pub fn new(speed: f32) -> Self {
        Self {
            path: None,
            waypoint_index: 0,
            speed,
        }
    }

    /// Set the return path.
    pub fn set_path(&mut self, path: NavPath) {
        self.path = Some(path);
        self.waypoint_index = 0;
    }

    /// Tick the return movement. Returns the next position to move toward,
    /// or `None` if the mob has reached its spawn point.
    pub fn tick(&mut self, dt: f32, current_pos: &NavPoint) -> Option<NavPoint> {
        let path = self.path.as_ref()?;
        if self.waypoint_index >= path.waypoints.len() {
            return None;
        }
        let wp = path.waypoints[self.waypoint_index];
        if current_pos.distance_2d(&wp) < self.speed * dt {
            self.waypoint_index += 1;
            if self.waypoint_index >= path.waypoints.len() {
                return None; // arrived
            }
            return Some(path.waypoints[self.waypoint_index]);
        }
        Some(wp)
    }
}

// --- Patrol paths ---

/// A waypoint in a patrol path with an optional delay.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PatrolWaypoint {
    pub position: NavPoint,
    /// Seconds to wait at this waypoint before moving to the next.
    pub delay: f32,
}

/// A predefined patrol path that a creature follows in a loop.
///
/// Loaded from AzerothCore `waypoint_data` table. The creature walks
/// from waypoint to waypoint, pausing at each for the specified delay,
/// then loops back to the start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatrolPath {
    pub waypoints: Vec<PatrolWaypoint>,
}

/// State for a creature following a patrol path.
#[derive(Debug, Clone, PartialEq)]
pub struct PatrolState {
    /// Current waypoint index.
    pub index: usize,
    /// Remaining delay at current waypoint (0 = moving).
    pub delay_remaining: f32,
    /// Movement speed in yards/second.
    pub speed: f32,
}

/// What the patrol system should do this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatrolTickResult {
    /// Move toward this position.
    MoveTo(NavPoint),
    /// Waiting at a waypoint.
    Waiting,
}

impl PatrolState {
    pub fn new(speed: f32) -> Self {
        Self {
            index: 0,
            delay_remaining: 0.0,
            speed,
        }
    }

    /// Tick the patrol. Returns whether to move or wait.
    pub fn tick(&mut self, dt: f32, current_pos: &NavPoint, path: &PatrolPath) -> PatrolTickResult {
        if path.waypoints.is_empty() {
            return PatrolTickResult::Waiting;
        }

        // Waiting at current waypoint
        if self.delay_remaining > 0.0 {
            self.delay_remaining -= dt;
            return PatrolTickResult::Waiting;
        }

        let wp = &path.waypoints[self.index];
        let dist = current_pos.distance_2d(&wp.position);

        // Reached current waypoint — start delay and advance
        if dist < self.speed * dt {
            self.delay_remaining = wp.delay;
            self.index = (self.index + 1) % path.waypoints.len();
            if self.delay_remaining > 0.0 {
                return PatrolTickResult::Waiting;
            }
            // No delay: immediately head to next
            return PatrolTickResult::MoveTo(path.waypoints[self.index].position);
        }

        PatrolTickResult::MoveTo(wp.position)
    }
}

// --- Fear / knockback ---

/// Type of forced movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcedMoveType {
    /// Fear: run in a random direction, changing direction periodically.
    Fear,
    /// Knockback: single push in a fixed direction.
    Knockback,
}

/// Compute a fear/knockback target point from an origin in a given direction.
///
/// `angle` is in radians (0 = +X, π/2 = +Z).
/// `distance` is how far to push in yards.
/// The result is clamped to the navmesh — if the target is outside all
/// walkable polygons, it's pulled back to the navmesh boundary by
/// binary-searching along the ray.
pub fn forced_move_target(
    tile: &NavTile,
    origin: &NavPoint,
    angle: f32,
    distance: f32,
) -> NavPoint {
    let dx = angle.cos() * distance;
    let dz = angle.sin() * distance;
    let target = NavPoint::new(origin.x + dx, origin.y, origin.z + dz);

    // If the target is on the navmesh, use it directly
    if tile.find_polygon(&target).is_some() {
        return target;
    }

    // Binary search to find the farthest walkable point along the ray
    clamp_to_navmesh(tile, origin, &target)
}

/// Binary search along origin→target to find the farthest point on the navmesh.
fn clamp_to_navmesh(tile: &NavTile, origin: &NavPoint, target: &NavPoint) -> NavPoint {
    let mut lo = 0.0_f32;
    let mut hi = 1.0_f32;
    let mut best = *origin;

    for _ in 0..8 {
        let mid = (lo + hi) / 2.0;
        let test = NavPoint::new(
            origin.x + (target.x - origin.x) * mid,
            origin.y,
            origin.z + (target.z - origin.z) * mid,
        );
        if tile.find_polygon(&test).is_some() {
            best = test;
            lo = mid;
        } else {
            hi = mid;
        }
    }

    best
}

/// State for an entity under forced movement (fear or knockback).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForcedMoveState {
    pub move_type: ForcedMoveType,
    /// Target position to move toward.
    pub target: NavPoint,
    /// Time remaining on the effect (seconds).
    pub duration: f32,
    /// Movement speed during forced movement (yards/sec).
    pub speed: f32,
}

impl ForcedMoveState {
    /// Create a knockback: single push toward a fixed target.
    pub fn knockback(target: NavPoint, duration: f32, speed: f32) -> Self {
        Self {
            move_type: ForcedMoveType::Knockback,
            target,
            duration,
            speed,
        }
    }

    /// Create a fear: run toward a random target, duration-limited.
    pub fn fear(target: NavPoint, duration: f32, speed: f32) -> Self {
        Self {
            move_type: ForcedMoveType::Fear,
            target,
            duration,
            speed,
        }
    }

    /// Tick the forced movement. Returns `true` if the effect has expired.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.duration -= dt;
        self.duration <= 0.0
    }

    /// Whether the entity has reached its forced-move target.
    pub fn reached_target(&self, current_pos: &NavPoint) -> bool {
        current_pos.distance_2d(&self.target) < 1.0
    }
}

// --- Charge / leap ---

/// Type of direct-line movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectMoveType {
    /// Charge: rush to a target (Warrior Charge). Stops at target.
    Charge,
    /// Leap: jump to a location (Heroic Leap). Lands at destination.
    Leap,
}

/// State for a direct-line movement that ignores navmesh pathing.
///
/// The entity moves in a straight line from start to destination at
/// a fixed speed. No pathfinding — passes through obstacles.
/// The server validates the destination is on walkable ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectMoveState {
    pub move_type: DirectMoveType,
    pub start: NavPoint,
    pub destination: NavPoint,
    pub speed: f32,
    pub elapsed: f32,
    pub total_time: f32,
}

impl DirectMoveState {
    /// Create a charge toward a target entity's position.
    pub fn charge(start: NavPoint, target: NavPoint, speed: f32) -> Self {
        let dist = start.distance_2d(&target);
        let total = if speed > 0.0 { dist / speed } else { 0.0 };
        Self {
            move_type: DirectMoveType::Charge,
            start,
            destination: target,
            speed,
            elapsed: 0.0,
            total_time: total,
        }
    }

    /// Create a leap to a ground location.
    pub fn leap(start: NavPoint, destination: NavPoint, travel_time: f32) -> Self {
        Self {
            move_type: DirectMoveType::Leap,
            start,
            destination,
            speed: 0.0,
            elapsed: 0.0,
            total_time: travel_time,
        }
    }

    /// Current interpolated position along the direct line.
    pub fn current_position(&self) -> NavPoint {
        let t = if self.total_time > 0.0 {
            (self.elapsed / self.total_time).min(1.0)
        } else {
            1.0
        };
        NavPoint::new(
            self.start.x + (self.destination.x - self.start.x) * t,
            self.start.y + (self.destination.y - self.start.y) * t,
            self.start.z + (self.destination.z - self.start.z) * t,
        )
    }

    /// Tick the movement. Returns `true` when the entity has arrived.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        self.elapsed >= self.total_time
    }

    /// Whether the movement is complete.
    pub fn is_complete(&self) -> bool {
        self.elapsed >= self.total_time
    }
}

#[cfg(test)]
#[path = "navmesh_tests.rs"]
mod tests;
