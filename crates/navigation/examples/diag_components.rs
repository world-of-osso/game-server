//! Export the native Detour component reached from one Bevy-space point.
use glam::Vec3;
use navigation::NavigationMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err("usage: diag_components NAV_DIRECTORY X Y Z OUTPUT".into());
    }
    let point = Vec3::new(args[2].parse()?, args[3].parse()?, args[4].parse()?);
    NavigationMap::open(Path::new(&args[1]))?.diagnostic_export_component(point, &args[5])?;
    Ok(())
}
