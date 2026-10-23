//! Offline all-pairs native walk distances; one loaded map, no runtime mutation.
use glam::Vec3;
use navigation::NavigationMap;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

fn parse_points(text: &str) -> Result<Vec<(u32, Vec3)>, String> {
    let mut points = Vec::new();
    let mut identities = HashSet::new();
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 4 {
            return Err(format!("point line{} requires guid,x,y,z", index + 1));
        }
        let guid: u32 = fields[0]
            .parse()
            .map_err(|error| format!("point line{} identity: {error}", index + 1))?;
        if !identities.insert(guid) {
            return Err(format!("duplicate source identity {guid}"));
        }
        let mut coordinates = [0.0_f32; 3];
        for (axis, value) in coordinates.iter_mut().zip(&fields[1..]) {
            *axis = value
                .parse()
                .map_err(|error| format!("point line{} coordinate: {error}", index + 1))?;
        }
        let point = Vec3::from_array(coordinates);
        if !point.is_finite() {
            return Err(format!(
                "point line{} has non-finite coordinates",
                index + 1
            ));
        }
        points.push((guid, point));
    }
    if points.is_empty() {
        return Err("source point file is empty".into());
    }
    Ok(points)
}

fn query_and_print_edge(
    map: &mut NavigationMap,
    (from, start): (u32, Vec3),
    (to, end): (u32, Vec3),
) -> Result<&'static str, String> {
    match map.complete_path(start, end) {
        Ok(path) => {
            let yards: f32 = path.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
            if !yards.is_finite() {
                return Err(format!("native edge {from}->{to} has non-finite distance"));
            }
            let status = if yards > 0.0 { "walk" } else { "coincident" };
            println!("{from},{to},{status},{yards}");
            Ok(status)
        }
        Err(error) => {
            eprintln!("blocked native edge {from}->{to}: {error}");
            println!("{from},{to},blocked,");
            Ok("blocked")
        }
    }
}

fn query_and_print_edges(map: &mut NavigationMap, points: &[(u32, Vec3)]) -> Result<(), String> {
    let mut counts = BTreeMap::<&str, usize>::new();
    println!("from,to,status,yards");
    for (index, source) in points.iter().enumerate() {
        for destination in &points[index + 1..] {
            let status = query_and_print_edge(map, *source, *destination)?;
            *counts.entry(status).or_default() += 1;
        }
    }
    eprintln!("native edge counts={counts:?}");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: follower_edges NAV_DIRECTORY SOURCE_POINTS_CSV".into());
    }
    let text = std::fs::read_to_string(&args[2])?;
    let points = parse_points(&text)?;
    let mut map = NavigationMap::open(Path::new(&args[1]))?;
    query_and_print_edges(&mut map, &points)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retain_explicit_native_identity_and_three_dimensional_pose() {
        let points = parse_points("29004390,-9.47569,222.65,1003.72\n").unwrap();
        assert_eq!(
            points,
            vec![(29004390, Vec3::new(-9.47569, 222.65, 1003.72))]
        );
    }

    #[test]
    fn reject_ambiguous_or_unusable_native_input() {
        for input in [
            "",
            "42,1,2",
            "bad,1,2,3",
            "42,NaN,2,3",
            "42,1,2,3\n42,4,5,6\n",
        ] {
            assert!(parse_points(input).is_err(), "accepted {input:?}");
        }
    }
}
