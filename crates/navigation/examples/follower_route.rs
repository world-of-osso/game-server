//! Print real baked navigation corners as Bevy x,y,z CSV for diagnostic physical walking.
use glam::Vec3;
use navigation::NavigationMap;
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
    for corner in map.path(start, end)? {
        println!("{},{},{}", corner.x, corner.y, corner.z);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
