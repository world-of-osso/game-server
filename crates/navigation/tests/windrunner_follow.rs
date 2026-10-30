//! Real-bake regression for the follower formation offset on Windrunner's ramp.
use glam::Vec3;
use navigation::{NavigationMap, NavigationPlan};
use std::path::PathBuf;

#[test]
#[ignore = "requires GAME_SERVER_NAV_DIR with the Windrunner bake"]
fn windrunner_ramp_formation_reaches_connected_ground() {
    let root = PathBuf::from(std::env::var_os("GAME_SERVER_NAV_DIR").expect("GAME_SERVER_NAV_DIR"));
    let mut map = NavigationMap::open(&root.join("2805")).expect("Windrunner navigation bake");
    let follower = Vec3::new(5327.3438, 101.20245, 3218.2292);
    let raw_offset = Vec3::new(5329.034, 100.521484, 3222.6953);
    // A player position on the same ramp, observed on the return route.
    let owner = Vec3::new(5325.944, 100.5208, 3225.244);
    let path = map
        .follow_formation_path(follower, raw_offset, owner, &mut NavigationPlan::default())
        .expect("formation point must be walkable and connected");
    assert!(path.complete, "follower must reach the resolved point");
    let destination = *path.points.last().expect("route has a destination");
    assert!(
        destination.distance(owner) < 5.0,
        "{destination:?} is not near {owner:?}"
    );
    map.complete_walk_path(owner, destination)
        .expect("resolved formation point is connected to owner");
}
