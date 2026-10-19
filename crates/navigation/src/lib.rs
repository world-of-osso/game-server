//! Offline Recast navigation tiles and Detour queries, in Bevy world coordinates.
#[cfg(feature = "bake")]
pub mod bake;
mod query;
pub use query::NavigationMap;
pub use query::read_walkable_polygons;
pub mod worker;

pub const TILE_SIZE: f32 = 1600.0 / 3.0;
pub const SUBDIVISIONS: u32 = 4;
pub const SUBTILE_SIZE: f32 = TILE_SIZE / SUBDIVISIONS as f32;
pub const CELL_SIZE: f32 = TILE_SIZE / 2048.0;
pub const CELL_HEIGHT: f32 = 0.2;
/// Same body radius as shared::los::body::BODY_RADIUS (TrinityCore ObjectDefines.h).
pub const AGENT_RADIUS: f32 = 0.388;
/// Server mirror_timers::DEFAULT_COLLISION_HEIGHT (TrinityCore Object.h), used
/// without display-specific collision heights by the existing server.
pub const AGENT_HEIGHT: f32 = 2.03128;
/// shared::ground::STEP_UP_HEIGHT, the client/server grounded step budget.
pub const AGENT_CLIMB: f32 = 1.6;
/// shared::movement::MAX_SLOPE_ANGLE, in degrees for Recast.
pub const AGENT_SLOPE: f32 = 50.0;
pub const WATER_AREA: u8 = 2;
pub const WALK_FLAG: u16 = 1;
pub const MAGIC: &[u8; 8] = b"OSSONAV3";

pub fn tile_bounds((row, col): (u32, u32)) -> (f32, f32) {
    let center = 32.0 * TILE_SIZE;
    (
        center - (col + 1) as f32 * TILE_SIZE,
        row as f32 * TILE_SIZE - center,
    )
}

pub fn tile_of(point: glam::Vec3) -> (u32, u32) {
    let center = 32.0 * TILE_SIZE;
    (
        ((point.z + center) / TILE_SIZE).floor().clamp(0.0, 63.0) as u32,
        ((center - point.x) / TILE_SIZE).floor().clamp(0.0, 63.0) as u32,
    )
}
