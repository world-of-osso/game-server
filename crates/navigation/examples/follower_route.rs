//! Print real baked navigation corners as Bevy x,y,z CSV for diagnostic physical walking.
use glam::Vec3;
use navigation::NavigationMap;
use std::collections::HashSet;
use std::path::Path;

fn parse_point(values: &[String]) -> Result<Vec3, Box<dyn std::error::Error>> {
    Ok(Vec3::new(
        values[0].parse()?,
        values[1].parse()?,
        values[2].parse()?,
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 8 {
        return Err(
            "usage: follower_route NAV_DIRECTORY START_X START_Y START_Z END_X END_Y END_Z".into(),
        );
    }
    let start = parse_point(&args[2..5])?;
    let end = parse_point(&args[5..8])?;
    let mut map = NavigationMap::open(Path::new(&args[1]))?;
    for corner in query_complete_route(&mut map, start, end)? {
        println!("{},{},{}", corner.x, corner.y, corner.z);
    }
    Ok(())
}

/// Model physical segment-end repathing, without raising the production node pool.
fn query_complete_route(
    map: &mut NavigationMap,
    mut from: Vec3,
    target: Vec3,
) -> Result<Vec<Vec3>, String> {
    let mut endpoints = HashSet::new();
    let mut points = Vec::new();
    let mut plan = navigation::NavigationPlan::default();
    loop {
        let segment = map.path_with_plan(from, target, &mut plan)?;
        let endpoint = *segment.points.last().ok_or("empty navigation segment")?;
        points.extend(segment.points);
        if segment.complete {
            return Ok(points);
        }
        let is_new = endpoints.insert(endpoint.to_array().map(f32::to_bits));
        if !is_new || endpoint == from {
            return Err(
                "unreachable target (partial corridor made no progress or repeated)".into(),
            );
        }
        from = endpoint;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2519 assets"]
    fn canonical_gorek_route_models_segment_end_repath() {
        let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
        let mut map = NavigationMap::open(&root.join("2519")).unwrap();
        let start = Vec3::new(70.6448, 72.6027, 779.064);
        let target = Vec3::new(-264.45807, 125.19722, 503.5);
        let expected = map.complete_path(target, target).unwrap();
        let route = query_complete_route(&mut map, start, target).unwrap();
        assert!(route.len() > 500);
        assert!(route.last().unwrap().distance(*expected.last().unwrap()) < 0.001);
    }

    #[test]
    #[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2519 assets"]
    fn canonical_gorek_reaches_magmatusk_and_sargha() {
        let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
        let start = Vec3::new(-257.87, 123.38, 489.73);
        for (entry, target) in [
            (181861, Vec3::new(-211.1, 99.34897, 932.7)),
            (189901, Vec3::new(-380.025, 63.997375, 772.8375)),
        ] {
            let mut map = NavigationMap::open(&root.join("2519")).unwrap();
            let expected = map.complete_path(target, target).unwrap();
            let mut cursor = start;
            let mut plan = navigation::NavigationPlan::default();
            for step in 0..8 {
                let segment = map.path_with_plan(cursor, target, &mut plan).unwrap();
                let endpoint = *segment.points.last().unwrap();
                eprintln!(
                    "entry {entry} segment {step}: {cursor:?} -> {endpoint:?}, complete={}, points={}",
                    segment.complete,
                    segment.points.len()
                );
                if segment.complete || endpoint == cursor {
                    break;
                }
                cursor = endpoint;
            }
            let route = query_complete_route(&mut map, start, target)
                .unwrap_or_else(|error| panic!("entry {entry}: {error}"));
            assert!(route.last().unwrap().distance(*expected.last().unwrap()) < 0.001);
        }
    }

    #[test]
    #[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2516 assets"]
    fn canonical_map_2516_reaches_distant_bosses() {
        let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
        let start = Vec3::new(-570.405, 431.296, -2189.37);
        for target in [
            Vec3::new(-536.05316, 381.20233, -4163.211),
            Vec3::new(-2095.497, 536.04694, -4335.8853),
        ] {
            let mut map = NavigationMap::open(&root.join("2516")).unwrap();
            let expected = map.complete_path(target, target).unwrap();
            let route = query_complete_route(&mut map, start, target).unwrap();
            assert!(route.last().unwrap().distance(*expected.last().unwrap()) < 0.001);
        }
    }

    #[test]
    #[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2516 assets"]
    fn map_2516_portal_replanning_does_not_oscillate() {
        let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
        let mut map = NavigationMap::open(&root.join("2516")).unwrap();
        let start = Vec3::new(-559.8952, 340.06406, -1631.8356);
        let end = Vec3::new(-1593.5763, 467.57016, -2037.4996);
        let expected = map.complete_path(end, end).unwrap();
        let route = query_complete_route(&mut map, start, end).unwrap();
        assert!(route.last().unwrap().distance(*expected.last().unwrap()) < 0.001);
    }

    #[test]
    #[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2516 assets"]
    fn map_2516_crossing_reaches_the_selected_polygon() {
        let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
        let mut map = NavigationMap::open(&root.join("2516")).unwrap();
        let start = Vec3::new(-380.46945, 4.942341, -5310.091);
        let end = Vec3::new(-1086.8489, 148.08261, -4447.6563);
        let expected = map.complete_path(end, end).unwrap();
        let route = query_complete_route(&mut map, start, end).unwrap();
        assert!(route.last().unwrap().distance(*expected.last().unwrap()) < 0.001);
    }

    #[test]
    #[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2527 assets"]
    fn connected_route_can_leave_the_endpoint_tile_rectangle() {
        let root = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
        let mut map = NavigationMap::open(&root.join("2527")).unwrap();
        let start = Vec3::new(1596.4414, 0.2, 3729.7742);
        let end = Vec3::new(-15.495445, 0.15541077, -518.7507);
        let expected = map.complete_path(end, end).unwrap();
        let route = query_complete_route(&mut map, start, end).unwrap();
        assert!(route.last().unwrap().distance(*expected.last().unwrap()) < 0.001);
    }

    #[test]
    #[ignore = "requires FOLLOWER_NAV_CANONICAL canonical2519 assets"]
    fn long_route_rejects_missing_corrupt_and_mismatched_graph() {
        use std::os::unix::fs::symlink;
        let source = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap())
            .join("2519");
        let temp = tempfile::tempdir().unwrap();
        for entry in std::fs::read_dir(&source).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "nav") {
                symlink(&path, temp.path().join(path.file_name().unwrap())).unwrap();
            }
        }
        let start = Vec3::new(-257.87, 123.38, 489.73);
        let end = Vec3::new(-211.1, 99.34897, 932.7);
        let query = || {
            NavigationMap::open(temp.path())
                .unwrap()
                .path(start, end)
                .unwrap_err()
        };
        assert!(query().contains("portal-graph.json"));
        let artifact = temp.path().join("portal-graph.json");
        std::fs::write(&artifact, b"not a graph").unwrap();
        assert!(query().contains("portal-graph.json"));
        std::fs::copy(source.join("portal-graph.json"), &artifact).unwrap();
        let extra_tile = temp.path().join("0_0.nav");
        assert!(!extra_tile.exists());
        std::fs::write(extra_tile, b"changed").unwrap();
        assert!(query().contains("digest mismatch"));
    }

    #[test]
    fn parse_real_stockade_corner_and_reject_invalid_coordinates() {
        let input = ["56.6821".into(), "-19.2691".into(), "-0.62376".into()];
        assert_eq!(
            parse_point(&input).unwrap(),
            Vec3::new(56.6821, -19.2691, -0.62376)
        );
        let invalid = ["bad".into(), "0".into(), "0".into()];
        assert!(parse_point(&invalid).is_err());
    }
}
