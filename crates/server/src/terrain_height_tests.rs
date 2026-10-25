use super::*;

#[test]
fn height_at_returns_none_for_unloaded() {
    let map = TerrainHeightMap::default();
    assert!(map.height_at(0.0, 0.0).is_none());
    assert!(map.height_at(-8949.0, -132.0).is_none());
}

#[test]
fn asymmetric_plane_maps_rows_to_wow_x_and_columns_to_wow_y() {
    let mut heights = [0.0; MCVT_COUNT];
    for row in 0..=8 {
        for col in 0..=8 {
            heights[row * 17 + col] = 10.0 * row as f32 + 2.0 * col as f32;
        }
        if row < 8 {
            for col in 0..8 {
                heights[row * 17 + 9 + col] = 10.0 * (row as f32 + 0.5) + 2.0 * (col as f32 + 0.5);
            }
        }
    }
    let map = TerrainHeightMap {
        chunks: vec![ChunkHeight {
            corner_x: 100.0,
            corner_y: 200.0,
            area_id: 0,
            base_z: 50.0,
            heights,
            holes: 0,
            liquid: ChunkLiquid::default(),
        }],
    };
    for (row, col, expected) in [(2.0, 5.0, 80.0), (2.25, 5.75, 84.0), (7.5, 1.5, 128.0)] {
        let x = 100.0 - row * UNIT_SIZE;
        let y = 200.0 - col * UNIT_SIZE;
        let actual = map.height_at(x, y).unwrap();
        assert!(
            (actual - expected).abs() < 0.001,
            "row={row} col={col}: {actual} != {expected}"
        );
        assert!((map.height_at_bevy(x, -y).unwrap() - expected).abs() < 0.001);
    }
}

#[test]
fn nonplanar_cell_uses_center_vertex_and_all_four_triangles() {
    let mut heights = [0.0; MCVT_COUNT];
    heights[0] = 0.0;
    heights[1] = 4.0;
    heights[17] = 8.0;
    heights[18] = 12.0;
    heights[9] = 20.0;
    let map = TerrainHeightMap {
        chunks: vec![ChunkHeight {
            corner_x: 100.0,
            corner_y: 200.0,
            area_id: 0,
            base_z: 50.0,
            heights,
            holes: 0,
            liquid: ChunkLiquid::default(),
        }],
    };
    // Each off-center point is halfway from an edge midpoint to the center.
    for (row, col, expected) in [
        (0.5, 0.5, 70.0),
        (0.25, 0.5, 61.0),
        (0.5, 0.75, 64.0),
        (0.75, 0.5, 65.0),
        (0.5, 0.25, 62.0),
        (0.0, 0.5, 52.0),
        (1.0, 0.5, 60.0),
    ] {
        let actual = map
            .height_at(100.0 - row * UNIT_SIZE, 200.0 - col * UNIT_SIZE)
            .unwrap();
        assert!(
            (actual - expected).abs() < 0.001,
            "row={row} col={col}: {actual} != {expected}"
        );
    }
}

