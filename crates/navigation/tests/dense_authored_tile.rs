//! Real authored City of Threads tile through the production map-bake executable.
#![cfg(feature = "bake")]

use navigation::{NavigationMap, read_walkable_polygons};
use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires FOLLOWER_NAV_DATA and FOLLOWER_NAV_WORLD_DB local assets"]
fn city_of_threads_dense_authored_tile_bakes_walkable_navigation() {
    let data = PathBuf::from(std::env::var_os("FOLLOWER_NAV_DATA").unwrap());
    let world = std::env::var_os("FOLLOWER_NAV_WORLD_DB").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let selection = dir.path().join("selection");
    std::fs::create_dir(&selection).unwrap();
    // The existing baker reads only LOS filenames for terrain-tile selection.
    // All geometry still comes from this real tile's authored WDT/ADT/WMO/M2 inputs.
    std::fs::write(selection.join("31_37.los"), []).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nav-bake"))
        .arg("--data")
        .arg(data)
        .arg("--world-db")
        .arg(world)
        .arg("--out")
        .arg(dir.path())
        .args(["--map", "2669", "--los-tiles"])
        .arg(selection)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let nav_dir = dir.path().join("navmesh/2669");
    let polygons = read_walkable_polygons(&nav_dir).unwrap();
    assert!(
        !polygons.is_empty(),
        "authored walkable floors must survive"
    );
    let mut nav = NavigationMap::open(&nav_dir).unwrap();
    let polygon = &polygons[polygons.len() / 2];
    let center = polygon
        .iter()
        .copied()
        .map(glam::Vec3::from)
        .sum::<glam::Vec3>()
        / polygon.len() as f32;
    let path = nav
        .path(center, center)
        .expect("baked floor must be queryable");
    assert!(!path.is_empty());
}
