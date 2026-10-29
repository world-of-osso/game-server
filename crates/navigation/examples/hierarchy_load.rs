//! Measure graph artifact loading, including tile hash verification.
use navigation::hierarchy::PortalGraph;
use std::path::Path;
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
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: hierarchy_load <map directory>")?;
    let before = rss_kib()?;
    let start = Instant::now();
    let graph = PortalGraph::read_artifact(Path::new(&path))?;
    let elapsed = start.elapsed();
    let after = rss_kib()?;
    println!(
        "load_seconds={:.6} rss_before_kib={before} rss_after_kib={after} graph_delta_kib={} clusters={} nodes={} edges={}",
        elapsed.as_secs_f64(),
        after.saturating_sub(before),
        graph.clusters.len(),
        graph.nodes.len(),
        graph
            .nodes
            .iter()
            .map(|node| node.edges.len())
            .sum::<usize>()
    );
    std::hint::black_box(graph);
    Ok(())
}
