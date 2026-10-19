//! Manual Stockade trace renderer; never installed as a server system.
use std::path::PathBuf;

#[path = "../bot_map/mod.rs"]
mod bot_map;
use bot_map::{render_trace, scene::read_scene};

fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let trace = PathBuf::from(args.next().ok_or("usage: bot-map TRACE -o OUTPUT")?);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("-o")) {
        return Err("usage: bot-map TRACE -o OUTPUT".into());
    }
    let output = PathBuf::from(args.next().ok_or("missing output path")?);
    let mut database = PathBuf::from("data/world.db");
    let mut navmesh = PathBuf::from("data/navmesh/34");
    while let Some(flag) = args.next() {
        let value = PathBuf::from(args.next().ok_or("missing option value")?);
        match flag.to_str() {
            Some("--world-db") => database = value,
            Some("--navmesh") => navmesh = value,
            _ => return Err(format!("Unknown option: {}", flag.to_string_lossy())),
        }
    }
    let input = std::fs::read_to_string(&trace).map_err(|e| format!("{}: {e}", trace.display()))?;
    let scene = read_scene(&database, &navmesh)?;
    std::fs::write(&output, render_trace(&input, &scene)?)
        .map_err(|e| format!("{}: {e}", output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bot_map::scene::Scene;

    #[test]
    #[ignore = "behavioral RED: opted-in Stockade trace renderer not implemented"]
    fn concrete_trace_draws_paths_time_ticks_and_death_marker() {
        let trace = concat!(
            "{\"event\":\"run_start\",\"run_id\":\"stockade-fixture\",\"revision\":\"abc123\",\"map_id\":34,\"at_ms\":1000}\n",
            "{\"event\":\"pose\",\"actor\":\"101\",\"name\":\"Tank\",\"position\":[10,2,20],\"facing\":0,\"at_ms\":1000}\n",
            "{\"event\":\"pose\",\"actor\":\"101\",\"name\":\"Tank\",\"position\":[15,2,25],\"facing\":1,\"at_ms\":11000}\n",
            "{\"event\":\"death\",\"actor\":\"101\",\"position\":[15,2,25],\"at_ms\":11000}\n",
            "{\"event\":\"run_end\",\"at_ms\":12000}\n"
        );
        let svg = render_trace(trace, &Scene::default()).unwrap();
        assert!(svg.contains("10,20 15,25"), "trace points absent: {svg}");
        assert!(svg.contains("death"), "death marker absent");
        assert!(svg.contains("10s"), "periodic time label absent");
        assert!(svg.contains("stockade-fixture") && svg.contains("abc123"));
    }
}
