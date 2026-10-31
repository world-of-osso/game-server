use super::*;

#[test]
fn brackenhide_complete_routes_load_tiles_from_a_cold_map() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh/2520");
    let start = Vec3::new(-3300.65, 0.129981, -4681.51);
    for (target, longer_than_two_minutes) in [
        (Vec3::new(-3217.9695, 34.78784, -4078.9055), false),
        (Vec3::new(-3344.54, 111.17662, -3663.96), true),
    ] {
        let mut map = NavigationMap::open(&root).unwrap();
        assert!(
            map.loaded.is_empty(),
            "route begins without preloaded tiles"
        );
        let route = crate::complete_route::follow_complete_route(&mut map, start, target);
        assert!(route.complete, "incomplete route: {:?}", route.reason);
        assert_eq!(
            route.distance > 7.0 * 120.0,
            longer_than_two_minutes,
            "{} yards",
            route.distance
        );
        assert!(!map.loaded.is_empty(), "the query loaded its own tiles");
    }
}
