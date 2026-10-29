//! Measure worker startup and two requests after all installed graphs are loaded.
use glam::Vec3;
use navigation::worker::{Query, Request, Worker};
use std::path::PathBuf;
use std::time::Instant;

fn rss_kib() -> Result<u64, String> {
    let status = std::fs::read_to_string("/proc/self/status").map_err(|error| error.to_string())?;
    status
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmRSS:")
                .and_then(|value| value.split_whitespace().next())
                .and_then(|value| value.parse().ok())
        })
        .ok_or("VmRSS not found".into())
}

fn main() -> Result<(), String> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: hierarchy_startup NAV_ROOT")?,
    );
    let count = std::fs::read_dir(&root)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("portal-graph.json").is_file())
        .count();
    let before = rss_kib()?;
    let started = Instant::now();
    let mut worker = Worker::new(root);
    let startup = started.elapsed();
    let after = rss_kib()?;
    println!(
        "maps={count} startup_seconds={:.3} rss_before_kib={before} rss_after_kib={after} delta_kib={}",
        startup.as_secs_f64(),
        after.saturating_sub(before)
    );
    let queries = [
        (
            1,
            2660,
            Vec3::new(1063.1083, 0.2, 529.7743),
            Vec3::new(159.20139, 0.029205322, -2757.6824),
        ),
        (
            2,
            2519,
            Vec3::new(-257.87, 123.38, 489.73),
            Vec3::new(-257.87, 123.38, 489.73),
        ),
    ];
    let started = Instant::now();
    for (entity, map_id, from, to) in queries {
        worker.submit(Request {
            entity,
            id: 1,
            map_id,
            query: Query::Path { from, to },
        });
    }
    while !worker.is_idle() {
        for result in worker.poll() {
            println!(
                "entity={} response_seconds={:.3} result={}",
                result.entity,
                started.elapsed().as_secs_f64(),
                if result.result.is_ok() { "ok" } else { "error" }
            );
            result.result?;
        }
        std::thread::yield_now();
    }
    Ok(())
}
