#![cfg(feature = "bake")]

use glam::Vec3;
use los::bake::CollectedGeometry;
use navigation::{NavigationMap, TILE_SIZE, bake::bake_collected_on_grid};

fn write_floor(dir: &std::path::Path, subdivisions: [u32; 2]) {
    let a = Vec3::new(TILE_SIZE - 20.0, 0.0, 10.0);
    let b = Vec3::new(TILE_SIZE + 20.0, 0.0, 10.0);
    let c = Vec3::new(TILE_SIZE + 20.0, 0.0, 30.0);
    let d = Vec3::new(TILE_SIZE - 20.0, 0.0, 30.0);
    let geometry = CollectedGeometry {
        triangles: vec![[a, b, c], [a, c, d]],
        obstacles: vec![],
    };
    for (tile, grid) in [((32, 30), subdivisions[0]), ((32, 31), subdivisions[1])] {
        let bytes = bake_collected_on_grid(&geometry, &[], tile, grid).unwrap();
        std::fs::write(dir.join(format!("{}_{}.nav", tile.0, tile.1)), bytes).unwrap();
    }
}

#[test]
fn refined_grid_routes_across_authored_wow_tile_boundary_in_both_directions() {
    let dir = tempfile::tempdir().unwrap();
    write_floor(dir.path(), [8, 8]);
    let mut nav = NavigationMap::open(dir.path()).unwrap();
    for direction in [1.0, -1.0] {
        let start = Vec3::new(TILE_SIZE - direction * 10.0, 0.0, 20.0);
        let end = Vec3::new(TILE_SIZE + direction * 10.0, 0.0, 20.0);
        let route = nav.path(start, end).expect("connected refined-grid floor");
        let length: f32 = route.windows(2).map(|p| p[0].distance(p[1])).sum();
        assert!(
            (length - 20.0).abs() < 0.1,
            "route {route:?}, length {length}"
        );
        assert!(route.last().unwrap().distance(end) < 0.3);
    }
}

#[test]
fn mixed_navigation_grids_are_explicitly_rejected() {
    let dir = tempfile::tempdir().unwrap();
    write_floor(dir.path(), [8, 4]);
    let result = NavigationMap::open(dir.path()).and_then(|mut nav| {
        nav.path(
            Vec3::new(TILE_SIZE - 10.0, 0.0, 20.0),
            Vec3::new(TILE_SIZE + 10.0, 0.0, 20.0),
        )
    });
    let error = result.expect_err("mixed grids must never produce a route");
    assert!(error.contains("mixed navigation grids"), "{error}");
}
