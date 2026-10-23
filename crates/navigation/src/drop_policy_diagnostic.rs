//! Read-only native edge evidence for a proposed one-way-drop policy.
use super::gorek_diagnostic::{connected_polygons, tile_and_poly};
use super::*;
use std::io::{BufWriter, Write};

fn linked_edges(nav: &NavigationMap, reference: dtPolyRef) -> u8 {
    let (tile, poly) = tile_and_poly(nav, reference);
    let mut mask = 0;
    // SAFETY: immutable owned native tile; serialized links may be only four-byte aligned.
    unsafe {
        let mut index = (*poly).firstLink;
        while index != DT_NULL_LINK {
            let link = std::ptr::read_unaligned((*tile).links.add(index as usize));
            if link.ref_ != 0 {
                mask |= 1 << link.edge;
            }
            index = link.next;
        }
    }
    mask
}

#[test]
#[ignore = "requires canonical four-map assets and FOLLOWER_COMPONENT_DIAGNOSTICS"]
fn native_source_component_walkable_boundary_edges() {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_DIAGNOSTICS").unwrap());
    let wow = |x: f32, y: f32, z: f32| Vec3::new(x, z, -y);
    for (map_id, source) in [
        (2527, wow(-0.370431, 228.269, 20.8162)),
        (2526, wow(1410.22, -2771.71, 955.637)),
        (2651, wow(239.839, 75.8715, 94.7386)),
        (2661, wow(2650.88, -4835.88, 101.042)),
    ] {
        let mut nav = NavigationMap::open(&root.join(map_id.to_string())).unwrap();
        let mut tiles = Vec::new();
        for entry in std::fs::read_dir(&nav.dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|ext| ext != "nav") {
                continue;
            }
            let (row, col) = path
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .split_once('_')
                .unwrap();
            tiles.push((row.parse().unwrap(), col.parse().unwrap()));
        }
        tiles.sort_unstable();
        for tile in tiles {
            nav.load_tile(tile).unwrap();
        }
        let filter = filter();
        let (source_ref, _) = nav.nearest(source, &filter).unwrap();
        let component = connected_polygons(&nav, source_ref, &filter);
        let mut references: Vec<_> = component.keys().copied().collect();
        references.sort_unstable();
        let mut csv = BufWriter::new(
            std::fs::File::create(
                output.join(format!("{map_id}-source-native-boundary-edges.csv")),
            )
            .unwrap(),
        );
        writeln!(csv, "reference,edge,ax,ay,az,bx,by,bz").unwrap();
        let mut edge_count = 0;
        for reference in references {
            let mask = linked_edges(&nav, reference);
            let (tile, poly) = tile_and_poly(&nav, reference);
            // SAFETY: validated polygon and tile vertices owned by unchanged native mesh.
            unsafe {
                let poly = std::ptr::read_unaligned(poly);
                let count = usize::from(poly.vertCount);
                for edge in 0..count {
                    if mask & (1 << edge) != 0 {
                        continue;
                    }
                    let a = std::slice::from_raw_parts(
                        (*tile).verts.add(usize::from(poly.verts[edge]) * 3),
                        3,
                    );
                    let b = std::slice::from_raw_parts(
                        (*tile)
                            .verts
                            .add(usize::from(poly.verts[(edge + 1) % count]) * 3),
                        3,
                    );
                    writeln!(
                        csv,
                        "{reference},{edge},{},{},{},{},{},{}",
                        a[0], a[1], a[2], b[0], b[1], b[2]
                    )
                    .unwrap();
                    edge_count += 1;
                }
            }
        }
        csv.flush().unwrap();
        eprintln!(
            "map={map_id} source_component={} native_unlinked_boundary_edges={edge_count}",
            component.len()
        );
    }
}