#[test]
fn parsed_mcnk_preserves_asymmetric_world_coordinates() {
    let mut payload = vec![0u8; 128];
    payload[0x68..0x6c].copy_from_slice(&(-8900.0f32).to_le_bytes());
    payload[0x6c..0x70].copy_from_slice(&(-100.0f32).to_le_bytes());
    payload[0x70..0x74].copy_from_slice(&80.0f32.to_le_bytes());
    payload.extend_from_slice(b"TVCM");
    payload.extend_from_slice(&(MCVT_COUNT as u32 * 4).to_le_bytes());
    payload.extend_from_slice(&[0u8; MCVT_COUNT * 4]);
    let mut adt = b"KNCM".to_vec();
    adt.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    adt.extend_from_slice(&payload);
    let map = parse_adt_heights(&adt).unwrap();
    assert_eq!(map.height_at(-8901.0, -102.0), Some(80.0));
    assert_eq!(map.height_at_bevy(-8901.0, 102.0), Some(80.0));
    assert_eq!(map.height_at(-102.0, -8901.0), None);
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
            area_id: 0,
            base_z: 50.0,
            heights,
            holes: 0,
            liquid: ChunkLiquid::default(),
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

    // Cell midpoint is its authored inner vertex, whose relative height is zero.
    let mid_x = 100.0 - UNIT_SIZE * 0.5;
    let mid_y = 100.0 - UNIT_SIZE * 0.5;
    let h = map.height_at(mid_x, mid_y).unwrap();
    assert!(
        (h - 50.0).abs() < 0.01,
        "mid-cell height should be 50, got {h}"
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
    assert!(
        map.height_at(corner_x, corner_y).is_some(),
        "top-left corner"
    );

    // Bottom-right corner (min x, min y) — inside
    let min_x = corner_x - CHUNK_SIZE;
    let min_y = corner_y - CHUNK_SIZE;
    assert!(map.height_at(min_x, min_y).is_some(), "bottom-right corner");

    // Just outside each edge — None
    assert!(
        map.height_at(corner_x + 0.01, corner_y).is_none(),
        "past max x"
    );
    assert!(
        map.height_at(corner_x, corner_y + 0.01).is_none(),
        "past max y"
    );
    assert!(
        map.height_at(min_x - 0.01, corner_y).is_none(),
        "past min x"
    );
    assert!(
        map.height_at(corner_x, min_y - 0.01).is_none(),
        "past min y"
    );

    // All boundary positions return same flat height
    assert!((map.height_at(corner_x, corner_y).unwrap() - 50.0).abs() < 0.01);
    assert!((map.height_at(min_x, min_y).unwrap() - 50.0).abs() < 0.01);
    assert!((map.height_at(corner_x, min_y).unwrap() - 50.0).abs() < 0.01);
    assert!((map.height_at(min_x, corner_y).unwrap() - 50.0).abs() < 0.01);
}


#[test]
fn multiple_chunks_loaded_and_routed_correctly() {
    // Two adjacent chunks with different base heights
    let map = TerrainHeightMap {
        chunks: vec![
            ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0,
                area_id: 0,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                holes: 0,
                liquid: ChunkLiquid::default(),
            },
            ChunkHeight {
                corner_x: 100.0 - CHUNK_SIZE,
                corner_y: 100.0,
                area_id: 0,
                base_z: 80.0,
                heights: [0.0; MCVT_COUNT],
                holes: 0,
                liquid: ChunkLiquid::default(),
            },
        ],
    };

    // Query in first chunk → height 50
    let h1 = map.height_at(95.0, 95.0).unwrap();
    assert!((h1 - 50.0).abs() < 0.01, "chunk 1: got {h1}");

    // Query in second chunk → height 80
    let h2 = map.height_at(95.0 - CHUNK_SIZE, 95.0).unwrap();
    assert!((h2 - 80.0).abs() < 0.01, "chunk 2: got {h2}");

    // Query outside both → None
    assert!(map.height_at(200.0, 200.0).is_none());
}

#[test]
fn height_at_exact_vertex_no_interpolation() {
    let mut heights = [0.0f32; MCVT_COUNT];
    // Set vertex at outer grid (0,0) to 25.0
    heights[0] = 25.0;
    // Set vertex at outer grid (0,4) to 15.0 (midway along first row)
    heights[4] = 15.0;

    let map = TerrainHeightMap {
        chunks: vec![ChunkHeight {
            corner_x: 100.0,
            corner_y: 100.0,
            area_id: 0,
            base_z: 100.0,
            heights,
            holes: 0,
            liquid: ChunkLiquid::default(),
        }],
    };

    // Exact vertex (0,0) at chunk corner → 100 + 25 = 125
    let h = map.height_at(100.0, 100.0).unwrap();
    assert!((h - 125.0).abs() < 0.01, "vertex (0,0): got {h}");

    // Exact vertex (0,4) at 4 units along decreasing Y → 100 + 15 = 115
    let y = 100.0 - UNIT_SIZE * 4.0;
    let h = map.height_at(100.0, y).unwrap();
    assert!((h - 115.0).abs() < 0.01, "vertex (0,4): got {h}");
}

#[test]
fn height_consistent_across_adjacent_chunk_boundary() {
    // Two chunks adjacent along X axis. Chunk A's min-X edge = Chunk B's max-X edge.
    // Shared boundary at x = 100 - CHUNK_SIZE.
    let boundary_x = 100.0 - CHUNK_SIZE;
    let shared_height = 42.0;

    // Chunk A: row=8 is the min-X boundary; chunk B starts at row=0.
    let mut heights_a = [0.0f32; MCVT_COUNT];
    heights_a[8 * 17] = shared_height;

    // Chunk B: corner at (boundary_x, 100). Row=0 is the max-X edge.
    let mut heights_b = [0.0f32; MCVT_COUNT];
    heights_b[0] = shared_height; // outer grid (0, 0) = row 0, col 0

    let map = TerrainHeightMap {
        chunks: vec![
            ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0,
                area_id: 0,
                base_z: 0.0,
                heights: heights_a,
                holes: 0,
                liquid: ChunkLiquid::default(),
            },
            ChunkHeight {
                corner_x: boundary_x,
                corner_y: 100.0,
                area_id: 0,
                base_z: 0.0,
                heights: heights_b,
                holes: 0,
                liquid: ChunkLiquid::default(),
            },
        ],
    };

    // Query from chunk A side of boundary
    let h_a = map.height_at(boundary_x, 100.0).unwrap();
    // Query from chunk B side (same point, found in chunk B)
    // Both should return shared_height
    assert!(
        (h_a - shared_height).abs() < 0.01,
        "chunk A boundary: {h_a}, expected {shared_height}",
    );
}

