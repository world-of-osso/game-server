//! ADT MH2O terrain liquid, sampled the way the Godot client samples it
//! (game-engine `src/asset/adt_format/adt_tex_water.rs`, `terrain_height_data.rs`), so the
//! server and the client agree on where the water surface is; and the TrinityCore liquid
//! status of a unit in it (`GridMap::GetLiquidStatus`, GridMap.cpp:676-690).
//!
//! WMO liquid (MLIQ) is not read: the client swims in terrain water only.

use wow_adt::adt_tex::{
    parse_water_attributes, read_liquid_instance_header, read_mh2o_chunk_header,
};

/// One MH2O liquid instance of one MCNK.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterLayer {
    /// `LiquidType.db2` ID.
    pub liquid_type: u16,
    pub min_height: f32,
    pub x_offset: u8,
    pub y_offset: u8,
    pub width: u8,
    pub height: u8,
    /// One byte per row, one bit per column.
    pub exists: [u8; 8],
    /// `(width + 1) * (height + 1)` vertex heights; empty for depth-only layers.
    pub heights: Vec<f32>,
}

/// The liquid of one MCNK: its layers and the MH2O `deep` attribute mask.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChunkLiquid {
    pub layers: Vec<WaterLayer>,
    pub deep: u64,
}

/// Liquid over a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidSample {
    /// Surface height (WoW Z = Bevy Y).
    pub level: f32,
    /// Terrain height under the liquid (`LiquidData::depth_level`); filled by the terrain
    /// lookup.
    pub depth_level: f32,
    pub liquid_type: u16,
    /// The chunk's MH2O `deep` mask is set: `map_liquidHeaderTypeFlags::DarkWater`
    /// (map_extractor System.cpp:690).
    pub deep: bool,
}

/// Yards between MH2O vertices (a chunk is 8 of them).
const WATER_STEP: f32 = 100.0 / 3.0 / 8.0;
const CHUNK_COUNT: usize = 256;
const CHUNK_HEADER_SIZE: usize = 12;
const INSTANCE_SIZE: usize = 24;

/// Parse an MH2O payload into the liquid of its 256 chunks, in MCNK order.
pub fn parse_mh2o(payload: &[u8]) -> Result<Vec<ChunkLiquid>, String> {
    if payload.len() < CHUNK_COUNT * CHUNK_HEADER_SIZE {
        return Err(format!("MH2O too small: {} bytes", payload.len()));
    }
    let boundaries = liquid_boundaries(payload)?;
    (0..CHUNK_COUNT)
        .map(|index| parse_chunk(payload, index, &boundaries))
        .collect()
}

/// Modern MH2O values >=42 are LiquidObject IDs, not direct vertex formats.
/// Infer the authored LVF from its exact vertex block size (1/5/8/9 bytes per
/// vertex), as documented by warcraft-rs MH2O; do not read an ID as height data.
fn liquid_boundaries(payload: &[u8]) -> Result<std::collections::BTreeSet<usize>, String> {
    let mut boundaries = std::collections::BTreeSet::from([payload.len()]);
    for index in 0..CHUNK_COUNT {
        let header = read_mh2o_chunk_header(payload, index)?;
        let instances = header.instance_offset as usize;
        let count = header.layer_count as usize;
        let attributes = header.attributes_offset as usize;
        if attributes != 0 {
            boundaries.insert(attributes);
        }
        if count == 0 {
            continue;
        }
        if instances == 0
            || instances
                .checked_add(count * INSTANCE_SIZE)
                .is_none_or(|end| end > payload.len())
        {
            return Err(format!("invalid MH2O instance range for chunk {index}"));
        }
        boundaries.insert(instances);
        for layer in 0..count {
            add_instance_boundaries(payload, instances + layer * INSTANCE_SIZE, &mut boundaries)?;
        }
    }
    Ok(boundaries)
}

fn add_instance_boundaries(
    payload: &[u8],
    offset: usize,
    boundaries: &mut std::collections::BTreeSet<usize>,
) -> Result<(), String> {
    let instance = read_liquid_instance_header(payload, offset)?;
    for start in [instance.exists_offset, instance.vertex_offset] {
        if start != 0 {
            boundaries.insert(start as usize);
        }
    }
    Ok(())
}

