/// Convert WoW coordinates (X-right, Y-forward, Z-up) to Bevy (X-right, Y-up, Z-back).
pub(crate) fn wow_to_bevy(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    (x, z, -y)
}

/// Convert Bevy coordinates (X-right, Y-up, Z-back) to WoW (X-right, Y-forward, Z-up).
pub(crate) fn bevy_to_wow(bx: f32, by: f32, bz: f32) -> (f32, f32, f32) {
    (bx, -bz, by)
}

/// Yards per MCNK chunk side (100/3).
const CHUNK_SIZE: f32 = 100.0 / 3.0;
/// Distance between outer grid vertices.
const UNIT_SIZE: f32 = CHUNK_SIZE / 8.0;
/// Number of height floats per MCNK (9x9 outer + 8x8 inner).
const MCVT_COUNT: usize = 145;
use crate::liquid::{ChunkLiquid, LiquidSample, parse_mh2o};
use wow_adt::adt::{parse_mcvt, parse_terrain_height_header};
use wow_chunk::{ChunkIter, find_reversed_chunk};

pub use shared::movement::GRAVITY;
/// Small threshold before gravity kicks in (yards). Stricter than client for validation.
pub const GROUND_SNAP_THRESHOLD: f32 = 0.1;

/// Per-chunk height data in WoW world coordinates.
#[derive(Clone, Debug)]
struct ChunkHeight {
    /// WoW X of chunk corner (MCNK offset 0x68).
    corner_x: f32,
    /// WoW Y of chunk corner (MCNK offset 0x6c).
    corner_y: f32,
    /// MCNK area ID at header offset 0x34 (AreaTable.db2).
    area_id: u32,
    /// Base Z height (= raw pos[2]), MCVT heights are relative to this.
    base_z: f32,
    /// 145 height floats from MCVT (9x9 outer + 8x8 inner grid).
    heights: [f32; MCVT_COUNT],
    /// Holes as an 8x8 quad mask, bit `row * 8 + col` (the low-res 4x4 mask widened).
    holes: u64,
    /// MH2O liquid of this chunk.
    liquid: ChunkLiquid,
}

/// Parsed ADT terrain heights of one map tile for server-side collision.
#[derive(Default)]
pub struct TerrainHeightMap {
    chunks: Vec<ChunkHeight>,
}

impl TerrainHeightMap {
    /// Look up the interpolated terrain height at a WoW world position.
    ///
    /// Returns `None` outside all loaded terrain chunks and in a terrain hole, which has no
    /// terrain (TrinityCore `GridMap::getHeight` returns `INVALID_HEIGHT` there).
    pub fn height_at(&self, x: f32, y: f32) -> Option<f32> {
        let chunk = self.find_chunk(x, y)?;
        let (row, col) = chunk_quad(chunk, x, y);
        let hole = chunk.holes >> (row * 8 + col) & 1 != 0;
        (!hole).then(|| interpolate_height(chunk, x, y))
    }

    /// AreaTable ID authored on the MCNK containing WoW `(x, y)`.
    pub fn area_at(&self, x: f32, y: f32) -> Option<u32> {
        self.find_chunk(x, y).map(|chunk| chunk.area_id)
    }

    /// Find the chunk containing the given WoW (x, y) position.
    fn find_chunk(&self, x: f32, y: f32) -> Option<&ChunkHeight> {
        // MCNK corners are independently rounded f32s. Compare their authored grid
        // cells, not raw bounds: subtraction otherwise leaves sub-milliyard seams.
        let grid_x = (x / CHUNK_SIZE).ceil();
        let grid_y = (y / CHUNK_SIZE).ceil();
        self.chunks.iter().find(|chunk| {
            (chunk.corner_x / CHUNK_SIZE).round() == grid_x
                && (chunk.corner_y / CHUNK_SIZE).round() == grid_y
        })
    }

    /// Look up terrain height for a position in Bevy space.
    /// Converts Bevy→WoW for lookup, returns height in Bevy Y.
    pub fn height_at_bevy(&self, bx: f32, bz: f32) -> Option<f32> {
        let (wow_x, wow_y, _) = bevy_to_wow(bx, 0.0, bz);
        self.height_at(wow_x, wow_y).map(|wow_z| {
            let (_, bevy_y, _) = wow_to_bevy(0.0, 0.0, wow_z);
            bevy_y
        })
    }

