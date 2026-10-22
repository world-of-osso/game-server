//! Read-only full-map component evidence; no generated connections or bake changes.
use super::gorek_diagnostic::{connected_polygons, tile_and_poly};
use super::*;
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

struct Case {
    label: &'static str,
    start: Vec3,
    target: Vec3,
}

fn cases(map: u32) -> Vec<Case> {
    let wow = |x: f32, y: f32, z: f32| Vec3::new(x, z, -y);
    match map {
        2527 => vec![
            Case {
                label: "halls_189722",
                start: wow(-0.370431, 228.269, 20.8162),
                target: wow(-353.12274, -155.7234, -67.9066),
            },
            Case {
                label: "halls_189727",
                start: wow(-0.370431, 228.269, 20.8162),
                target: wow(-315.97385, 161.89757, -100.404625),
            },
        ],
        2526 => vec![Case {
            label: "vexamus_194181",
            start: wow(1410.22, -2771.71, 955.637),
            target: wow(1815.2512, -2783.9756, 904.01917),
        }],
        2651 => vec![
            Case {
                label: "darkflame_208743",
                start: wow(239.839, 75.8715, 94.7386),
                target: wow(406.7775, -155.27142, 56.68355),
            },
            Case {
                label: "darkflame_208747",
                start: wow(239.839, 75.8715, 94.7386),
                target: wow(188.47888, -547.81714, 59.8),
            },
        ],
        2661 => vec![Case {
            label: "goldie_214661",
            start: wow(2650.88, -4835.88, 101.042),
            target: wow(2649.0535, -5013.1484, 110.600006),
        }],
        // This packet explicitly uses Bevy coordinates, not WoW XYZ.
        2515 => vec![
            Case {
                label: "azure_rune_29001884_to_pack_29001895",
                start: Vec3::new(-5353.02, 508.083, -1117.58),
                target: Vec3::new(-5374.25, 425.248, -1098.11),
            },
            Case {
                label: "azure_book_29002007_to_receiver_29001886",
                start: Vec3::new(-5303.22, 509.262, -1075.58),
                target: Vec3::new(-5336.61, 426.341, -1113.19),
            },
        ],
        _ => unreachable!("diagnostic map list is explicit"),
    }
}

fn load_complete_map(nav: &mut NavigationMap) {
    let mut tiles = Vec::new();
    for entry in std::fs::read_dir(&nav.dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "nav") {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap();
        let (row, col) = name.split_once('_').unwrap();
        tiles.push((row.parse::<u32>().unwrap(), col.parse::<u32>().unwrap()));
    }
    tiles.sort_unstable();
    for tile in tiles {
        nav.load_tile(tile).unwrap();
    }
}

fn add_component_masks(
    nav: &NavigationMap,
    cases: &[Case],
    trace: &mut impl Write,
) -> BTreeMap<dtPolyRef, u32> {
    let filter = filter();
    let mut masks = BTreeMap::new();
    for (index, case) in cases.iter().enumerate() {
        let (start_ref, start_pos) = nav.nearest(case.start, &filter).unwrap();
        let (end_ref, end_pos) = nav.nearest(case.target, &filter).unwrap();
        let source = connected_polygons(nav, start_ref, &filter);
        let target = connected_polygons(nav, end_ref, &filter);
        let intersection = source
            .keys()
            .filter(|key| target.contains_key(*key))
            .count();
        writeln!(trace, "case={} source_mask={} target_mask={} source_polygons={} target_polygons={} intersection={intersection} source_reaches_target={} target_reaches_source={} source_ref={start_ref} target_ref={end_ref} nearest_source={start_pos:?} nearest_target={end_pos:?}", case.label, 1u32 << (index*2), 1u32 << (index*2+1), source.len(), target.len(), source.contains_key(&end_ref), target.contains_key(&start_ref)).unwrap();
        for reference in source.keys() {
            *masks.entry(*reference).or_default() |= 1u32 << (index * 2);
        }
        for reference in target.keys() {
            *masks.entry(*reference).or_default() |= 1u32 << (index * 2 + 1);
        }
    }
    masks
}

fn write_native_polygons(nav: &NavigationMap, masks: BTreeMap<dtPolyRef, u32>, output: &Path) {
    let mut csv = BufWriter::new(std::fs::File::create(output).unwrap());
    writeln!(
        csv,
        "reference,component_mask,vertices,x0,y0,z0,x1,y1,z1,x2,y2,z2,x3,y3,z3,x4,y4,z4,x5,y5,z5"
    )
    .unwrap();
    for (reference, mask) in masks {
        let (tile, poly) = tile_and_poly(nav, reference);
        // SAFETY: initialized owned mesh remains immutable; native polygons contain
        // at most six validated vertex indices, each referencing its tile's vertices.
        unsafe {
            let polygon = std::ptr::read_unaligned(poly);
            let count = usize::from(polygon.vertCount);
            assert!(count <= 6);
            write!(csv, "{reference},{mask},{count}").unwrap();
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

#[test]
#[ignore = "requires canonical five-map assets and FOLLOWER_COMPONENT_DIAGNOSTICS"]
fn full_map_disconnected_component_polygon_export() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_DIAGNOSTICS").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut trace =
        BufWriter::new(std::fs::File::create(output.join("native-components.txt")).unwrap());
    for map_id in [2527, 2526, 2651, 2661, 2515] {
        let mut nav = NavigationMap::open(&root.join(map_id.to_string())).unwrap();
        load_complete_map(&mut nav);
        writeln!(
            trace,
            "map={map_id} full_map_loaded_wow_tiles={}",
            nav.loaded.len()
        )
        .unwrap();
        let masks = add_component_masks(&nav, &cases(map_id), &mut trace);
        eprintln!(
            "map={map_id} tiles={} exported_polygons={}",
            nav.loaded.len(),
            masks.len()
        );
        write_native_polygons(
            &nav,
            masks,
            &output.join(format!("{map_id}-component-polygons.csv")),
        );
        trace.flush().unwrap();
    }
}
