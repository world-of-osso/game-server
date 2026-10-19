use super::codec::{append_float, header};
use los::liquid::ChunkLiquid;
use std::collections::BTreeSet;

const STEP: f32 = 100.0 / 3.0 / 8.0;
const EDGE_OFFSET: f32 = 1.0 / 512.0;
pub(super) type Point = (u32, [f32; 2]);

fn add(points: &mut BTreeSet<(u32, u32, u32)>, index: usize, row: f32, col: f32) {
    points.insert((
        index as u32,
        (-row * STEP).to_bits(),
        (-col * STEP).to_bits(),
    ));
}

fn add_edges(
    points: &mut BTreeSet<(u32, u32, u32)>,
    index: usize,
    layer: &los::liquid::WaterLayer,
) {
    let (r0, c0) = (layer.y_offset as f32, layer.x_offset as f32);
    let (r1, c1) = (r0 + layer.height as f32, c0 + layer.width as f32);
    for offset in [-EDGE_OFFSET, 0.0, EDGE_OFFSET] {
        for row in [r0 + offset, r1 + offset] {
            add(points, index, row, (c0 + c1) * 0.5);
        }
        for col in [c0 + offset, c1 + offset] {
            add(points, index, (r0 + r1) * 0.5, col);
        }
    }
}

/// Capture-only: replay decodes the saved XY bits instead of using the new output.
pub(super) fn generate(chunks: &[ChunkLiquid]) -> Vec<Point> {
    let mut points = BTreeSet::new();
    add(&mut points, 255, -1.0, 9.0);
    for (index, chunk) in chunks
        .iter()
        .enumerate()
        .filter(|(_, chunk)| !chunk.layers.is_empty())
    {
        for row in 0..=8 {
            for col in 0..=8 {
                add(&mut points, index, row as f32, col as f32);
            }
        }
        for row in 0..8 {
            for col in 0..8 {
                add(&mut points, index, row as f32 + 0.5, col as f32 + 0.5);
            }
        }
        for layer in &chunk.layers {
            add_edges(&mut points, index, layer);
        }
    }
    points
        .into_iter()
        .map(|(i, x, y)| (i, [f32::from_bits(x), f32::from_bits(y)]))
        .collect()
}

pub(super) fn encode(points: &[Point]) -> Vec<u8> {
    let mut bytes = header(b"WMQI", points.len());
    for (index, [x, y]) in points {
        bytes.extend_from_slice(&index.to_le_bytes());
        append_float(&mut bytes, *x);
        append_float(&mut bytes, *y);
    }
    assert_eq!(bytes.len(), 12 + 12 * points.len());
    bytes
}

pub(super) fn decode(bytes: &[u8]) -> Vec<Point> {
    assert!(bytes.len() >= 12);
    assert_eq!(&bytes[..4], b"WMQI");
    assert_eq!(wow_chunk::read_u32(bytes, 4).unwrap(), 1);
    let count = wow_chunk::read_u32(bytes, 8).unwrap() as usize;
    assert_eq!(bytes.len(), 12 + 12 * count);
    bytes[12..]
        .chunks_exact(12)
        .map(|record| {
            (
                wow_chunk::read_u32(record, 0).unwrap(),
                [
                    wow_chunk::read_f32(record, 4).unwrap(),
                    wow_chunk::read_f32(record, 8).unwrap(),
                ],
            )
        })
        .collect()
}

pub(super) fn outcomes(chunks: &[ChunkLiquid], points: &[Point]) -> Vec<u8> {
    let mut bytes = header(b"WMQO", points.len());
    for &(index, [x, y]) in points {
        bytes.extend_from_slice(&index.to_le_bytes());
        append_float(&mut bytes, x);
        append_float(&mut bytes, y);
        if let Some(liquid) = chunks[index as usize].sample(0.0, 0.0, x, y) {
            bytes.push(1);
            append_float(&mut bytes, liquid.level);
            append_float(&mut bytes, liquid.depth_level);
            bytes.extend_from_slice(&liquid.liquid_type.to_le_bytes());
            bytes.push(u8::from(liquid.deep));
        } else {
            bytes.extend_from_slice(&[0; 12]);
        }
    }
    assert_eq!(bytes.len(), 12 + 24 * points.len());
    bytes
}
