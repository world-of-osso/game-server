#![cfg(feature = "bake")]

use glam::Vec3;
use los::bake::Baker;
use std::path::Path;

fn has_vale_wall(triangles: &[[Vec3; 3]]) -> bool {
    triangles.iter().any(|triangle| {
        triangle.iter().any(|point| {
            (point.x - 1506.7384).abs() < 0.01
                && (point.y - 167.72896).abs() < 0.01
                && (point.z + 1776.4214).abs() < 0.01
        })
    })
}

#[test]
#[ignore = "requires extracted Blinding Vale collision assets"]
fn inferred_wall_exclusion_is_limited_to_vale_navigation() {
    let data = Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    for (map, expected_wall) in [(2859, false), (2858, true)] {
        let mut baker = Baker::for_navigation_map(data, map);
        let tiles = baker.map_tiles(6_380_645).unwrap();
        let (geometry, _) = baker.collect_tile(&tiles, (28, 28)).unwrap().unwrap();
        assert!(baker.problems.is_empty(), "{:?}", baker.problems);
        assert_eq!(has_vale_wall(&geometry.triangles), expected_wall);
    }
}
