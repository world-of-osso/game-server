//! Diagnostic-only partial-corridor walking with the unchanged production query pool.
use super::*;
use std::io::Write;

#[test]
#[ignore = "requires canonical2519 assets and FOLLOWER_GOREK_DIAGNOSTICS output directory"]
fn neltharus_gorek_iterative_existing_pool_probe() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut trace = std::fs::File::create(output.join("iterative-existing-pool.txt")).unwrap();
    let mut current = Vec3::new(70.6448, 72.6027, 779.064);
    let target = Vec3::new(-264.45807, 125.19722, 503.5);
    let mut visited = HashSet::new();
    let mut reached = false;
    // This bounds diagnostic execution, not any production path or movement policy.
    const MAX_DIAGNOSTIC_SEGMENTS: usize = 128;
    for segment in 0..MAX_DIAGNOSTIC_SEGMENTS {
        // Fresh query matches independently submitted path requests with no reused corridor.
        let mut nav = NavigationMap::open(&root.join("2519")).unwrap();
        nav.load_around(current, target).unwrap();
        let filter = filter();
        let (start_ref, start_pos) = nav.nearest(current, &filter).unwrap();
        let (end_ref, end_pos) = nav.nearest(target, &filter).unwrap();
        let mut corridor = [0; MAX_PATH];
        let mut count = 0;
        // SAFETY: same owned native mesh/query and bounded buffers as production.
        let result = unsafe {
            (*nav.query).findPath(
                start_ref,
                end_ref,
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                &filter,
                corridor.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
            )
        };
        status(result, "iterative diagnostic search").unwrap();
        assert!(count > 0);
        let last_ref = corridor[count as usize - 1];
        let mut last_point = [0.0; 3];
        // SAFETY: last_ref came from this query; target and output are three floats.
        status(
            unsafe {
                (*nav.query).closestPointOnPoly(
                    last_ref,
                    end_pos.as_ptr(),
                    last_point.as_mut_ptr(),
                    std::ptr::null_mut(),
                )
            },
            "iterative diagnostic endpoint",
        )
        .unwrap();
        let next = Vec3::from_array(last_point);
        reached = last_ref == end_ref;
        let points = nav
            .pull_corners(&start_pos, &last_point, &corridor[..count as usize])
            .unwrap();
        let mut csv =
            std::fs::File::create(output.join(format!("iterative-segment-{segment:03}.csv")))
                .unwrap();
        for point in &points {
            writeln!(csv, "{},{},{}", point.x, point.y, point.z).unwrap();
        }
        writeln!(trace, "segment={segment} start={start_pos:?} endpoint={last_point:?} status={result:#x} corridor_polygons={count} corners={} reached={reached}", points.len()).unwrap();
        assert_eq!(
            result & DT_BUFFER_TOO_SMALL,
            0,
            "diagnostic corridor truncated"
        );
        if reached {
            break;
        }
        // Detect exact no-progress/cycles without introducing movement tolerances.
        let key = (next.x.to_bits(), next.y.to_bits(), next.z.to_bits());
        if next == current || !visited.insert(key) {
            writeln!(trace, "stopped=no_progress_or_cycle").unwrap();
            break;
        }
        current = next;
    }
    writeln!(
        trace,
        "existing_pool={MAX_NODES} iterative_reached={reached}"
    )
    .unwrap();
    eprintln!(
        "Gorek unchanged-pool iterative diagnostic reached={reached}; trace={}",
        output.join("iterative-existing-pool.txt").display()
    );
}
