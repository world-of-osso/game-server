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
    loop {
        let segment = map.path(from, target)?;
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
