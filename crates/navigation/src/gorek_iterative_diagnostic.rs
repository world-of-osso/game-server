//! Diagnostic-only partial-corridor walking with the unchanged production query pool.
use super::*;
use std::io::Write;

#[test]
#[ignore = "requires canonical2519 assets"]
fn neltharus_gorek_production_path_preserves_walkable_progress() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let mut nav = NavigationMap::open(&root.join("2519")).unwrap();
    let start = Vec3::new(70.6448, 72.6027, 779.064);
    let target = Vec3::new(-264.45807, 125.19722, 503.5);
    let path = nav
        .path(start, target)
        .expect("nonempty Gorek corridor must remain available for movement and repathing");
    assert!(!path.complete, "the first segment is not the entire route");
    assert!(path.points.len() > 2);
    let endpoint = *path.points.last().unwrap();
    assert!(endpoint.distance(start) > 100.0);
    assert!(
        endpoint.distance(target) > 50.0,
        "do not pull a partial corridor to the requested target"
    );
    assert!(
        path.into_complete().is_err(),
        "admission must not mistake partial movement for full connectivity"
    );
}

#[test]
#[ignore = "requires canonical placement assets and FOLLOWER_GOREK_DIAGNOSTICS"]
fn authored_placement_native_status_probe() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut trace =
        std::fs::File::create(output.join("authored-placement-native-status.txt")).unwrap();
    let wow = |x: f32, y: f32, z: f32| Vec3::new(x, z, -y);
    let halls = wow(-0.370431, 228.269, 20.8162);
    let nokhud = wow(-570.405, 2189.37, 431.296);
    for (map_id, entry, start, end) in [
        (2527, 189722, halls, wow(-353.12274, -155.7234, -67.9066)),
        (2527, 189727, halls, wow(-315.97385, 161.89757, -100.404625)),
        (
            2526,
            194181,
            wow(1410.22, -2771.71, 955.637),
            wow(1815.2512, -2783.9756, 904.01917),
        ),
        (2516, 186151, nokhud, wow(-536.05316, 4163.211, 381.20233)),
        (2516, 186338, nokhud, wow(-2095.4968, 4335.8853, 536.04694)),
        (2516, 186616, nokhud, wow(-1167.5642, 1356.6361, 390.72348)),
        (
            2521,
            190485,
            wow(1386.2, -169.719, 131.0),
            wow(1773.8756, 18.689396, 313.123),
        ),
        (
            2651,
            208743,
            wow(239.839, 75.8715, 94.7386),
            wow(406.7775, -155.27142, 56.68355),
        ),
        (
            2651,
            208747,
            wow(239.839, 75.8715, 94.7386),
            wow(188.47888, -547.81714, 59.8),
        ),
    ] {
        let mut nav = NavigationMap::open(&root.join(map_id.to_string())).unwrap();
        nav.load_around(start, end).unwrap();
        let filter = filter();
        let (start_ref, start_pos) = nav.nearest(start, &filter).unwrap();
        let (end_ref, end_pos) = nav.nearest(end, &filter).unwrap();
        let mut corridor = [0; MAX_PATH];
        let mut count = 0;
        // SAFETY: owned native query, valid refs and bounded output arrays.
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
        status(result, "authored placement diagnostic").unwrap();
        assert!(count > 0);
        let last_ref = corridor[count as usize - 1];
        let mut last_point = [0.0; 3];
        // SAFETY: final native corridor ref and three-float input/output arrays.
        status(
            unsafe {
                (*nav.query).closestPointOnPoly(
                    last_ref,
                    end_pos.as_ptr(),
                    last_point.as_mut_ptr(),
                    std::ptr::null_mut(),
                )
            },
            "authored placement partial end",
        )
        .unwrap();
        let reached = last_ref == end_ref;
        writeln!(trace, "map={map_id} entry={entry} status={result:#x} partial={} out_of_nodes={} buffer_too_small={} reached={reached} polys={count} nearest_start={start_pos:?} nearest_end={end_pos:?} last_point={last_point:?}", result & DT_PARTIAL_RESULT != 0, result & DT_OUT_OF_NODES != 0, result & DT_BUFFER_TOO_SMALL != 0).unwrap();
        eprintln!("map={map_id} entry={entry} status={result:#x} reached={reached} polys={count}");
    }
}

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
