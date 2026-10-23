//! Existing collision-reader evidence around measured native component boundaries.
use super::*;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, BufWriter, Write};

fn overlaps(triangle: &[Vec3; 3], min: Vec3, max: Vec3) -> bool {
    let lower = triangle[0].min(triangle[1]).min(triangle[2]);
    let upper = triangle[0].max(triangle[1]).max(triangle[2]);
    lower.cmple(max).all() && upper.cmpge(min).all()
}

fn export_geometry(
    geometry: &los::bake::CollectedGeometry,
    min: Vec3,
    max: Vec3,
    output: &mut impl Write,
) -> usize {
    let obstacles: HashSet<_> = geometry
        .obstacles
        .iter()
        .map(|tri| tri.map(|point| point.to_array().map(f32::to_bits)))
        .collect();
    let mut count = 0;
    for triangle in &geometry.triangles {
        if !overlaps(triangle, min, max) {
            continue;
        }
        let normal = (triangle[1] - triangle[0])
            .cross(triangle[2] - triangle[0])
            .normalize_or_zero();
        let angle = normal.y.abs().clamp(0.0, 1.0).acos().to_degrees();
        let obstacle = obstacles.contains(&triangle.map(|p| p.to_array().map(f32::to_bits)));
        write!(output, "{angle},{obstacle}").unwrap();
        for point in triangle {
            write!(output, ",{},{},{}", point.x, point.y, point.z).unwrap();
        }
        writeln!(output).unwrap();
        count += 1;
    }
    count
}

fn trace_boundary_wmo_sources(data: &Path, wdt: u32, min: Vec3, max: Vec3, trace: &mut impl Write) {
    use los::bake::adt::{self, ModelRef};
    let wdt_data = std::fs::read(data.join(format!("terrain/{wdt}.wdt"))).unwrap();
    let mut placements = BTreeMap::new();
    let tiles = adt::wdt_tiles(&wdt_data).unwrap();
    let a = tile_of(min);
    let b = tile_of(max);
    for row in a.0.min(b.0).saturating_sub(1)..=(a.0.max(b.0) + 1).min(63) {
        for col in a.1.min(b.1).saturating_sub(1)..=(a.1.max(b.1) + 1).min(63) {
            let tile = tiles[(col * 64 + row) as usize];
            if tile.obj0 == 0 {
                continue;
            }
            let path = data.join(format!("terrain/{}.adt", tile.obj0));
            let (_, wmos) = adt::placements(&std::fs::read(&path).unwrap()).unwrap();
            for placement in wmos {
                placements.insert(placement.unique_id, (path.clone(), placement));
            }
        }
    }
    for (_, (path, placement)) in placements {
        let ModelRef::Fdid(fdid) = &placement.model else {
            panic!("diagnostic requires sourced FDID WMO placement");
        };
        let root_path = data.join(format!("models/{fdid}.wmo"));
        let root = los::bake::wmo::WmoRoot::parse(&std::fs::read(&root_path).unwrap()).unwrap();
        let world = placement.world_from_local();
        let mut hits = Vec::new();
        for &group in &root.group_fdids {
            let group_path = data.join(format!("models/{group}.wmo"));
            let (triangles, _) = root
                .group_triangles(&std::fs::read(group_path).unwrap())
                .unwrap();
            let count = triangles
                .iter()
                .filter(|triangle| {
                    let transformed = triangle.map(|p| world.transform_point3(p));
                    overlaps(&transformed, min, max)
                })
                .count();
            if count > 0 {
                hits.push((group, count));
            }
        }
        if !hits.is_empty() {
            let selected: Vec<_> = root.doodads(placement.doodad_set).map(|d| d.fdid).collect();
            writeln!(trace, "wmo_source obj0={} placement={placement:?} root={} render_groups_overlapping_window={hits:?} selected_set={} selected_doodad_fdids={selected:?} qualification=render-group-overlap-not-collision-identity", path.display(), root_path.display(), placement.doodad_set).unwrap();
        }
    }
}

#[test]
#[ignore = "requires FOLLOWER_BOUNDARY_WINDOWS CSV and local source geometry"]
fn authored_collision_at_native_component_boundaries() {
    let windows = PathBuf::from(std::env::var_os("FOLLOWER_BOUNDARY_WINDOWS").unwrap());
    let data = PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_ASSETS").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_DIAGNOSTICS").unwrap());
    let mut trace =
        BufWriter::new(std::fs::File::create(output.join("boundary-geometry.txt")).unwrap());
    let mut lines = BufReader::new(std::fs::File::open(windows).unwrap()).lines();
    assert_eq!(
        lines.next().unwrap().unwrap(),
        "label,map,wdt,min_x,min_y,min_z,max_x,max_y,max_z"
    );
    for line in lines {
        let line = line.unwrap();
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 9);
        let label = fields[0];
        let map: u32 = fields[1].parse().unwrap();
        let wdt: u32 = fields[2].parse().unwrap();
        let coordinates: Vec<f32> = fields[3..].iter().map(|v| v.parse().unwrap()).collect();
        let min = Vec3::from_slice(&coordinates[..3]);
        let max = Vec3::from_slice(&coordinates[3..]);
        let mut baker = los::bake::Baker::for_navigation(&data);
        let mut csv = BufWriter::new(
            std::fs::File::create(output.join(format!("{label}-collision.csv"))).unwrap(),
        );
        writeln!(csv, "slope_degrees,m2_obstacle,x0,y0,z0,x1,y1,z1,x2,y2,z2").unwrap();
        trace_boundary_wmo_sources(&data, wdt, min, max, &mut trace);
        let mut count = 0;
        if let Some(global) = baker.collect_global(wdt).unwrap() {
            for (_, geometry) in global {
                count += export_geometry(&geometry, min, max, &mut csv);
            }
        } else {
            let tiles = baker.map_tiles(wdt).unwrap();
            let a = tile_of(min);
            let b = tile_of(max);
            for row in a.0.min(b.0)..=a.0.max(b.0) {
                for col in a.1.min(b.1)..=a.1.max(b.1) {
                    if let Some((geometry, _)) = baker.collect_tile(&tiles, (row, col)).unwrap() {
                        count += export_geometry(&geometry, min, max, &mut csv);
                    }
                }
            }
        }
        csv.flush().unwrap();
        writeln!(trace, "label={label} map={map} wdt={wdt} min={min:?} max={max:?} collision_triangles={count} geometry_problems={:?}", baker.problems).unwrap();
        trace.flush().unwrap();
        eprintln!(
            "{label}: map={map} collision_triangles={count} problems={}",
            baker.problems.len()
        );
    }
}
