#![cfg(feature = "bake")]

use glam::Vec3;
use los::bake::CollectedGeometry;
use navigation::{NavigationMap, bake::bake_collected};

fn horizontal_patch(x0: f32, x1: f32, y: f32) -> [[Vec3; 3]; 2] {
    let a = Vec3::new(x0, y, 10.0);
    let b = Vec3::new(x0, y, 190.0);
    let c = Vec3::new(x1, y, 190.0);
    let d = Vec3::new(x1, y, 10.0);
    [[a, b, c], [a, c, d]]
}

fn bake_and_query_obstacle_corridor(height: f32) -> Result<Vec<Vec3>, String> {
    let floor = horizontal_patch(10.0, 190.0, 0.0);
    let obstruction = horizontal_patch(90.0, 110.0, height);
    let geometry = CollectedGeometry {
        triangles: floor.into_iter().chain(obstruction).collect(),
        obstacles: obstruction.to_vec(),
    };
    let dir = tempfile::tempdir().unwrap();
    let baked = bake_collected(&geometry, &[], (32, 31)).unwrap();
    std::fs::write(dir.path().join("32_31.nav"), baked).unwrap();
    let mut nav = NavigationMap::open(dir.path()).unwrap();
    nav.complete_path(Vec3::new(60.0, 0.2, 100.0), Vec3::new(140.0, 0.2, 100.0))
}

#[test]
fn low_hanging_obstacle_inside_native_climb_budget_preserves_walkable_step() {
    // This collision obstruction spans the whole corridor, 0.8yd above its floor.
    // Native TC filtering carries walkability onto this climbable span before
    // clearance filtering, rather than deleting the floor beneath it as headroom.
    let points = bake_and_query_obstacle_corridor(0.8)
        .expect("climbable low-hanging collision must not disconnect the corridor");
    assert!(
        points
            .last()
            .unwrap()
            .distance(Vec3::new(140.0, 0.2, 100.0))
            < 0.3
    );
    // Connectivity is the contract here. Funnel corners need not sample the
    // obstruction's peak; exact physical ground correction is a separate layer.
}

#[test]
fn low_hanging_filter_does_not_promote_an_obstacle_above_native_climb_budget() {
    assert!(
        bake_and_query_obstacle_corridor(2.0).is_err(),
        "a two-yard obstruction remains above the unchanged1.6-yard step budget"
    );
}
