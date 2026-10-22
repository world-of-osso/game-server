//! Export validated baked walkable polygons for offline sourced-placement heights.
//! Each line is one polygon: semicolon-separated Bevy x,y,z vertices.
use navigation::read_walkable_polygons;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: follower_surfaces NAV_DIRECTORY".into());
    }
    for polygon in read_walkable_polygons(Path::new(&args[1]))? {
        let vertices: Vec<_> = polygon
            .iter()
            .map(|point| format!("{},{},{}", point[0], point[1], point[2]))
            .collect();
        println!("{}", vertices.join(";"));
    }
    Ok(())
}
