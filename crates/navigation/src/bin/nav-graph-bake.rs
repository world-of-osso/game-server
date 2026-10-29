use navigation::NavigationMap;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let directory = args
        .next()
        .ok_or("usage: nav-graph-bake <map-nav-directory>")?;
    if args.next().is_some() {
        return Err("usage: nav-graph-bake <map-nav-directory>".into());
    }
    let started = Instant::now();
    let mut map = NavigationMap::open(Path::new(&directory))?;
    map.bake_portal_graph()?;
    println!(
        "baked hierarchy for {directory} in {:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