fn parse_chunk(
    payload: &[u8],
    chunk_index: usize,
    boundaries: &std::collections::BTreeSet<usize>,
) -> Result<ChunkLiquid, String> {
    let header = read_mh2o_chunk_header(payload, chunk_index)?;
    let instances = header.instance_offset as usize;
    let layer_count = header.layer_count as usize;
    let attributes = header.attributes_offset as usize;
    let deep = if attributes == 0 {
        0
    } else {
        parse_water_attributes(payload, attributes)
            .map_err(|_| format!("MH2O u64 out of bounds at {:#x}", attributes + 8))?
            .deep
    };
    let layers = if instances == 0 {
        Vec::new()
    } else {
        (0..layer_count)
            .map(|layer| parse_instance(payload, instances + layer * INSTANCE_SIZE, boundaries))
            .collect::<Result<_, _>>()?
    };
    Ok(ChunkLiquid { layers, deep })
}

fn parse_instance(
    payload: &[u8],
    offset: usize,
    boundaries: &std::collections::BTreeSet<usize>,
) -> Result<WaterLayer, String> {
    let instance = read_liquid_instance_header(payload, offset)
        .map_err(|_| format!("MH2O instance out of bounds at {offset:#x}"))?;
    let liquid_type = instance.liquid_type;
    let vertex_format = instance.liquid_object;
    let min_height = instance.min_height;
    let (x_offset, y_offset, width, height) = (
        instance.x_offset,
        instance.y_offset,
        instance.width,
        instance.height,
    );
    let exists_offset = instance.exists_offset as usize;
    let vertex_offset = instance.vertex_offset as usize;
    let vertex_format =
        resolve_vertex_format(vertex_format, vertex_offset, width, height, boundaries)?;
    Ok(WaterLayer {
        liquid_type,
        min_height,
        x_offset,
        y_offset,
        width,
        height,
        exists: read_exists(payload, exists_offset, width, height)?,
        heights: read_heights(payload, vertex_offset, vertex_format, width, height)?,
    })
}

fn resolve_vertex_format(
    encoded: u16,
    offset: usize,
    width: u8,
    height: u8,
    boundaries: &std::collections::BTreeSet<usize>,
) -> Result<u16, String> {
    if encoded < 42 || offset == 0 {
        return Ok(encoded);
    }
    let end = *boundaries
        .range((offset + 1)..)
        .next()
        .ok_or("MH2O vertex block has no end")?;
    let count = (usize::from(width) + 1) * (usize::from(height) + 1);
    let length = end - offset;
    if length % count != 0 {
        return Err(format!(
            "LiquidObject {encoded} vertex block has {length} bytes for {count} vertices"
        ));
    }
    match length / count {
        1 => Ok(2),
        5 => Ok(0),
        8 => Ok(1),
        9 => Ok(3),
        stride => Err(format!(
            "LiquidObject {encoded} has unknown vertex stride {stride}"
        )),
    }
}

/// No bitmap: every quad of the `width × height` rectangle exists.
fn read_exists(payload: &[u8], offset: usize, width: u8, height: u8) -> Result<[u8; 8], String> {
    let mut exists = [0u8; 8];
    let rows = usize::from(height.min(8));
    if offset == 0 {
        let mask = (1u16.wrapping_shl(u32::from(width)) - 1) as u8;
        exists[..rows].fill(mask);
        return Ok(exists);
    }
    let bytes = payload
        .get(offset..offset + rows)
        .ok_or_else(|| format!("MH2O exists bitmap out of bounds at {offset:#x}"))?;
    exists[..rows].copy_from_slice(bytes);
    Ok(exists)
}

/// Vertex heights by liquid vertex format: height+UV (1) and height+UV+depth (3) interleave
/// the height with 4 and 5 bytes more, depth-only (2) has none, any other stores the heights
/// as a float array first.
fn read_heights(
    payload: &[u8],
    offset: usize,
    vertex_format: u16,
    width: u8,
    height: u8,
) -> Result<Vec<f32>, String> {
    if offset == 0 || vertex_format == 2 {
        return Ok(Vec::new());
    }
    let stride = match vertex_format {
        1 => 8,
        3 => 9,
        _ => 4,
    };
    let count = (usize::from(width) + 1) * (usize::from(height) + 1);
    (0..count)
        .map(|index| read_f32(payload, offset + index * stride))
        .collect()
}

