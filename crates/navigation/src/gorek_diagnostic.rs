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

#[test]
#[ignore = "requires canonical2521 assets and FOLLOWER_GOREK_DIAGNOSTICS"]
fn ruby_resource_limited_tail_native_component_probe() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut trace = std::fs::File::create(output.join("ruby-tail-native-status.txt")).unwrap();
    let entrance = Vec3::new(1386.2, 131.0, 169.719);
    let target = Vec3::new(1773.8756, 313.123, -18.689396);
    let actual_tail = Vec3::new(1641.4063, 164.04453, 4.6875);
    let mut nav = NavigationMap::open(&root.join("2521")).unwrap();
    // Retain the entrance query's loaded mesh, as the production worker does.
    nav.load_around(entrance, target).unwrap();
    let filter = filter();
    let (entrance_ref, _) = nav.nearest(entrance, &filter).unwrap();
    let (target_ref, end_pos) = nav.nearest(target, &filter).unwrap();
    let parents = connected_polygons(&nav, entrance_ref, &filter);
    let mut loaded: Vec<_> = nav.loaded.iter().copied().collect();
    loaded.sort_unstable();
    writeln!(trace, "loaded_wow_tiles={loaded:?}").unwrap();
    writeln!(
        trace,
        "entrance_component_polygons={} target_in_component={}",
        parents.len(),
        parents.contains_key(&target_ref)
    )
    .unwrap();
    for (label, start) in [("entrance", entrance), ("actual_mover_tail", actual_tail)] {
        let (start_ref, start_pos) = nav.nearest(start, &filter).unwrap();
        let mut corridor = [0; MAX_PATH];
        let mut count = 0;
        // SAFETY: initialized owned native query, valid refs and bounded buffers.
        let result = unsafe {
            (*nav.query).findPath(
                start_ref,
                target_ref,
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                &filter,
                corridor.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
            )
        };
        status(result, "Ruby native tail diagnostic").unwrap();
        assert!(count > 0);
        let last_ref = corridor[count as usize - 1];
        let mut last_point = [0.0; 3];
        // SAFETY: returned native reference and three-float arrays.
        status(
            unsafe {
                (*nav.query).closestPointOnPoly(
                    last_ref,
                    end_pos.as_ptr(),
                    last_point.as_mut_ptr(),
                    std::ptr::null_mut(),
                )
            },
            "Ruby native partial endpoint",
        )
        .unwrap();
        writeln!(trace, "case={label} status={result:#x} reached={} polys={count} start_in_entrance_component={} nearest_start={start_pos:?} nearest_target={end_pos:?} native_last={last_point:?}", last_ref == target_ref, parents.contains_key(&start_ref)).unwrap();
        match nav.path(start, target) {
            Ok(path) => writeln!(
                trace,
                "production_complete={} production_points={} production_last={:?}",
                path.complete,
                path.points.len(),
                path.points.last()
            )
            .unwrap(),
            Err(error) => writeln!(trace, "production_error={error}").unwrap(),
        }
    }
    eprintln!(
        "Ruby component={} target_connected={}; diagnostic={}",
        parents.len(),
        parents.contains_key(&target_ref),
        output.join("ruby-tail-native-status.txt").display()
    );
}