#[test]
fn negative_terrain_height_underwater() {
    let map = test_helpers::flat_terrain(100.0, 100.0, -25.0);

    let h = map.height_at(95.0, 95.0).unwrap();
    assert!((h - (-25.0)).abs() < 0.01, "underwater height: got {h}");

    // Negative offset on negative base
    let mut heights = [0.0f32; MCVT_COUNT];
    heights[0] = -5.0;
    let map = TerrainHeightMap {
        chunks: vec![ChunkHeight {
            corner_x: 100.0,
            corner_y: 100.0,
            area_id: 0,
            base_z: -10.0,
            heights,
            holes: 0,
            liquid: ChunkLiquid::default(),
        }],
    };
    let h = map.height_at(100.0, 100.0).unwrap();
    assert!(
        (h - (-15.0)).abs() < 0.01,
        "base -10 + offset -5 = -15: got {h}"
    );
}

#[test]
fn water_surface_height_separate_from_terrain() {
    // One flat layer over the whole chunk at 5.0, over terrain at -20.
    let layer = crate::liquid::WaterLayer {
        liquid_type: 2,
        min_height: 5.0,
        x_offset: 0,
        y_offset: 0,
        width: 8,
        height: 8,
        exists: [0xff; 8],
        heights: Vec::new(),
    };
    let map = TerrainHeightMap {
        chunks: vec![ChunkHeight {
            corner_x: 100.0,
            corner_y: 100.0,
            area_id: 0,
            base_z: -20.0,
            heights: [0.0; MCVT_COUNT],
            holes: 0,
            liquid: ChunkLiquid {
                layers: vec![layer],
                deep: 0,
            },
        }],
    };

    let terrain = map.height_at(95.0, 95.0).unwrap();
    let water = map.liquid_at(95.0, 95.0).unwrap();
    assert!((terrain - (-20.0)).abs() < 0.01, "terrain: {terrain}");
    assert_eq!(
        water,
        LiquidSample {
            level: 5.0,
            depth_level: -20.0,
            liquid_type: 2,
            deep: false
        }
    );

    // Chunk without water returns None for the liquid.
    let dry_map = test_helpers::flat_terrain(100.0, 100.0, 50.0);
    assert!(dry_map.liquid_at(95.0, 95.0).is_none());

    // Outside loaded area returns None for both
    assert!(map.liquid_at(200.0, 200.0).is_none());
}

#[test]
fn large_coordinate_values_no_overflow() {
    // WoW map range is roughly ±17066 on each axis
    let map = test_helpers::flat_terrain(17000.0, 17000.0, 500.0);

    // Within chunk at extreme coordinates
    let h = map.height_at(16990.0, 16990.0).unwrap();
    assert!((h - 500.0).abs() < 0.01, "large coords: got {h}");

    // Outside at extreme values — returns None, no panic
    assert!(map.height_at(100_000.0, 100_000.0).is_none());
    assert!(map.height_at(-100_000.0, -100_000.0).is_none());
    assert!(map.height_at(f32::MAX, 0.0).is_none());
    assert!(map.height_at(0.0, f32::MIN).is_none());

    // Coordinate conversion round-trips at large values
    let (bx, by, bz) = wow_to_bevy(17000.0, -17000.0, 500.0);
    let (rx, ry, rz) = bevy_to_wow(bx, by, bz);
    assert!((rx - 17000.0).abs() < 1e-3);
    assert!((ry - (-17000.0)).abs() < 1e-3);
    assert!((rz - 500.0).abs() < 1e-3);
}

#[test]
fn adt_hole_returns_none_for_missing_chunk() {
    // 2x2 grid with a hole: chunks at (0,0), (0,1), (1,1) but NOT (1,0)
    let map = TerrainHeightMap {
        chunks: vec![
            ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0,
                area_id: 0,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                holes: 0,
                liquid: ChunkLiquid::default(),
            },
            ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0 - CHUNK_SIZE,
                area_id: 0,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                holes: 0,
                liquid: ChunkLiquid::default(),
            },
            ChunkHeight {
                corner_x: 100.0 - CHUNK_SIZE,
                corner_y: 100.0 - CHUNK_SIZE,
                area_id: 0,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                holes: 0,
                liquid: ChunkLiquid::default(),
            },
            // Hole: no chunk at (100 - CHUNK_SIZE, 100)
        ],
    };

    // Present chunks return height
    assert!(map.height_at(95.0, 95.0).is_some(), "chunk (0,0)");
    assert!(
        map.height_at(95.0, 95.0 - CHUNK_SIZE).is_some(),
        "chunk (0,1)",
    );
    assert!(
        map.height_at(95.0 - CHUNK_SIZE, 95.0 - CHUNK_SIZE)
            .is_some(),
        "chunk (1,1)",
    );

    // Hole: no ground
    assert!(
        map.height_at(95.0 - CHUNK_SIZE, 95.0).is_none(),
        "hole at (1,0) should return None",
    );
}

