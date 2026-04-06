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
    /// IDs of adjacent polygons (for graph traversal). One per edge.
    /// 0 = no neighbor (boundary edge).
    pub neighbors: Vec<u32>,
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
    pub fn find_polygon(&self, point: &NavPoint) -> Option<u32> {
        self.polygons
            .iter()
            .find(|poly| poly.flags.walkable && self.point_in_polygon(poly, point))
            .map(|poly| poly.id)
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
        let poly_id = self.find_polygon(&point)?;
        let poly = &self.polygons[poly_id as usize];
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
                neighbors: vec![0, 0, 0, 0],
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
                    neighbors: vec![0, 0, 0],
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
}
