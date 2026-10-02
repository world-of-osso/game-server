use glam::Vec3;

use super::LosTile;

/// A deterministic scatter of `count` triangles up to 4 yd across in a 200 yd cube.
fn scatter(count: usize) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 40) as f32 / (1u64 << 24) as f32
    };
    let mut vertices = Vec::new();
    for _ in 0..count {
        let center = Vec3::new(next(), next(), next()) * 200.0;
        for _ in 0..3 {
            vertices.push(center + (Vec3::new(next(), next(), next()) - 0.5) * 4.0);
        }
    }
    let triangles = (0..count as u32)
        .map(|tri| [tri * 3, tri * 3 + 1, tri * 3 + 2])
        .collect();
    (vertices, triangles)
}

#[test]
fn bvh_first_hit_matches_every_triangle_tested() {
    let (vertices, triangles) = scatter(5000);
    let tile = LosTile::build(32, 48, vertices.clone(), triangles.clone());
    let brute = LosTile {
        row: 32,
        col: 48,
        vertices,
        triangles: triangles.clone(),
        nodes: vec![super::Node {
            min: Vec3::splat(-10.0),
            max: Vec3::splat(210.0),
            start: 0,
            count: triangles.len() as u32,
        }],
    };
    let mut hits = 0;
    for step in 0..400 {
        let f = step as f32;
        let a = Vec3::new(f * 0.5, 100.0 + (f * 0.37).sin() * 90.0, 0.0);
        let b = Vec3::new(200.0 - f * 0.3, 100.0 + (f * 0.11).cos() * 90.0, 200.0);
        assert_eq!(
            tile.first_hit(a, b),
            brute.first_hit(a, b),
            "segment {step}"
        );
        assert_eq!(tile.blocks(a, b), brute.blocks(a, b), "segment {step}");
        hits += usize::from(tile.blocks(a, b));
    }
    assert!(
        hits > 50 && hits < 350,
        "{hits} of 400 segments hit: not a discriminating set"
    );
}

#[test]
fn tile_bytes_round_trip_and_reject_truncation() {
    let (vertices, triangles) = scatter(300);
    let tile = LosTile::build(30, 47, vertices, triangles);
    let bytes = tile.to_bytes();
    assert_eq!(LosTile::from_bytes(&bytes).unwrap(), tile);
    assert!(LosTile::from_bytes(&bytes[..bytes.len() - 4]).is_err());
}

#[test]
fn segment_touching_a_surface_at_its_end_is_clear() {
    let ground = LosTile::build(
        0,
        0,
        vec![
            Vec3::new(-10.0, 0.0, -10.0),
            Vec3::new(10.0, 0.0, -10.0),
            Vec3::new(0.0, 0.0, 10.0),
        ],
        vec![[0, 1, 2]],
    );
    assert!(!ground.blocks(Vec3::new(0.0, 5.0, 0.0), Vec3::new(1.0, 0.0, 0.0)));
    assert!(ground.blocks(Vec3::new(0.0, 5.0, 0.0), Vec3::new(1.0, -0.5, 0.0)));
}