impl ChunkLiquid {
    /// The highest layer surface over WoW `(x, y)` in the chunk whose MCNK corner is
    /// `(corner_x, corner_y)`.
    pub fn sample(&self, corner_x: f32, corner_y: f32, x: f32, y: f32) -> Option<LiquidSample> {
        self.layers
            .iter()
            .filter_map(|layer| {
                layer
                    .level_at((corner_x - x) / WATER_STEP, (corner_y - y) / WATER_STEP)
                    .map(|level| LiquidSample {
                        level,
                        depth_level: f32::NEG_INFINITY,
                        liquid_type: layer.liquid_type,
                        deep: self.deep != 0,
                    })
            })
            .max_by(|a, b| a.level.total_cmp(&b.level))
    }
}

impl WaterLayer {
    /// Surface at chunk-local vertex coordinates (`row` from WoW X, `col` from WoW Y);
    /// layer edges are half-open.
    pub fn level_at(&self, row: f32, col: f32) -> Option<f32> {
        let (x_min, y_min) = (f32::from(self.x_offset), f32::from(self.y_offset));
        let inside = col >= x_min
            && col < x_min + f32::from(self.width)
            && row >= y_min
            && row < y_min + f32::from(self.height);
        if !inside {
            return None;
        }
        let (abs_row, abs_col) = (row.floor() as usize, col.floor() as usize);
        let quad_row = abs_row - usize::from(self.y_offset);
        let quad_col = abs_col - usize::from(self.x_offset);
        if quad_row >= 8 || quad_col >= 8 || (self.exists[quad_row] >> quad_col) & 1 == 0 {
            return None;
        }
        let (fx, fz) = (col - abs_col as f32, row - abs_row as f32);
        let vertex = |r: usize, c: usize| self.vertex_height(r, c);
        let top = lerp(
            vertex(quad_row, quad_col),
            vertex(quad_row, quad_col + 1),
            fx,
        );
        let bottom = lerp(
            vertex(quad_row + 1, quad_col),
            vertex(quad_row + 1, quad_col + 1),
            fx,
        );
        Some(lerp(top, bottom, fz))
    }

    fn vertex_height(&self, row: usize, col: usize) -> f32 {
        let width = usize::from(self.width) + 1;
        self.heights
            .get(row * width + col)
            .copied()
            .unwrap_or(self.min_height)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// `ZLiquidStatus` (MapDefines.h:132-140) without the ocean-floor bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidStatus {
    NoWater,
    AboveWater,
    WaterWalk,
    InWater,
    UnderWater,
}

/// `GridMap::GetLiquidStatus` (GridMap.cpp:661-684): no water where the liquid or the feet
/// are under the ground; else under water when the surface is more than `collision_height`
/// over the feet, in water when over them, water-walking within 0.1 under them.
pub fn liquid_status(level: f32, ground: f32, feet: f32, collision_height: f32) -> LiquidStatus {
    /// `GROUND_LEVEL_OFFSET_HACK` (GridMap.cpp).
    const GROUND_LEVEL_OFFSET_HACK: f32 = 0.02;
    if level < ground - GROUND_LEVEL_OFFSET_HACK || feet < ground - GROUND_LEVEL_OFFSET_HACK {
        return LiquidStatus::NoWater;
    }
    let delta = level - feet;
    if delta > collision_height {
        LiquidStatus::UnderWater
    } else if delta > 0.0 {
        LiquidStatus::InWater
    } else if delta > -0.1 {
        LiquidStatus::WaterWalk
    } else {
        LiquidStatus::AboveWater
    }
}

fn read_f32(data: &[u8], offset: usize) -> Result<f32, String> {
    wow_chunk::read_f32(data, offset).map_err(|_| format!("MH2O u32 out of bounds at {offset:#x}"))
}

#[cfg(all(test, feature = "bake"))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires real local-CASC root ADT776382"]
    fn modern_ocean_liquid_object_42_is_depth_only() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../game-engine/data/terrain/776382.adt");
        let root = std::fs::read(data).unwrap();
        let payload = crate::bake::chunks::find_reversed(&root, b"MH2O").unwrap();
        let liquids = parse_mh2o(payload).unwrap();
        assert_eq!(liquids.len(), 256);
        for chunk in liquids {
            assert_eq!(chunk.layers.len(), 1);
            let layer = &chunk.layers[0];
            assert_eq!(layer.liquid_type, 2);
            assert!(
                layer.heights.is_empty(),
                "ocean stores81depth bytes, not81float heights"
            );
            assert_eq!(layer.level_at(4.5, 4.5), Some(0.0));
        }
    }
}
