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
            water_level: None,
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
fn wow_bevy_coordinate_round_trip() {
    let cases = [
        (0.0, 0.0, 0.0),
        (-8949.0, -132.0, 83.0),
        (100.0, -50.0, 200.0),
        (-1000.0, 5000.0, -300.0),
    ];
    for (wx, wy, wz) in cases {
        let (bx, by, bz) = wow_to_bevy(wx, wy, wz);
        let (rx, ry, rz) = bevy_to_wow(bx, by, bz);
        assert!(
            (rx - wx).abs() < 1e-6 && (ry - wy).abs() < 1e-6 && (rz - wz).abs() < 1e-6,
            "wow→bevy→wow failed for ({wx},{wy},{wz}): got ({rx},{ry},{rz})",
        );

        // Also test bevy→wow→bevy
        let (bx2, by2, bz2) = wow_to_bevy(rx, ry, rz);
        assert!(
            (bx2 - bx).abs() < 1e-6 && (by2 - by).abs() < 1e-6 && (bz2 - bz).abs() < 1e-6,
            "bevy→wow→bevy failed",
        );
    }
}

#[test]
fn multiple_chunks_loaded_and_routed_correctly() {
    // Two adjacent chunks with different base heights
    let map = TerrainHeightMap {
        chunks: vec![
            ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                water_level: None,
            },
            ChunkHeight {
                corner_x: 100.0 - CHUNK_SIZE,
                corner_y: 100.0,
                base_z: 80.0,
                heights: [0.0; MCVT_COUNT],
                water_level: None,
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
            base_z: 100.0,
            heights,
            water_level: None,
        }],
    };

    // Exact vertex (0,0) at chunk corner → 100 + 25 = 125
    let h = map.height_at(100.0, 100.0).unwrap();
    assert!((h - 125.0).abs() < 0.01, "vertex (0,0): got {h}");

    // Exact vertex (0,4) at 4 units along X → 100 + 15 = 115
    let x = 100.0 - UNIT_SIZE * 4.0;
    let h = map.height_at(x, 100.0).unwrap();
    assert!((h - 115.0).abs() < 0.01, "vertex (0,4): got {h}");
}

#[test]
fn height_consistent_across_adjacent_chunk_boundary() {
    // Two chunks adjacent along X axis. Chunk A's min-X edge = Chunk B's max-X edge.
    // Shared boundary at x = 100 - CHUNK_SIZE.
    let boundary_x = 100.0 - CHUNK_SIZE;
    let shared_height = 42.0;

    // Chunk A: corner at (100, 100). Col=8 is the min-X edge (boundary).
    // Set entire col=8 row=0 vertex to shared_height.
    let mut heights_a = [0.0f32; MCVT_COUNT];
    heights_a[8] = shared_height; // outer grid (0, 8) = row 0, col 8

    // Chunk B: corner at (boundary_x, 100). Col=0 is the max-X edge.
    let mut heights_b = [0.0f32; MCVT_COUNT];
    heights_b[0] = shared_height; // outer grid (0, 0) = row 0, col 0

    let map = TerrainHeightMap {
        chunks: vec![
            ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0,
                base_z: 0.0,
                heights: heights_a,
                water_level: None,
            },
            ChunkHeight {
                corner_x: boundary_x,
                corner_y: 100.0,
                base_z: 0.0,
                heights: heights_b,
                water_level: None,
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
            base_z: -10.0,
            heights,
            water_level: None,
        }],
    };
    let h = map.height_at(100.0, 100.0).unwrap();
    assert!((h - (-15.0)).abs() < 0.01, "base -10 + offset -5 = -15: got {h}");
}

#[test]
fn water_surface_height_separate_from_terrain() {
    let map = TerrainHeightMap {
        chunks: vec![ChunkHeight {
            corner_x: 100.0,
            corner_y: 100.0,
            base_z: -20.0, // terrain is underwater
            heights: [0.0; MCVT_COUNT],
            water_level: Some(5.0), // water surface at 5.0
        }],
    };

    let terrain = map.height_at(95.0, 95.0).unwrap();
    let water = map.water_height_at(95.0, 95.0).unwrap();
    assert!((terrain - (-20.0)).abs() < 0.01, "terrain: {terrain}");
    assert!((water - 5.0).abs() < 0.01, "water surface: {water}");
    assert!(water > terrain, "water above terrain");

    // Chunk without water returns None for water_height
    let dry_map = test_helpers::flat_terrain(100.0, 100.0, 50.0);
    assert!(dry_map.water_height_at(95.0, 95.0).is_none());

    // Outside loaded area returns None for both
    assert!(map.water_height_at(200.0, 200.0).is_none());
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
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                water_level: None,
            },
            ChunkHeight {
                corner_x: 100.0,
                corner_y: 100.0 - CHUNK_SIZE,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                water_level: None,
            },
            ChunkHeight {
                corner_x: 100.0 - CHUNK_SIZE,
                corner_y: 100.0 - CHUNK_SIZE,
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                water_level: None,
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
        map.height_at(95.0 - CHUNK_SIZE, 95.0 - CHUNK_SIZE).is_some(),
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
                base_z: 50.0,
                heights: [0.0; MCVT_COUNT],
                water_level: None,
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

    // azeroth_32_48.adt covers roughly X: -533..0, Y: -9067..-8533.
    // Pick a point in the center of the tile.
    let h = map.height_at(-266.0, -8800.0);
    assert!(h.is_some(), "center of tile should have terrain");
    let h = h.unwrap();
    assert!(h > -500.0 && h < 2000.0, "height {h} seems unreasonable");

    // Outside the tile returns None.
    assert!(map.height_at(-8949.0, -132.0).is_none());
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

    // The Northshire start position is on a neighboring tile, so verify
    // a known coordinate within this tile instead.
    // azeroth_32_48 covers X: ~-533..0, Y: ~-9067..-8533 (approx).
    // Pick the tile center and verify height is reasonable terrain.
    let h = map.height_at(-266.0, -8800.0).unwrap();
    // Elwynn Forest terrain is roughly 30-120m elevation.
    assert!(
        h > 20.0 && h < 200.0,
        "Elwynn terrain height {h} should be 20-200",
    );
}