    /// The terrain liquid over WoW `(x, y)`: `None` outside loaded terrain or where no
    /// layer covers the point.
    pub fn liquid_at(&self, x: f32, y: f32) -> Option<LiquidSample> {
        let chunk = self.find_chunk(x, y)?;
        let liquid = chunk.liquid.sample(chunk.corner_x, chunk.corner_y, x, y)?;
        Some(LiquidSample {
            depth_level: interpolate_height(chunk, x, y),
            ..liquid
        })
    }
}

/// The chunk's quad grid position of WoW `(x, y)`: fractional (row, col) from its corner.
fn chunk_grid_position(chunk: &ChunkHeight, x: f32, y: f32) -> (f32, f32) {
    (
        (chunk.corner_x - x).clamp(0.0, CHUNK_SIZE) / UNIT_SIZE,
        (chunk.corner_y - y).clamp(0.0, CHUNK_SIZE) / UNIT_SIZE,
    )
}

/// The (row, col) of the chunk quad holding WoW `(x, y)`.
fn chunk_quad(chunk: &ChunkHeight, x: f32, y: f32) -> (usize, usize) {
    let (row_f, col_f) = chunk_grid_position(chunk, x, y);
    ((row_f as usize).min(7), (col_f as usize).min(7))
}

/// Sample the authored four-triangle fan, including its inner center vertex.
fn interpolate_height(chunk: &ChunkHeight, x: f32, y: f32) -> f32 {
    let (row_f, col_f) = chunk_grid_position(chunk, x, y);
    let (row, col) = chunk_quad(chunk, x, y);
    let h = |r: usize, c: usize| chunk.heights[r * 17 + c];
    let corners = [
        h(row, col),
        h(row, col + 1),
        h(row + 1, col),
        h(row + 1, col + 1),
    ];
    let center = chunk.heights[row * 17 + 9 + col];
    chunk.base_z + interpolate_fan(corners, center, col_f - col as f32, row_f - row as f32)
}

fn interpolate_fan([tl, tr, bl, br]: [f32; 4], center: f32, col: f32, row: f32) -> f32 {
    if row <= col && row <= 1.0 - col {
        tl * (1.0 - col - row) + tr * (col - row) + center * (2.0 * row)
    } else if col >= row && col >= 1.0 - row {
        tr * (col - row) + br * (col + row - 1.0) + center * (2.0 * (1.0 - col))
    } else if row >= col && row >= 1.0 - col {
        bl * (row - col) + br * (col + row - 1.0) + center * (2.0 * (1.0 - row))
    } else {
        tl * (1.0 - col - row) + bl * (row - col) + center * (2.0 * col)
    }
}

// -- ADT binary parsing (server-side, no rendering) --

/// Parse an ADT file's raw bytes and return a `TerrainHeightMap` resource.
pub fn parse_adt_heights(data: &[u8]) -> Result<TerrainHeightMap, String> {
    let payloads = collect_mcnk_payloads(data)?;
    let mut liquids = match find_reversed_chunk(data, b"MH2O")? {
        Some(mh2o) => parse_mh2o(mh2o)?,
        None => Vec::new(),
    }
    .into_iter();
    let mut chunks = Vec::with_capacity(payloads.len());
    for payload in payloads {
        let mut chunk = parse_mcnk_height(payload)?;
        chunk.liquid = liquids.next().unwrap_or_default();
        chunks.push(chunk);
    }
    Ok(TerrainHeightMap { chunks })
}

/// Collect all KNCM (reversed MCNK) chunk payloads from the ADT.
fn collect_mcnk_payloads(data: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut mcnks = Vec::with_capacity(256);
    let mut off = 0usize;
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk.map_err(|_| format!("Chunk truncated at {off:#x}"))?;
        if tag == b"KNCM" {
            mcnks.push(payload);
        }
        off += 8 + payload.len();
    }
    if mcnks.is_empty() {
        return Err("No KNCM (MCNK) chunks found".to_string());
    }
    Ok(mcnks)
}