#[test]
#[ignore = "requires canonical2521 assets and FOLLOWER_GOREK_DIAGNOSTICS"]
fn ruby_disconnected_component_geometry_export() {
    use std::io::Write;
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut nav = NavigationMap::open(&root.join("2521")).unwrap();
    load_all_diagnostic_tiles(&mut nav);
    let filter = filter();
    let entrance = Vec3::new(1386.2, 131.0, 169.719);
    let target = Vec3::new(1773.8756, 313.123, -18.689396);
    let (source_ref, _) = nav.nearest(entrance, &filter).unwrap();
    let (target_ref, _) = nav.nearest(target, &filter).unwrap();
    let source = connected_polygons(&nav, source_ref, &filter);
    let destination = connected_polygons(&nav, target_ref, &filter);
    let intersection = source
        .keys()
        .filter(|key| destination.contains_key(key))
        .count();
    let mut trace = std::fs::File::create(output.join("ruby-component-geometry.txt")).unwrap();
    writeln!(trace, "loaded_tiles={} source_polygons={} target_polygons={} intersection={intersection} source_reaches_target={} target_reaches_source={}", nav.loaded.len(), source.len(), destination.len(), source.contains_key(&target_ref), destination.contains_key(&source_ref)).unwrap();
    for (label, point) in [
        ("entrance", entrance),
        ("runtime_erkhart_190485", target),
        (
            "actual_repeat_endpoint",
            Vec3::new(1641.4063, 167.84453, 11.979166),
        ),
        (
            "tdb_melidrussa_188252",
            Vec3::new(1551.33, 129.782, 257.688),
        ),
        ("tdb_kokia_189232", Vec3::new(1643.71, 331.257, 131.701)),
        ("tdb_kyrakka_199790", Vec3::new(1864.12, 332.153, -14.5156)),
        ("tdb_drake_9001663", Vec3::new(1551.32, 232.711, 266.693)),
    ] {
        match nav.nearest(point, &filter) {
            Ok((reference, projected)) => writeln!(trace, "point={label} requested={point:?} reference={reference} projected={projected:?} in_source={} in_target={}", source.contains_key(&reference), destination.contains_key(&reference)).unwrap(),
            Err(error) => writeln!(trace, "point={label} requested={point:?} projection_error={error}").unwrap(),
        }
    }
    write_diagnostic_component_polygons(&nav, &source, &output.join("ruby-source-polygons.csv"));
    write_diagnostic_component_polygons(
        &nav,
        &destination,
        &output.join("ruby-target-polygons.csv"),
    );
    eprintln!(
        "Ruby full-map geometry: source={} target={} intersection={intersection}",
        source.len(),
        destination.len()
    );
}

fn load_all_diagnostic_tiles(nav: &mut NavigationMap) {
    let mut tiles = Vec::new();
    for entry in std::fs::read_dir(&nav.dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|extension| extension != "nav") {
            continue;
        }
        let (row, col) = path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .split_once('_')
            .unwrap();
        tiles.push((row.parse::<u32>().unwrap(), col.parse::<u32>().unwrap()));
    }
    tiles.sort_unstable();
    for tile in tiles {
        nav.load_tile(tile).unwrap();
    }
}

fn write_diagnostic_component_polygons(
    nav: &NavigationMap,
    component: &std::collections::HashMap<dtPolyRef, dtPolyRef>,
    output: &Path,
) {
    use std::io::{BufWriter, Write};
    let mut csv = BufWriter::new(std::fs::File::create(output).unwrap());
    writeln!(
        csv,
        "reference,vertices,x0,y0,z0,x1,y1,z1,x2,y2,z2,x3,y3,z3,x4,y4,z4,x5,y5,z5"
    )
    .unwrap();
    let mut references: Vec<_> = component.keys().copied().collect();
    references.sort_unstable();
    for reference in references {
        let (tile, polygon) = tile_and_poly(nav, reference);
        // SAFETY: immutable owned Detour mesh, validated native polygon vertices.
        unsafe {
            let polygon = std::ptr::read_unaligned(polygon);
            let count = usize::from(polygon.vertCount);
            assert!(count <= polygon.verts.len());
            write!(csv, "{reference},{count}").unwrap();
            for index in &polygon.verts[..count] {
                let point =
                    std::slice::from_raw_parts((*tile).verts.add(usize::from(*index) * 3), 3);
                write!(csv, ",{},{},{}", point[0], point[1], point[2]).unwrap();
            }
        }
        writeln!(csv).unwrap();
    }
    csv.flush().unwrap();
}

fn tile_and_poly(nav: &NavigationMap, reference: dtPolyRef) -> (*const dtMeshTile, *const dtPoly) {
    let mut tile = std::ptr::null();
    let mut poly = std::ptr::null();
    // SAFETY: reference is from this owned mesh's nearest query or its native links.
    let result = unsafe { (*nav.mesh).getTileAndPolyByRef(reference, &mut tile, &mut poly) };
    status(result, "diagnostic graph polygon").unwrap();
    (tile, poly)
}

fn linked_polygons(
    nav: &NavigationMap,
    reference: dtPolyRef,
    filter: &dtQueryFilter,
) -> Vec<dtPolyRef> {
    let (tile, poly) = tile_and_poly(nav, reference);
    let mut output = Vec::new();
    // SAFETY: native links and headers are owned by the initialized immutable mesh.
    unsafe {
        let mut index = (*poly).firstLink;
        while index != DT_NULL_LINK {
            assert!(index < (*(*tile).header).maxLinkCount as u32);
            // Detour's serialized sections use four-byte alignment, while
            // 64-bit dtPolyRef makes Rust's dtLink alignment eight bytes.
            let link = std::ptr::read_unaligned((*tile).links.add(index as usize));
            if link.ref_ != 0 {
                let (_, neighbor) = tile_and_poly(nav, link.ref_);
                let flags = (*neighbor).flags;
                let included = flags & filter.m_includeFlags != 0;
                let excluded = flags & filter.m_excludeFlags != 0;
                if included && !excluded {
                    output.push(link.ref_);
                }
            }
            index = link.next;
        }
    }
    output.sort_unstable();
    output
}

