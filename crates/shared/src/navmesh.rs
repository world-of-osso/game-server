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

use std::collections::BinaryHeap;
use std::cmp::Ordering;

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
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
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
        let (sx, sy, sz) = poly.vertices.iter().fold((0.0, 0.0, 0.0), |(x, y, z), &vi| {
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
            return Some(NavPath { waypoints: vec![*start, *end] });
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
        open.push(AStarNode { index: start, f_cost: start_pos.distance_2d(&goal) });

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
            if ni == NO_NEIGHBOR || ni >= g.len() || closed[ni] || !self.polygons[ni].flags.walkable {
                continue;
            }
            let nc = self.polygon_center(&self.polygons[ni]);
            let tg = g[ci] + cc.distance_2d(&nc);
            if tg < g[ni] {
                g[ni] = tg;
                from[ni] = ci;
                open.push(AStarNode { index: ni, f_cost: tg + nc.distance_2d(goal) });
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
mod tests {
    use super::*;

    fn sample_tile() -> NavTile {
        // Simple square tile with one walkable quad polygon
        NavTile {
            map_id: 0,
            tile_x: 32,
            tile_y: 32,
            vertices: vec![
                NavPoint::new(0.0, 100.0, 0.0),
                NavPoint::new(10.0, 100.0, 0.0),
                NavPoint::new(10.0, 100.0, 10.0),
                NavPoint::new(0.0, 100.0, 10.0),
            ],
            polygons: vec![NavPolygon {
                id: 0,
                vertices: vec![0, 1, 2, 3],
                neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
                flags: NavFlags {
                    walkable: true,
                    ..Default::default()
                },
            }],
        }
    }

    #[test]
    fn nav_point_distance() {
        let a = NavPoint::new(0.0, 0.0, 0.0);
        let b = NavPoint::new(3.0, 4.0, 0.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn nav_point_distance_2d() {
        let a = NavPoint::new(0.0, 100.0, 0.0);
        let b = NavPoint::new(3.0, 200.0, 4.0);
        assert!((a.distance_2d(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn find_polygon_inside() {
        let tile = sample_tile();
        let point = NavPoint::new(5.0, 0.0, 5.0);
        assert_eq!(tile.find_polygon(&point), Some(0));
    }

    #[test]
    fn find_polygon_outside() {
        let tile = sample_tile();
        let point = NavPoint::new(20.0, 0.0, 20.0);
        assert_eq!(tile.find_polygon(&point), None);
    }

    #[test]
    fn height_at_inside_polygon() {
        let tile = sample_tile();
        let height = tile.height_at(5.0, 5.0).unwrap();
        assert!((height - 100.0).abs() < 0.001);
    }

    #[test]
    fn height_at_outside_returns_none() {
        let tile = sample_tile();
        assert!(tile.height_at(50.0, 50.0).is_none());
    }

    #[test]
    fn world_to_tile_center() {
        let (tx, ty) = world_to_tile(0.0, 0.0);
        assert_eq!(tx, 32);
        assert_eq!(ty, 32);
    }

    #[test]
    fn nav_flags_default() {
        let flags = NavFlags::default();
        assert!(!flags.walkable);
        assert!(!flags.water);
        assert!(!flags.steep);
        assert!(!flags.indoor);
    }

    #[test]
    fn tile_with_water_and_walkable() {
        let tile = NavTile {
            map_id: 0,
            tile_x: 0,
            tile_y: 0,
            vertices: vec![
                NavPoint::new(0.0, 50.0, 0.0),
                NavPoint::new(10.0, 50.0, 0.0),
                NavPoint::new(10.0, 50.0, 10.0),
            ],
            polygons: vec![
                NavPolygon {
                    id: 0,
                    vertices: vec![0, 1, 2],
                    neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
                    flags: NavFlags {
                        walkable: true,
                        water: true,
                        ..Default::default()
                    },
                },
            ],
        };
        let poly = tile.find_polygon(&NavPoint::new(3.0, 0.0, 3.0));
        assert_eq!(poly, Some(0));
        assert!(tile.polygons[0].flags.water);
    }

    // --- A* pathfinding tests ---

    fn path_tile() -> NavTile {
        // Three connected triangles in a line: poly 1 → 2 → 3
        //   v0---v1---v3---v5
        //   |  1  |  2  |  3 |
        //   v2---v4---v6---v7
        let w = NavFlags { walkable: true, ..Default::default() };
        NavTile {
            map_id: 0,
            tile_x: 0,
            tile_y: 0,
            vertices: vec![
                NavPoint::new(0.0, 0.0, 0.0),   // 0
                NavPoint::new(10.0, 0.0, 0.0),  // 1
                NavPoint::new(0.0, 0.0, 10.0),  // 2
                NavPoint::new(20.0, 0.0, 0.0),  // 3
                NavPoint::new(10.0, 0.0, 10.0), // 4
                NavPoint::new(30.0, 0.0, 0.0),  // 5
                NavPoint::new(20.0, 0.0, 10.0), // 6
                NavPoint::new(30.0, 0.0, 10.0), // 7
            ],
            polygons: vec![
                NavPolygon { id: 0, vertices: vec![0, 1, 4, 2], neighbors: vec![NO_NEIGHBOR, 1, NO_NEIGHBOR, NO_NEIGHBOR], flags: w },
                NavPolygon { id: 1, vertices: vec![1, 3, 6, 4], neighbors: vec![NO_NEIGHBOR, 2, NO_NEIGHBOR, 0], flags: w },
                NavPolygon { id: 2, vertices: vec![3, 5, 7, 6], neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR, 1], flags: w },
            ],
        }
    }

    #[test]
    fn path_same_polygon() {
        let tile = path_tile();
        let start = NavPoint::new(2.0, 0.0, 5.0);
        let end = NavPoint::new(8.0, 0.0, 5.0);
        let path = tile.find_path(&start, &end).unwrap();
        assert_eq!(path.waypoints.len(), 2);
        assert_eq!(path.waypoints[0], start);
        assert_eq!(path.waypoints[1], end);
    }

    #[test]
    fn path_across_two_polygons() {
        let tile = path_tile();
        let start = NavPoint::new(2.0, 0.0, 5.0);
        let end = NavPoint::new(15.0, 0.0, 5.0);
        let path = tile.find_path(&start, &end).unwrap();
        assert!(path.waypoints.len() >= 2);
        assert_eq!(*path.waypoints.first().unwrap(), start);
        assert_eq!(*path.waypoints.last().unwrap(), end);
    }

    #[test]
    fn path_across_three_polygons() {
        let tile = path_tile();
        let start = NavPoint::new(2.0, 0.0, 5.0);
        let end = NavPoint::new(28.0, 0.0, 5.0);
        let path = tile.find_path(&start, &end).unwrap();
        assert!(path.waypoints.len() >= 3);
    }

    #[test]
    fn path_outside_returns_none() {
        let tile = path_tile();
        let outside = NavPoint::new(100.0, 0.0, 100.0);
        assert!(tile.find_path(&NavPoint::new(5.0, 0.0, 5.0), &outside).is_none());
    }

    #[test]
    fn path_no_connection_returns_none() {
        // Two disconnected polygons (no neighbors)
        let tile = NavTile {
            map_id: 0,
            tile_x: 0,
            tile_y: 0,
            vertices: vec![
                NavPoint::new(0.0, 0.0, 0.0),
                NavPoint::new(5.0, 0.0, 0.0),
                NavPoint::new(0.0, 0.0, 5.0),
                NavPoint::new(20.0, 0.0, 0.0),
                NavPoint::new(25.0, 0.0, 0.0),
                NavPoint::new(20.0, 0.0, 5.0),
            ],
            polygons: vec![
                NavPolygon {
                    id: 1,
                    vertices: vec![0, 1, 2],
                    neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
                    flags: NavFlags { walkable: true, ..Default::default() },
                },
                NavPolygon {
                    id: 2,
                    vertices: vec![3, 4, 5],
                    neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
                    flags: NavFlags { walkable: true, ..Default::default() },
                },
            ],
        };
        let start = NavPoint::new(2.0, 0.0, 2.0);
        let end = NavPoint::new(22.0, 0.0, 2.0);
        assert!(tile.find_path(&start, &end).is_none());
    }

    // --- Chase tests ---

    #[test]
    fn chase_reached_when_in_range() {
        let mut chase = ChaseState::new(1, NavPoint::new(5.0, 0.0, 5.0), 7.0);
        chase.set_path(
            NavPath { waypoints: vec![NavPoint::new(0.0, 0.0, 0.0), NavPoint::new(5.0, 0.0, 5.0)] },
            NavPoint::new(5.0, 0.0, 5.0),
        );
        let result = chase.tick(0.016, &NavPoint::new(4.0, 0.0, 5.0), &NavPoint::new(5.0, 0.0, 5.0), 5.0);
        assert_eq!(result, ChaseTickResult::Reached);
    }

    #[test]
    fn chase_moves_toward_waypoint() {
        let mut chase = ChaseState::new(1, NavPoint::new(20.0, 0.0, 0.0), 7.0);
        chase.set_path(
            NavPath { waypoints: vec![NavPoint::new(0.0, 0.0, 0.0), NavPoint::new(10.0, 0.0, 0.0), NavPoint::new(20.0, 0.0, 0.0)] },
            NavPoint::new(20.0, 0.0, 0.0),
        );
        let result = chase.tick(0.016, &NavPoint::new(0.0, 0.0, 0.0), &NavPoint::new(20.0, 0.0, 0.0), 5.0);
        assert!(matches!(result, ChaseTickResult::MoveTo(_)));
    }

    #[test]
    fn chase_need_repath_no_path() {
        let mut chase = ChaseState::new(1, NavPoint::new(20.0, 0.0, 0.0), 7.0);
        let result = chase.tick(0.016, &NavPoint::new(0.0, 0.0, 0.0), &NavPoint::new(20.0, 0.0, 0.0), 5.0);
        assert_eq!(result, ChaseTickResult::NeedRepath);
    }

    #[test]
    fn chase_need_repath_target_moved() {
        let mut chase = ChaseState::new(1, NavPoint::new(10.0, 0.0, 0.0), 7.0);
        chase.set_path(
            NavPath { waypoints: vec![NavPoint::new(0.0, 0.0, 0.0), NavPoint::new(10.0, 0.0, 0.0)] },
            NavPoint::new(10.0, 0.0, 0.0),
        );
        // Target moved significantly
        let result = chase.tick(0.016, &NavPoint::new(0.0, 0.0, 0.0), &NavPoint::new(20.0, 0.0, 0.0), 5.0);
        assert_eq!(result, ChaseTickResult::NeedRepath);
    }

    #[test]
    fn chase_need_repath_timer_expired() {
        let mut chase = ChaseState::new(1, NavPoint::new(20.0, 0.0, 0.0), 7.0);
        chase.set_path(
            NavPath { waypoints: vec![NavPoint::new(0.0, 0.0, 0.0), NavPoint::new(20.0, 0.0, 0.0)] },
            NavPoint::new(20.0, 0.0, 0.0),
        );
        // Tick past repath interval
        let result = chase.tick(2.0, &NavPoint::new(0.0, 0.0, 0.0), &NavPoint::new(20.0, 0.0, 0.0), 5.0);
        assert_eq!(result, ChaseTickResult::NeedRepath);
    }

    // --- Leash tests ---

    #[test]
    fn leash_triggers_when_too_far() {
        let mut chase = ChaseState::new(1, NavPoint::new(50.0, 0.0, 0.0), 7.0);
        chase.set_path(
            NavPath { waypoints: vec![NavPoint::new(45.0, 0.0, 0.0), NavPoint::new(50.0, 0.0, 0.0)] },
            NavPoint::new(50.0, 0.0, 0.0),
        );
        let spawn = NavPoint::new(0.0, 0.0, 0.0);
        let current = NavPoint::new(45.0, 0.0, 0.0); // 45y from spawn
        let result = chase.tick_with_leash(
            0.016, &current, &NavPoint::new(50.0, 0.0, 0.0), 5.0, &spawn, 40.0,
        );
        assert_eq!(result, ChaseTickResult::Leashed);
    }

    #[test]
    fn leash_does_not_trigger_within_range() {
        let mut chase = ChaseState::new(1, NavPoint::new(30.0, 0.0, 0.0), 7.0);
        chase.set_path(
            NavPath { waypoints: vec![NavPoint::new(20.0, 0.0, 0.0), NavPoint::new(30.0, 0.0, 0.0)] },
            NavPoint::new(30.0, 0.0, 0.0),
        );
        let spawn = NavPoint::new(0.0, 0.0, 0.0);
        let current = NavPoint::new(20.0, 0.0, 0.0); // 20y from spawn, within 40y leash
        let result = chase.tick_with_leash(
            0.016, &current, &NavPoint::new(30.0, 0.0, 0.0), 5.0, &spawn, 40.0,
        );
        assert!(matches!(result, ChaseTickResult::MoveTo(_)));
    }

    #[test]
    fn return_to_spawn_follows_path() {
        let mut ret = ReturnToSpawn::new(7.0);
        ret.set_path(NavPath {
            waypoints: vec![
                NavPoint::new(30.0, 0.0, 0.0),
                NavPoint::new(15.0, 0.0, 0.0),
                NavPoint::new(0.0, 0.0, 0.0),
            ],
        });
        let pos = NavPoint::new(30.0, 0.0, 0.0);
        let next = ret.tick(0.016, &pos);
        assert!(next.is_some());
    }

    #[test]
    fn return_to_spawn_arrives() {
        let mut ret = ReturnToSpawn::new(100.0); // very fast
        ret.set_path(NavPath {
            waypoints: vec![NavPoint::new(1.0, 0.0, 0.0), NavPoint::new(0.0, 0.0, 0.0)],
        });
        let pos = NavPoint::new(0.5, 0.0, 0.0);
        // First tick advances past first waypoint
        ret.tick(0.016, &pos);
        // Second tick from near spawn
        let result = ret.tick(0.016, &NavPoint::new(0.1, 0.0, 0.0));
        assert!(result.is_none()); // arrived
    }

    #[test]
    fn return_to_spawn_no_path() {
        let mut ret = ReturnToSpawn::new(7.0);
        assert!(ret.tick(0.016, &NavPoint::new(10.0, 0.0, 0.0)).is_none());
    }

    // --- Patrol tests ---

    fn sample_patrol() -> PatrolPath {
        PatrolPath {
            waypoints: vec![
                PatrolWaypoint { position: NavPoint::new(0.0, 0.0, 0.0), delay: 2.0 },
                PatrolWaypoint { position: NavPoint::new(10.0, 0.0, 0.0), delay: 0.0 },
                PatrolWaypoint { position: NavPoint::new(10.0, 0.0, 10.0), delay: 1.0 },
            ],
        }
    }

    #[test]
    fn patrol_moves_toward_waypoint() {
        let path = sample_patrol();
        let mut state = PatrolState::new(7.0);
        let pos = NavPoint::new(5.0, 0.0, 0.0);
        let result = state.tick(0.016, &pos, &path);
        assert!(matches!(result, PatrolTickResult::MoveTo(_)));
    }

    #[test]
    fn patrol_waits_at_waypoint() {
        let path = sample_patrol();
        let mut state = PatrolState::new(100.0); // very fast
        let pos = NavPoint::new(0.0, 0.0, 0.0); // at waypoint 0
        let result = state.tick(0.016, &pos, &path);
        // Reached wp0 (delay=2s), should wait
        assert_eq!(result, PatrolTickResult::Waiting);
        assert_eq!(state.index, 1); // advanced to next
    }

    #[test]
    fn patrol_resumes_after_delay() {
        let path = sample_patrol();
        let mut state = PatrolState::new(100.0);
        let pos = NavPoint::new(0.0, 0.0, 0.0);
        state.tick(0.016, &pos, &path); // reach wp0, start 2s delay
        state.tick(2.1, &pos, &path); // delay expired
        let result = state.tick(0.016, &pos, &path);
        // Should now move toward wp1
        assert!(matches!(result, PatrolTickResult::MoveTo(_)));
    }

    #[test]
    fn patrol_no_delay_skips_wait() {
        let path = sample_patrol();
        let mut state = PatrolState::new(100.0);
        state.index = 1; // at wp1 which has delay=0
        let pos = NavPoint::new(10.0, 0.0, 0.0);
        let result = state.tick(0.016, &pos, &path);
        // No delay → immediately MoveTo next
        assert!(matches!(result, PatrolTickResult::MoveTo(_)));
        assert_eq!(state.index, 2);
    }

    #[test]
    fn patrol_loops_back_to_start() {
        let path = sample_patrol();
        let mut state = PatrolState::new(100.0);
        state.index = 2; // at wp2 (last)
        let pos = NavPoint::new(10.0, 0.0, 10.0);
        state.tick(0.016, &pos, &path); // reach wp2, delay=1s
        assert_eq!(state.index, 0); // wrapped to 0
    }

    #[test]
    fn patrol_empty_path_waits() {
        let path = PatrolPath { waypoints: vec![] };
        let mut state = PatrolState::new(7.0);
        let result = state.tick(0.016, &NavPoint::new(0.0, 0.0, 0.0), &path);
        assert_eq!(result, PatrolTickResult::Waiting);
    }

    // --- Fear / knockback tests ---

    #[test]
    fn forced_move_target_on_navmesh() {
        let tile = sample_tile(); // 0-10 x 0-10 square
        let origin = NavPoint::new(5.0, 100.0, 5.0);
        let target = forced_move_target(&tile, &origin, 0.0, 3.0); // +X
        assert!((target.x - 8.0).abs() < 0.01);
        assert!(tile.find_polygon(&target).is_some());
    }

    #[test]
    fn forced_move_clamped_to_navmesh() {
        let tile = sample_tile(); // 0-10 x 0-10
        let origin = NavPoint::new(5.0, 100.0, 5.0);
        // Push 50 yards — way outside the 10x10 tile
        let target = forced_move_target(&tile, &origin, 0.0, 50.0);
        // Should be clamped to somewhere within the navmesh
        assert!(tile.find_polygon(&target).is_some());
        assert!(target.x <= 10.0);
    }

    #[test]
    fn knockback_state_expires() {
        let target = NavPoint::new(10.0, 0.0, 0.0);
        let mut state = ForcedMoveState::knockback(target, 0.5, 20.0);
        assert!(!state.tick(0.3));
        assert!(state.tick(0.3)); // expired
    }

    #[test]
    fn fear_state_expires() {
        let target = NavPoint::new(10.0, 0.0, 0.0);
        let mut state = ForcedMoveState::fear(target, 3.0, 10.0);
        assert!(!state.tick(2.0));
        assert!(state.tick(1.5));
    }

    #[test]
    fn forced_move_reached_target() {
        let target = NavPoint::new(10.0, 0.0, 0.0);
        let state = ForcedMoveState::knockback(target, 1.0, 20.0);
        assert!(!state.reached_target(&NavPoint::new(5.0, 0.0, 0.0)));
        assert!(state.reached_target(&NavPoint::new(10.0, 0.0, 0.0)));
    }

    #[test]
    fn forced_move_different_angles() {
        let tile = sample_tile();
        let origin = NavPoint::new(5.0, 100.0, 5.0);

        let north = forced_move_target(&tile, &origin, std::f32::consts::FRAC_PI_2, 3.0);
        assert!(north.z > origin.z); // moved +Z

        let south = forced_move_target(&tile, &origin, -std::f32::consts::FRAC_PI_2, 3.0);
        assert!(south.z < origin.z); // moved -Z
    }

    // --- Charge / leap tests ---

    #[test]
    fn charge_arrives_at_target() {
        let mut charge = DirectMoveState::charge(
            NavPoint::new(0.0, 0.0, 0.0),
            NavPoint::new(20.0, 0.0, 0.0),
            40.0, // 40 y/s → 0.5s to travel 20y
        );
        assert!(!charge.tick(0.3));
        assert!(charge.tick(0.3)); // 0.6s > 0.5s
        assert!(charge.is_complete());
    }

    #[test]
    fn charge_interpolation_midway() {
        let charge = DirectMoveState {
            move_type: DirectMoveType::Charge,
            start: NavPoint::new(0.0, 0.0, 0.0),
            destination: NavPoint::new(20.0, 0.0, 0.0),
            speed: 40.0,
            elapsed: 0.25,
            total_time: 0.5,
        };
        let pos = charge.current_position();
        assert!((pos.x - 10.0).abs() < 0.01); // halfway
    }

    #[test]
    fn leap_fixed_travel_time() {
        let mut leap = DirectMoveState::leap(
            NavPoint::new(0.0, 0.0, 0.0),
            NavPoint::new(30.0, 10.0, 0.0),
            0.8, // 0.8s travel
        );
        assert!(!leap.tick(0.5));
        assert!(leap.tick(0.4));
        let pos = leap.current_position();
        assert!((pos.x - 30.0).abs() < 0.01);
        assert!((pos.y - 10.0).abs() < 0.01);
    }

    #[test]
    fn charge_zero_distance_instant() {
        let mut charge = DirectMoveState::charge(
            NavPoint::new(5.0, 0.0, 5.0),
            NavPoint::new(5.0, 0.0, 5.0),
            40.0,
        );
        assert!(charge.tick(0.001)); // instant
    }

    #[test]
    fn direct_move_position_at_completion() {
        let mut charge = DirectMoveState::charge(
            NavPoint::new(0.0, 0.0, 0.0),
            NavPoint::new(10.0, 5.0, 10.0),
            100.0,
        );
        charge.tick(10.0); // way past
        let pos = charge.current_position();
        // Clamped to t=1.0
        assert!((pos.x - 10.0).abs() < 0.01);
        assert!((pos.y - 5.0).abs() < 0.01);
    }
}