/// Parse a single MCNK payload for height data only (position + MCVT).
fn parse_mcnk_height(payload: &[u8]) -> Result<ChunkHeight, String> {
    let header = parse_terrain_height_header(payload)?;
    let heights = find_mcvt(&payload[128..])?;
    let holes = header
        .holes_high_res
        .unwrap_or_else(|| widen_low_res_holes(header.holes_low_res));

    Ok(ChunkHeight {
        corner_x: header.origin[0],
        corner_y: header.origin[1],
        area_id: header.area_id,
        base_z: header.origin[2],
        heights,
        holes,
        liquid: ChunkLiquid::default(),
    })
}

/// The 4x4 low-res hole mask (bit `row * 4 + col`, each a 2x2 block of quads) as an 8x8
/// quad mask.
fn widen_low_res_holes(low: u16) -> u64 {
    (0..64)
        .filter(|bit| low >> ((bit / 8 / 2) * 4 + (bit % 8) / 2) & 1 != 0)
        .fold(0, |mask, bit| mask | 1 << bit)
}

/// Scan sub-chunks after the 128-byte MCNK header for TVCM (reversed MCVT).
fn find_mcvt(sub: &[u8]) -> Result<[f32; MCVT_COUNT], String> {
    // Runtime reports an incomplete scan as missing, including truncated subchunks.
    let payload = match find_reversed_chunk(sub, b"MCVT") {
        Ok(Some(payload)) => payload,
        Ok(None) | Err(_) => return Err("MCNK missing TVCM sub-chunk".to_string()),
    };
    parse_mcvt(payload)
}

/// Test helpers exposed for use by other modules' tests.
#[cfg(test)]
pub mod test_helpers {
    use super::*;

    /// Ground made of one flat terrain chunk (see [`flat_terrain`]).
    pub fn flat_ground(corner_x: f32, corner_y: f32, height: f32) -> crate::ground::GroundMap {
        let (bx, _, bz) = wow_to_bevy(corner_x - 1.0, corner_y - 1.0, 0.0);
        crate::ground::GroundMap::with_terrain(
            crate::ground::bevy_to_tile(bx, bz),
            flat_terrain(corner_x, corner_y, height),
        )
    }

    /// Explicit planar fixture spanning three chunks; no production ground fallback.
    pub fn flat_arena_ground(
        map_id: u32,
        corner: [f32; 2],
        height: f32,
    ) -> crate::ground::GroundMap {
        let mut terrain = TerrainHeightMap::default();
        for row in 0..3 {
            for column in 0..3 {
                terrain.chunks.extend(
                    flat_terrain(
                        corner[0] - column as f32 * CHUNK_SIZE,
                        corner[1] - row as f32 * CHUNK_SIZE,
                        height,
                    )
                    .chunks,
                );
            }
        }
        let (x, _, z) = wow_to_bevy(corner[0] - 1.0, corner[1] - 1.0, height);
        crate::ground::GroundMap::with_terrain_on(
            map_id,
            crate::ground::bevy_to_tile(x, z),
            terrain,
        )
    }

    /// Create a flat terrain chunk at the given corner with uniform height.
    pub fn flat_terrain(corner_x: f32, corner_y: f32, height: f32) -> TerrainHeightMap {
        flat_terrain_in_area(corner_x, corner_y, height, 0)
    }

    /// Explicit MCNK areas under the supplied WoW points, for fixed-region fixtures.
    pub fn terrain_areas(points: &[(f32, f32, u32)]) -> TerrainHeightMap {
        let chunks = points
            .iter()
            .flat_map(|&(x, y, area)| {
                let corner_x = (x / CHUNK_SIZE).ceil() * CHUNK_SIZE;
                let corner_y = (y / CHUNK_SIZE).ceil() * CHUNK_SIZE;
                flat_terrain_in_area(corner_x, corner_y, 0.0, area).chunks
            })
            .collect();
        TerrainHeightMap { chunks }
    }

    /// Explicit MCNK area fixture, consumed through GroundMap::area_at.
    pub fn flat_terrain_in_area(
        corner_x: f32,
        corner_y: f32,
        height: f32,
        area_id: u32,
    ) -> TerrainHeightMap {
        TerrainHeightMap {
            chunks: vec![ChunkHeight {
                corner_x,
                corner_y,
                area_id,
                base_z: height,
                heights: [0.0; MCVT_COUNT],
                holes: 0,
                liquid: ChunkLiquid::default(),
            }],
        }
    }
}

#[cfg(test)]
#[path = "terrain_height_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "terrain_height/format_receipts_tests.rs"]
mod format_receipts_tests;
