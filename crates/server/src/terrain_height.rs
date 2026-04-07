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
    /// WoW X of chunk corner (= raw pos[1]).
    corner_x: f32,
    /// WoW Y of chunk corner (= raw pos[0]).
    corner_y: f32,
    /// Base Z height (= raw pos[2]), MCVT heights are relative to this.
    base_z: f32,
    /// 145 height floats from MCVT (9x9 outer + 8x8 inner grid).
    heights: [f32; MCVT_COUNT],
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

    /// Returns true if any terrain data is loaded.
    pub fn is_loaded(&self) -> bool {
        !self.chunks.is_empty()
    }
}

/// Bilinear interpolation on the 9x9 outer grid for a point within a chunk.
fn interpolate_height(chunk: &ChunkHeight, x: f32, y: f32) -> f32 {
    // Distance from corner in positive direction (terrain grows negative).
    let dx = chunk.corner_x - x;
    let dy = chunk.corner_y - y;

    // Clamp to [0, CHUNK_SIZE] to handle edge cases.
    let dx = dx.clamp(0.0, CHUNK_SIZE);
    let dy = dy.clamp(0.0, CHUNK_SIZE);

    // Convert to grid cell coordinates (0..8 range for the 9x9 outer grid).
    let col_f = dx / UNIT_SIZE;
    let row_f = dy / UNIT_SIZE;

    let col = (col_f as usize).min(7);
    let row = (row_f as usize).min(7);

    // Fractional position within the cell.
    let frac_x = col_f - col as f32;
    let frac_y = row_f - row as f32;

    // Four corners of the outer grid cell.
    let h00 = outer_height(chunk, row, col);
    let h10 = outer_height(chunk, row, col + 1);
    let h01 = outer_height(chunk, row + 1, col);
    let h11 = outer_height(chunk, row + 1, col + 1);

    // Bilinear interpolation.
    let top = h00 + (h10 - h00) * frac_x;
    let bot = h01 + (h11 - h01) * frac_x;
    top + (bot - top) * frac_y
}

