//! Line of sight over baked visible geometry (docs/specs/line-of-sight.md).
//!
//! The bake (`los-bake`, feature `bake`) turns a map tile's terrain, opaque WMO batches and
//! solid M2 doodads into one `.los` file: triangles plus a prebuilt BVH. At runtime
//! [`LosMap`] loads those files per tile and answers segment queries; no model or texture
//! is read. Positions are in Bevy world space (Y up): Bevy `(x, y, z)` is WoW `(x, -z, y)`.
//! The crate has no Bevy dependency so the server and the client share it.

pub mod eye;
pub mod map;
pub mod tile;

#[cfg(feature = "bake")]
pub mod bake;

pub use glam::Vec3;
pub use map::LosMap;
pub use tile::LosTile;

/// Yards per map tile side (`shared::navmesh::TILE_SIZE`).
pub const TILE_SIZE: f32 = 533.333_3;
/// Tiles per map side.
pub const MAP_TILES: u32 = 64;

/// `(row, col)` of the map tile containing Bevy `(x, z)`, as the server's ground indexes
/// tiles (`ground::bevy_to_tile`): the row comes from WoW Y (`-z`), the column from WoW X.
pub fn tile_of(x: f32, z: f32) -> (u32, u32) {
    let center = 32.0 * TILE_SIZE;
    let row = ((center + z) / TILE_SIZE).floor() as i32;
    let col = ((center - x) / TILE_SIZE).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

/// The `.los` file of tile `(row, col)` in a map's bake directory.
pub fn tile_file_name((row, col): (u32, u32)) -> String {
    format!("{row}_{col}.los")
}
