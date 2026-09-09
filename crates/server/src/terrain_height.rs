use bevy::prelude::*;

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
    /// Base Z height (= raw pos[2]), MCVT heights are relative to this.
    base_z: f32,
    /// 145 height floats from MCVT (9x9 outer + 8x8 inner grid).
    heights: [f32; MCVT_COUNT],
    /// Water surface height (None = no water in this chunk).
    #[cfg_attr(not(test), allow(dead_code))]
    water_level: Option<f32>,
}

/// Bevy resource holding parsed ADT terrain heightmaps for server-side collision.
#[derive(Resource, Default)]
pub struct TerrainHeightMap {
    chunks: Vec<ChunkHeight>,
}

impl TerrainHeightMap {
    /// Look up the interpolated terrain height at a WoW world position.
    ///
    /// Returns `None` if the position is outside all loaded terrain chunks.
    pub fn height_at(&self, x: f32, y: f32) -> Option<f32> {
        let chunk = self.find_chunk(x, y)?;
        Some(interpolate_height(chunk, x, y))
    }

    /// Find the chunk containing the given WoW (x, y) position.
    fn find_chunk(&self, x: f32, y: f32) -> Option<&ChunkHeight> {
        self.chunks.iter().find(|c| {
            // Terrain grows in negative X/Y from the corner position.
            x <= c.corner_x
                && x >= c.corner_x - CHUNK_SIZE
                && y <= c.corner_y
                && y >= c.corner_y - CHUNK_SIZE
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

    #[cfg(test)]
    /// Look up water surface height at a WoW world position.
    ///
    /// Returns `None` if outside loaded terrain or chunk has no water.
    pub fn water_height_at(&self, x: f32, y: f32) -> Option<f32> {
        let chunk = self.find_chunk(x, y)?;
        chunk.water_level
    }

    /// Returns true if any terrain data is loaded.
    pub fn is_loaded(&self) -> bool {
        !self.chunks.is_empty()
    }
}

/// Sample the authored four-triangle fan, including its inner center vertex.
fn interpolate_height(chunk: &ChunkHeight, x: f32, y: f32) -> f32 {
    let row_f = (chunk.corner_x - x).clamp(0.0, CHUNK_SIZE) / UNIT_SIZE;
    let col_f = (chunk.corner_y - y).clamp(0.0, CHUNK_SIZE) / UNIT_SIZE;
    let row = (row_f as usize).min(7);
    let col = (col_f as usize).min(7);
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
    let mut chunks = Vec::with_capacity(payloads.len());
    for payload in payloads {
        chunks.push(parse_mcnk_height(payload)?);
    }
    Ok(TerrainHeightMap { chunks })
}

/// Collect all KNCM (reversed MCNK) chunk payloads from the ADT.
fn collect_mcnk_payloads(data: &[u8]) -> Result<Vec<&[u8]>, String> {
    let mut mcnks = Vec::with_capacity(256);
    let mut off = 0usize;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = read_u32(data, off + 4)? as usize;
        let payload_start = off + 8;
        let payload_end = payload_start + size;
        if payload_end > data.len() {
            return Err(format!("Chunk truncated at {off:#x}"));
        }
        if tag == b"KNCM" {
            mcnks.push(&data[payload_start..payload_end]);
        }
        off = payload_end;
    }
    if mcnks.is_empty() {
        return Err("No KNCM (MCNK) chunks found".to_string());
    }
    Ok(mcnks)
}

/// Parse a single MCNK payload for height data only (position + MCVT).
fn parse_mcnk_height(payload: &[u8]) -> Result<ChunkHeight, String> {
    if payload.len() < 128 {
        return Err(format!("MCNK too small: {} bytes", payload.len()));
    }
    // Position at offset 0x68 is [X_wow, Y_wow, Z_wow].
    let pos_x = read_f32(payload, 0x68)?; // WoW X
    let pos_y = read_f32(payload, 0x6c)?; // WoW Y
    let pos_z = read_f32(payload, 0x70)?; // WoW Z (base height)

    let heights = find_mcvt(&payload[128..])?;

    Ok(ChunkHeight {
        corner_x: pos_x,
        corner_y: pos_y,
        base_z: pos_z,
        heights,
        water_level: None,
    })
}

/// Scan sub-chunks after the 128-byte MCNK header for TVCM (reversed MCVT).
fn find_mcvt(sub: &[u8]) -> Result<[f32; MCVT_COUNT], String> {
    let mut off = 0usize;
    while off + 8 <= sub.len() {
        let tag = &sub[off..off + 4];
        let size = read_u32(sub, off + 4)? as usize;
        let payload_start = off + 8;
        let payload_end = payload_start + size;
        if payload_end > sub.len() {
            break;
        }
        if tag == b"TVCM" {
            return parse_mcvt(&sub[payload_start..payload_end]);
        }
        off = payload_end;
    }
    Err("MCNK missing TVCM sub-chunk".to_string())
}

/// Parse MCVT payload into 145 height floats.
fn parse_mcvt(payload: &[u8]) -> Result<[f32; MCVT_COUNT], String> {
    let needed = MCVT_COUNT * 4;
    if payload.len() < needed {
        return Err(format!(
            "MCVT too small: {} bytes (need {needed})",
            payload.len()
        ));
    }
    let mut heights = [0.0f32; MCVT_COUNT];
    for (i, h) in heights.iter_mut().enumerate() {
        *h = read_f32(payload, i * 4)?;
    }
    Ok(heights)
}

fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 OOB at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 OOB at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

// -- Bevy plugin for loading terrain at startup --

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_terrain);
    }
}

/// Attempt to load Elwynn Forest ADT at startup. Inserts TerrainHeightMap
/// resource regardless (empty if file not found).
fn load_terrain(mut commands: Commands) {
    let path = "../game-engine/data/terrain/azeroth_32_48.adt";
    match std::fs::read(path) {
        Ok(data) => match parse_adt_heights(&data) {
            Ok(map) => {
                info!("Loaded terrain: {} chunks from {path}", map.chunks.len());
                commands.insert_resource(map);
            }
            Err(e) => {
                warn!("Failed to parse ADT {path}: {e}");
                commands.insert_resource(TerrainHeightMap::default());
            }
        },
        Err(e) => {
            info!("No terrain file at {path}: {e}");
            commands.insert_resource(TerrainHeightMap::default());
        }
    }
}

/// Test helpers exposed for use by other modules' tests.
#[cfg(test)]
pub mod test_helpers {
    use super::*;

    /// Create a flat terrain chunk at the given corner with uniform height.
    pub fn flat_terrain(corner_x: f32, corner_y: f32, height: f32) -> TerrainHeightMap {
        TerrainHeightMap {
            chunks: vec![ChunkHeight {
                corner_x,
                corner_y,
                base_z: height,
                heights: [0.0; MCVT_COUNT],
                water_level: None,
            }],
        }
    }
}

#[cfg(test)]
#[path = "terrain_height_tests.rs"]
mod tests;