fn connected_polygons(
    nav: &NavigationMap,
    start: dtPolyRef,
    filter: &dtQueryFilter,
) -> std::collections::HashMap<dtPolyRef, dtPolyRef> {
    use std::collections::{HashMap, VecDeque, hash_map::Entry};
    let mut parents = HashMap::from([(start, 0)]);
    let mut queue = VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        for neighbor in linked_polygons(nav, current, filter) {
            if let Entry::Vacant(entry) = parents.entry(neighbor) {
                entry.insert(current);
                queue.push_back(neighbor);
            }
        }
    }
    parents
}

#[test]
#[ignore = "requires canonical2519 assets and FOLLOWER_GOREK_DIAGNOSTICS output directory"]
fn neltharus_gorek_native_connectivity_and_source_limit_probe() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut trace = std::fs::File::create(output.join("connectivity-probe.txt")).unwrap();
    let start = Vec3::new(70.6448, 72.6027, 779.064);
    let end = Vec3::new(-264.45807, 125.19722, 503.5);
    let mut nav = NavigationMap::open(&root.join("2519")).unwrap();
    nav.load_around(start, end).unwrap();
    let filter = filter();
    let (start_ref, start_pos) = nav.nearest(start, &filter).unwrap();
    let (end_ref, end_pos) = nav.nearest(end, &filter).unwrap();
    let parents = connected_polygons(&nav, start_ref, &filter);
    let graph_reaches_target = parents.contains_key(&end_ref);
    writeln!(
        trace,
        "native_link_graph_component_polygons={} target_in_component={graph_reaches_target}",
        parents.len()
    )
    .unwrap();
    if graph_reaches_target {
        let mut refs = vec![end_ref];
        while *refs.last().unwrap() != start_ref {
            refs.push(parents[refs.last().unwrap()]);
        }
        refs.reverse();
        writeln!(trace, "native_link_graph_corridor_polygons={}", refs.len()).unwrap();
        match nav.pull_corners(&start_pos, &end_pos, &refs) {
            Ok(points) => {
                let mut file = std::fs::File::create(
                    output.join("native-link-graph-route-diagnostic-only.csv"),
                )
                .unwrap();
                for point in points {
                    writeln!(file, "{},{},{}", point.x, point.y, point.z).unwrap();
                }
            }
            Err(error) => writeln!(trace, "native_link_graph_corner_error={error}").unwrap(),
        }
    }
    // Diagnostic only, never a production retry/fallback. Pinned DetourNode.h makes
    // dtNodeIndex unsigned short/DT_NULL_IDX=~0; Query.cpp171-174 accepts at most that.
    let source_max_nodes = i32::from(u16::MAX);
    // SAFETY: same owned query/mesh, explicit supported diagnostic resource ceiling.
    status(
        unsafe { (*nav.query).init(nav.mesh, source_max_nodes) },
        "source max-node diagnostic init",
    )
    .unwrap();
    let mut corridor = [0; MAX_PATH];
    let mut count = 0;
    // SAFETY: all refs, positions and output buffers belong to this owned mesh/query.
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
    status(result, "source max-node diagnostic path").unwrap();
    let reached = count > 0 && corridor[count as usize - 1] == end_ref;
    writeln!(trace, "diagnostic_only_max_nodes={source_max_nodes} status={result:#x} partial={} out_of_nodes={} buffer_too_small={} reached_end={reached} corridor_count={count}", result & DT_PARTIAL_RESULT != 0, result & DT_OUT_OF_NODES != 0, result & DT_BUFFER_TOO_SMALL != 0).unwrap();
    if reached {
        assert!(
            graph_reaches_target,
            "native path must stay in native link component"
        );
        let points = nav
            .pull_corners(&start_pos, &end_pos, &corridor[..count as usize])
            .unwrap();
        let mut file =
            std::fs::File::create(output.join("source-max-node-route-diagnostic-only.csv"))
                .unwrap();
        for point in points {
            writeln!(file, "{},{},{}", point.x, point.y, point.z).unwrap();
        }
    }
    eprintln!(
        "Gorek native-link component={} target_in_component={graph_reaches_target}; diagnostic maxnodes={source_max_nodes} status={result:#x} reached={reached} count={count}",
        parents.len()
    );
}