/// Get the absolute height of an outer grid vertex at (row, col) in the 9x9 grid.
fn outer_height(chunk: &ChunkHeight, row: usize, col: usize) -> f32 {
    // Outer vertices in the 145-array: row * 17 + col
    // (each pair of rows = 9 outer + 8 inner = 17 entries).
    let idx = row * 17 + col;
    chunk.base_z + chunk.heights[idx]
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
    // pos at offset 0x68 stored as [Y_wow, X_wow, Z_wow]
    let pos_y = read_f32(payload, 0x68)?; // WoW Y
    let pos_x = read_f32(payload, 0x6c)?; // WoW X
    let pos_z = read_f32(payload, 0x70)?; // WoW Z (base height)

    let heights = find_mcvt(&payload[128..])?;

    Ok(ChunkHeight {
        corner_x: pos_x,
        corner_y: pos_y,
        base_z: pos_z,
        heights,
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
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_at_returns_none_for_unloaded() {
        let map = TerrainHeightMap::default();
        assert!(map.height_at(0.0, 0.0).is_none());
        assert!(map.height_at(-8949.0, -132.0).is_none());
    }

    #[test]
    fn height_interpolation() {
        // Create a single chunk with known heights.
        // Corner at WoW X=100, Y=100, base Z=50.
        // Terrain extends from X=100 down to X=100-CHUNK_SIZE, same for Y.
        let mut heights = [0.0f32; MCVT_COUNT];
        // Set outer grid corners to known values:
        // (0,0) = 10.0, (0,8) = 20.0, (8,0) = 30.0, (8,8) = 40.0
        heights[0] = 10.0; // row=0, col=0 → idx 0
        heights[8] = 20.0; // row=0, col=8 → idx 8
        heights[8 * 17] = 30.0; // row=8, col=0 → idx 136
        heights[8 * 17 + 8] = 40.0; // row=8, col=8 → idx 144

        let map = TerrainHeightMap {
            chunks: vec![ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0,
                base_z: 50.0,
                heights,
            }],
        };

        // At the corner (x=100, y=100) → outer grid (0,0) → height = 50 + 10 = 60
        let h = map.height_at(100.0, 100.0).unwrap();
        assert!(
            (h - 60.0).abs() < 0.01,
            "corner height should be 60, got {h}"
        );

        // At the far corner (x=100-CHUNK_SIZE, y=100-CHUNK_SIZE) → outer grid (8,8)
        let far_x = 100.0 - CHUNK_SIZE;
        let far_y = 100.0 - CHUNK_SIZE;
        let h = map.height_at(far_x, far_y).unwrap();
        assert!((h - 90.0).abs() < 0.01, "far corner should be 90, got {h}");

        // Midpoint of first cell should interpolate between corners.
        // At (col_f=0.5, row_f=0.5), all 4 cell corners have height 0 except (0,0)=10.
        // top = 10 + (0-10)*0.5 = 5, bot = 0 + (0-0)*0.5 = 0, result = 5 + (0-5)*0.5 = 2.5
        let mid_x = 100.0 - UNIT_SIZE * 0.5;
        let mid_y = 100.0 - UNIT_SIZE * 0.5;
        let h = map.height_at(mid_x, mid_y).unwrap();
        assert!(
            (h - 52.5).abs() < 0.01,
            "mid-cell height should be 52.5, got {h}"
        );

        // Outside the chunk returns None.
        assert!(map.height_at(101.0, 100.0).is_none());
        assert!(map.height_at(100.0, 101.0).is_none());
    }

    #[test]
    fn height_at_chunk_center_matches_expected() {
        let corner_x = 100.0;
        let corner_y = 100.0;
        let base_height = 75.0;
        let map = test_helpers::flat_terrain(corner_x, corner_y, base_height);

        // Center of chunk: corner minus half chunk size
        let center_x = corner_x - CHUNK_SIZE / 2.0;
        let center_y = corner_y - CHUNK_SIZE / 2.0;
        let h = map.height_at(center_x, center_y).unwrap();
        assert!(
            (h - base_height).abs() < 0.01,
            "chunk center height {h}, expected {base_height}",
        );

        // Also verify the corner itself
        let h_corner = map.height_at(corner_x, corner_y).unwrap();
        assert!(
            (h_corner - base_height).abs() < 0.01,
            "chunk corner height {h_corner}, expected {base_height}",
        );
    }

    #[test]
    fn height_at_chunk_boundary_edges() {
        let corner_x = 100.0;
        let corner_y = 100.0;
        let map = test_helpers::flat_terrain(corner_x, corner_y, 50.0);

        // Top-left corner (max x, max y) — inside
        assert!(map.height_at(corner_x, corner_y).is_some(), "top-left corner");

        // Bottom-right corner (min x, min y) — inside
        let min_x = corner_x - CHUNK_SIZE;
        let min_y = corner_y - CHUNK_SIZE;
        assert!(map.height_at(min_x, min_y).is_some(), "bottom-right corner");

        // Just outside each edge — None
        assert!(map.height_at(corner_x + 0.01, corner_y).is_none(), "past max x");
        assert!(map.height_at(corner_x, corner_y + 0.01).is_none(), "past max y");
        assert!(map.height_at(min_x - 0.01, corner_y).is_none(), "past min x");
        assert!(map.height_at(corner_x, min_y - 0.01).is_none(), "past min y");

        // All boundary positions return same flat height
        assert!((map.height_at(corner_x, corner_y).unwrap() - 50.0).abs() < 0.01);
        assert!((map.height_at(min_x, min_y).unwrap() - 50.0).abs() < 0.01);
        assert!((map.height_at(corner_x, min_y).unwrap() - 50.0).abs() < 0.01);
        assert!((map.height_at(min_x, corner_y).unwrap() - 50.0).abs() < 0.01);
    }

    #[test]
    fn mcvt_145_floats() {
        // Build a minimal MCVT payload with exactly 145 floats.
        let mut payload = vec![0u8; MCVT_COUNT * 4];
        // Set float at index 72 (center vertex) to 42.0.
        let bytes = 42.0f32.to_le_bytes();
        payload[72 * 4..72 * 4 + 4].copy_from_slice(&bytes);

        let heights = parse_mcvt(&payload).unwrap();
        assert_eq!(heights.len(), MCVT_COUNT);
        assert!((heights[72] - 42.0).abs() < 1e-6);
        assert!((heights[0] - 0.0).abs() < 1e-6);

        // Too-small payload should fail.
        let short = vec![0u8; 144 * 4];
        assert!(parse_mcvt(&short).is_err());
    }

    #[test]
    fn parse_real_adt_file() {
        let path = "../../../game-engine/data/terrain/azeroth_32_48.adt";
        let Ok(data) = std::fs::read(path) else {
            // Skip if file not available (CI).
            return;
        };
        let map = parse_adt_heights(&data).unwrap();
        assert_eq!(map.chunks.len(), 256, "ADT should have 256 MCNK chunks");

        // azeroth_32_48.adt covers roughly X: -533..0, Y: -9067..-8533.
        // Pick a point in the center of the tile.
        let h = map.height_at(-266.0, -8800.0);
        assert!(h.is_some(), "center of tile should have terrain");
        let h = h.unwrap();
        assert!(h > -500.0 && h < 2000.0, "height {h} seems unreasonable");

        // Outside the tile returns None.
        assert!(map.height_at(-8949.0, -132.0).is_none());
    }
}