#[test]
fn height_query_1000_lookups_under_threshold() {
    // 16 chunks in a 4x4 grid
    let mut chunks = Vec::new();
    for row in 0..4 {
        for col in 0..4 {
            chunks.push(ChunkHeight {
                corner_x: 100.0 - col as f32 * CHUNK_SIZE,
                corner_y: 100.0 - row as f32 * CHUNK_SIZE,
                area_id: 0,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                holes: 0,
                liquid: ChunkLiquid::default(),
            });
        }
    }
    let map = TerrainHeightMap { chunks };

    let start = std::time::Instant::now();
    let mut hit = 0;
    for i in 0..1000 {
        let x = 100.0 - (i as f32 * 0.1) % (CHUNK_SIZE * 4.0);
        let y = 100.0 - (i as f32 * 0.07) % (CHUNK_SIZE * 4.0);
        if map.height_at(x, y).is_some() {
            hit += 1;
        }
    }
    let elapsed = start.elapsed();

    assert!(hit > 0, "some lookups hit terrain");
    assert!(
        elapsed.as_millis() < 100,
        "1000 lookups across 16 chunks took {}ms (should be <100ms)",
        elapsed.as_millis(),
    );
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

    // Azeroth 32,48 covers WoW X: -9067..-8533, Y: -533..0.
    let h = map.height_at(-8800.0, -266.0);
    assert!(h.is_some(), "center of tile should have terrain");
    let h = h.unwrap();
    assert!(h > -500.0 && h < 2000.0, "height {h} seems unreasonable");

    // Outside the tile returns None.
    assert!(map.height_at(-266.0, -8800.0).is_none());
}

#[test]
fn height_matches_known_azerothcore_coordinates() {
    // Goldshire inn area: WoW X≈-9459, Y≈62, expected Z≈56
    // This is on ADT tile row≈31, col≈49.
    // Also test Northshire Abbey: X≈-8949, Y≈-132, Z≈83
    // On ADT tile row≈32, col≈48.
    let path = "../../../game-engine/data/terrain/azeroth_32_48.adt";
    let Ok(data) = std::fs::read(path) else {
        // Skip if ADT file not available (CI).
        return;
    };
    let map = parse_adt_heights(&data).unwrap();

    // Authored Northshire caravan 10255 sits directly on the ground.
    // MDDF [17180.9140625, 81.8337326, 25962.435546875] in tile coordinates.
    let center = 32.0 * CHUNK_SIZE * 16.0;
    let x = center - 25962.435546875;
    let y = center - 17180.9140625;
    let h = map
        .height_at(x, y)
        .expect("Northshire must be within Azeroth 32,48");
    assert!((h - 81.83374).abs() < 0.001, "authored caravan ground: {h}");
}

/// The Godot swimming fixtures' deep-water spot on Azeroth 32,48 (game-engine
/// docs/specs/swimming.md): live, the client floated at 142.74, feet 1.25 under this
/// surface (liquid type 5, no deep mask).
#[test]
fn swim_spot_liquid_is_the_surface_the_client_floats_under() {
    let path = "../../../game-engine/data/terrain/azeroth_32_48.adt";
    let data = std::fs::read(path).expect("Azeroth 32,48 root ADT");
    let map = parse_adt_heights(&data).unwrap();

    let seabed = map.height_at(-8558.0, -490.0).unwrap();
    let liquid = map
        .liquid_at(-8558.0, -490.0)
        .expect("water at the swim spot");

    assert!((seabed - 140.04).abs() < 0.01, "seabed {seabed}");
    assert!(
        (liquid.level - 143.99).abs() < 0.01,
        "surface {}",
        liquid.level
    );
    assert_eq!(
        (liquid.depth_level, liquid.liquid_type, liquid.deep),
        (seabed, 5, false)
    );
}

/// A low-res hole bit covers a 2x2 block of quads: bit 5 (row 1, col 1) is quads 2..=3
/// on rows 2..=3.
#[test]
fn low_res_hole_bit_covers_its_two_by_two_quads() {
    let mask = widen_low_res_holes(1 << 5);
    let holes: Vec<(u32, u32)> = (0..64)
        .filter(|bit| mask >> bit & 1 != 0)
        .map(|bit| (bit / 8, bit % 8))
        .collect();
    assert_eq!(holes, vec![(2, 2), (2, 3), (3, 2), (3, 3)]);
}
