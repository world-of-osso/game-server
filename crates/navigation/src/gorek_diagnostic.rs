//! Opt-in observation of the exact canonical Gorek failure; no movement or bake edits.
use super::*;
use std::io::Write;

#[test]
#[ignore = "requires canonical2519 assets and FOLLOWER_GOREK_DIAGNOSTICS output directory"]
fn neltharus_gorek_native_query_trace() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut trace = std::fs::File::create(output.join("native-query-trace.txt")).unwrap();
    let start = Vec3::new(70.6448, 72.6027, 779.064);
    // Exact13998 packet inputs: WoWheadfloor1 XY and its three observed nav surfaces.
    let cases = [
        (
            "gorek_source_floor",
            Vec3::new(-264.45807, 125.19722, 503.5),
        ),
        ("gorek_low_surface", Vec3::new(-264.45807, 0.2, 503.5)),
        (
            "gorek_high_surface",
            Vec3::new(-264.45807, 197.979995, 503.5),
        ),
        ("magmatusk_control", Vec3::new(-211.1, 99.34897, 932.7)),
    ];
    for (label, end) in cases {
        // Fresh map/query per case matches the production route CLI process boundary.
        let mut nav = NavigationMap::open(&root.join("2519")).unwrap();
        nav.load_around(start, end).unwrap();
        let filter = filter();
        let (start_ref, start_pos) = nav.nearest(start, &filter).unwrap();
        let (end_ref, end_pos) = nav.nearest(end, &filter).unwrap();
        let mut corridor = [0; MAX_PATH];
        let mut count = 0;
        // SAFETY: same owned runtime mesh/query, inputs and bounded arrays as query_corridor.
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
        status(result, "diagnostic path search").unwrap();
        assert!(count > 0);
        let last_ref = corridor[count as usize - 1];
        let reached_end = last_ref == end_ref;
        let mut last_point = [0.0; 3];
        let mut over_polygon = false;
        // SAFETY: last_ref was returned by this owned query against its loaded mesh.
        let projection = unsafe {
            (*nav.query).closestPointOnPoly(
                last_ref,
                end_pos.as_ptr(),
                last_point.as_mut_ptr(),
                &mut over_polygon,
            )
        };
        status(projection, "diagnostic partial endpoint").unwrap();
        let mut loaded: Vec<_> = nav.loaded.iter().copied().collect();
        loaded.sort_unstable();
        writeln!(trace, "case={label}").unwrap();
        writeln!(trace, "input_start={start:?} input_end={end:?}").unwrap();
        writeln!(trace, "nearest_start={start_pos:?} nearest_end={end_pos:?}").unwrap();
        writeln!(
            trace,
            "start_ref={start_ref} end_ref={end_ref} last_ref={last_ref}"
        )
        .unwrap();
        writeln!(trace, "status={result:#x} partial={} out_of_nodes={} buffer_too_small={} reached_end={reached_end} corridor_count={count}", result & DT_PARTIAL_RESULT != 0, result & DT_OUT_OF_NODES != 0, result & DT_BUFFER_TOO_SMALL != 0).unwrap();
        writeln!(
            trace,
            "partial_last_point={last_point:?} distance_to_target={}",
            Vec3::from(last_point).distance(Vec3::from(end_pos))
        )
        .unwrap();
        writeln!(trace, "loaded_wow_tiles={loaded:?}").unwrap();
        match nav.pull_corners(&start_pos, &last_point, &corridor[..count as usize]) {
            Ok(corners) => {
                let mut file =
                    std::fs::File::create(output.join(format!("{label}-partial-only.csv")))
                        .unwrap();
                for point in corners {
                    writeln!(file, "{},{},{}", point.x, point.y, point.z).unwrap();
                }
            }
            Err(error) => writeln!(trace, "partial_corner_export_error={error}").unwrap(),
        }
        eprintln!(
            "{label}: status={result:#x}, partial={}, out_of_nodes={}, buffer={}, reached_end={reached_end}, count={count}, partial_last={last_point:?}",
            result & DT_PARTIAL_RESULT != 0,
            result & DT_OUT_OF_NODES != 0,
            result & DT_BUFFER_TOO_SMALL != 0
        );
        if label == "magmatusk_control" {
            assert!(
                reached_end,
                "existing positive control must remain connected"
            );
        }
        if label == "gorek_source_floor" {
            assert!(
                !reached_end,
                "observed canonical failure changed; re-establish the fixture"
            );
        }
    }
}
