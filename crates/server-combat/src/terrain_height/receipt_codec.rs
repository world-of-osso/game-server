use super::*;
use std::collections::BTreeSet;

pub(super) const HEIGHT_RECORD_BYTES: usize = 604;
pub(super) const QUERY_RECORD_BYTES: usize = 35;
const HEADER_BYTES: usize = 12;
const QUERY_INPUT_BYTES: usize = 8;
const SEAM_OFFSET: f32 = 1.0 / 512.0;

fn header(magic: &[u8; 4], count: usize) -> Vec<u8> {
    let mut bytes = magic.to_vec();
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&u32::try_from(count).unwrap().to_le_bytes());
    bytes
}

fn append_float(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
}

pub(super) fn height_snapshot(map: &TerrainHeightMap) -> Vec<u8> {
    let mut bytes = header(b"THGT", map.chunks.len());
    for chunk in &map.chunks {
        for value in [chunk.corner_x, chunk.corner_y, chunk.base_z] {
            append_float(&mut bytes, value);
        }
        bytes.extend_from_slice(&chunk.area_id.to_le_bytes());
        for &height in &chunk.heights {
            append_float(&mut bytes, height);
        }
        bytes.extend_from_slice(&chunk.holes.to_le_bytes());
    }
    assert_eq!(
        bytes.len(),
        HEADER_BYTES + map.chunks.len() * HEIGHT_RECORD_BYTES
    );
    bytes
}

fn add_point(points: &mut BTreeSet<[u32; 2]>, chunk: &ChunkHeight, row: f32, col: f32) {
    let x = chunk.corner_x - row * UNIT_SIZE;
    let y = chunk.corner_y - col * UNIT_SIZE;
    points.insert([x.to_bits(), y.to_bits()]);
}

fn add_quad_probes(points: &mut BTreeSet<[u32; 2]>, chunk: &ChunkHeight, row: usize, col: usize) {
    for (dr, dc) in [
        (0.25, 0.5),
        (0.5, 0.75),
        (0.75, 0.5),
        (0.5, 0.25),
        (0.5, 0.5),
    ] {
        add_point(points, chunk, row as f32 + dr, col as f32 + dc);
    }
}

fn add_vertex_and_seam(
    points: &mut BTreeSet<[u32; 2]>,
    chunk: &ChunkHeight,
    row: usize,
    col: usize,
) {
    add_point(points, chunk, row as f32, col as f32);
    if row != 0 && row != 8 && col != 0 && col != 8 {
        return;
    }
    let x = chunk.corner_x - row as f32 * UNIT_SIZE;
    let y = chunk.corner_y - col as f32 * UNIT_SIZE;
    for (dx, dy) in [
        (SEAM_OFFSET, 0.0),
        (-SEAM_OFFSET, 0.0),
        (0.0, SEAM_OFFSET),
        (0.0, -SEAM_OFFSET),
    ] {
        points.insert([(x + dx).to_bits(), (y + dy).to_bits()]);
    }
}

/// Capture-only: replay reads the saved bits and never calls this function.
pub(super) fn generate_query_points(map: &TerrainHeightMap) -> Vec<[f32; 2]> {
    let mut points = BTreeSet::new();
    for chunk in &map.chunks {
        for row in 0..8 {
            for col in 0..8 {
                add_quad_probes(&mut points, chunk, row, col);
            }
        }
        for row in 0..=8 {
            for col in 0..=8 {
                add_vertex_and_seam(&mut points, chunk, row, col);
            }
        }
    }
    // Existing authenticated Northshire wet spot and Jasperlode hole/floor controls.
    for [x, y] in [
        [-8558f32, -490f32],
        [-9172.45, -599.435],
        [-9176.0, -595.8],
        [-266.0, -8800.0],
    ] {
        points.insert([x.to_bits(), y.to_bits()]);
    }
    points
        .into_iter()
        .map(|[x, y]| [f32::from_bits(x), f32::from_bits(y)])
        .collect()
}

pub(super) fn encode_query_points(points: &[[f32; 2]]) -> Vec<u8> {
    let mut bytes = header(b"THQI", points.len());
    for [x, y] in points {
        append_float(&mut bytes, *x);
        append_float(&mut bytes, *y);
    }
    bytes
}

pub(super) fn decode_query_points(bytes: &[u8]) -> Vec<[f32; 2]> {
    assert!(bytes.len() >= HEADER_BYTES, "frozen query header truncated");
    assert_eq!(&bytes[..4], b"THQI");
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 1);
    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), HEADER_BYTES + count * QUERY_INPUT_BYTES);
    bytes[HEADER_BYTES..]
        .chunks_exact(QUERY_INPUT_BYTES)
        .map(|point| {
            [
                f32::from_bits(u32::from_le_bytes(point[..4].try_into().unwrap())),
                f32::from_bits(u32::from_le_bytes(point[4..8].try_into().unwrap())),
            ]
        })
        .collect()
}

fn append_optional_word(bytes: &mut Vec<u8>, value: Option<u32>) {
    bytes.push(u8::from(value.is_some()));
    bytes.extend_from_slice(&value.unwrap_or(0).to_le_bytes());
}

fn append_liquid(bytes: &mut Vec<u8>, value: Option<LiquidSample>) {
    let Some(value) = value else {
        bytes.extend_from_slice(&[0; 12]);
        return;
    };
    bytes.push(1);
    append_float(bytes, value.level);
    append_float(bytes, value.depth_level);
    bytes.extend_from_slice(&value.liquid_type.to_le_bytes());
    bytes.push(u8::from(value.deep));
}

pub(super) fn query_snapshot(map: &TerrainHeightMap, points: &[[f32; 2]]) -> Vec<u8> {
    let mut bytes = header(b"THQY", points.len());
    for &[x, y] in points {
        append_float(&mut bytes, x);
        append_float(&mut bytes, y);
        append_optional_word(&mut bytes, map.height_at(x, y).map(f32::to_bits));
        append_optional_word(&mut bytes, map.area_at(x, y));
        append_optional_word(&mut bytes, map.height_at_bevy(x, -y).map(f32::to_bits));
        append_liquid(&mut bytes, map.liquid_at(x, y));
    }
    assert_eq!(
        bytes.len(),
        HEADER_BYTES + points.len() * QUERY_RECORD_BYTES
    );
    bytes
}

pub(super) fn negative_header(count: usize) -> Vec<u8> {
    header(b"THNE", count)
}
