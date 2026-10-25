use super::codec::{append_float, header};
use crate::ground::GroundMap;
use crate::terrain_height::TerrainHeightMap;
use bevy::math::Vec3;
use std::collections::BTreeSet;

pub(super) type Point = (u32, [f32; 2]);
const EDGE_OFFSET: f32 = 1.0 / 512.0;

fn add(points: &mut BTreeSet<(u32, u32, u32)>, map: u32, x: f32, y: f32) {
    points.insert((map, x.to_bits(), y.to_bits()));
}

/// Capture-only. These OLD-reader coordinates are serialized and never regenerated on replay.
pub(super) fn generate(root: &[u8]) -> Vec<Point> {
    let chunks = wow_adt::adt::parse_terrain_height_chunks(root).unwrap();
    let x = chunks
        .iter()
        .map(|c| c.origin[0])
        .max_by(f32::total_cmp)
        .unwrap();
    let y = chunks
        .iter()
        .map(|c| c.origin[1])
        .max_by(f32::total_cmp)
        .unwrap();
    let side = super::super::SIZE_OF_GRIDS;
    let mut points = BTreeSet::new();
    for row in [0.0, 0.5, 1.0] {
        for col in [0.0, 0.5, 1.0] {
            let (px, py) = (x - row * side, y - col * side);
            add(&mut points, 0, px, py);
            for offset in [-EDGE_OFFSET, EDGE_OFFSET] {
                add(&mut points, 0, px + offset, py);
                add(&mut points, 0, px, py + offset);
            }
        }
    }
    for (row, col) in [
        (2., 1.),
        (1., 2.),
        (1., 4.),
        (2., 5.),
        (4., 5.),
        (5., 4.),
        (5., 2.),
        (4., 1.),
    ] {
        add(&mut points, 0, x - row / 6.0 * side, y - col / 6.0 * side);
    }
    for (row, col) in [(0.25, 0.25), (0.25, 0.75), (0.75, 0.25), (0.75, 0.75)] {
        for offset in [-EDGE_OFFSET, 0.0, EDGE_OFFSET] {
            add(&mut points, 0, x - row * side + offset, y - col * side);
        }
    }
    add(&mut points, 1, x - side * 0.5, y - side * 0.5);
    for [px, py] in [
        [-4500., -3700.],
        [-4300., -3600.],
        [-4700., -3650.],
        [-4500., -3210.],
        [-8820., 660.],
    ] {
        add(&mut points, 0, px, py);
    }
    points
        .into_iter()
        .map(|(map, x, y)| (map, [f32::from_bits(x), f32::from_bits(y)]))
        .collect()
}

pub(super) fn encode(points: &[Point]) -> Vec<u8> {
    let mut bytes = header(b"MFBQ", points.len());
    for (map, [x, y]) in points {
        bytes.extend_from_slice(&map.to_le_bytes());
        append_float(&mut bytes, *x);
        append_float(&mut bytes, *y);
    }
    assert_eq!(bytes.len(), 12 + 12 * points.len());
    bytes
}

pub(super) fn decode(bytes: &[u8]) -> Vec<Point> {
    assert!(bytes.len() >= 12);
    assert_eq!(&bytes[..4], b"MFBQ");
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

/// Warm the actual runtime cache with terrain/MFBO from the actual file loader.
/// No lazy WMO/neighbour reads: min-height queries do not consume their collision data.
pub(super) fn outcomes(
    terrain: TerrainHeightMap,
    min: Option<[i16; 9]>,
    tile: (u32, u32),
    points: &[Point],
) -> Vec<u8> {
    let mut ground = GroundMap::with_terrain(tile, terrain);
    let map = ground.maps.get_mut(&0).unwrap().as_mut().unwrap();
    let tile = map.tiles.get_mut(&tile).unwrap().as_mut().unwrap();
    tile.flight_min = min;
    let mut bytes = header(b"MFBO", points.len());
    let mut supported = 0;
    let mut missing = 0;
    for &(map, [x, y]) in points {
        bytes.extend_from_slice(&map.to_le_bytes());
        append_float(&mut bytes, x);
        append_float(&mut bytes, y);
        let result = ground.min_height(map, Vec3::new(x, 0.0, -y));
        if result.is_some() {
            supported += 1;
        } else {
            missing += 1;
        }
        if let (None, Some(value)) = (min, result) {
            assert_eq!(value.to_bits(), (-500.0f32).to_bits());
        }
        bytes.push(u8::from(result.is_some()));
        append_float(&mut bytes, result.unwrap_or(0.0));
    }
    assert!(
        supported >= 8,
        "queries must exercise the loaded tile, not only missing tiles"
    );
    assert!(
        missing > 0,
        "queries must preserve explicit missing-map/tile outcomes"
    );
    assert_eq!(bytes.len(), 12 + 17 * points.len());
    bytes
}
