//! Observe wider Halls raw support topology without rasterizing or changing tiles.
use super::*;
use std::io::{BufWriter, Write};

fn write_collision_triangles(
    geometry: &los::bake::CollectedGeometry,
    tile: (u32, u32),
    output: &mut impl Write,
) -> usize {
    let obstacles: HashSet<_> = geometry
        .obstacles
        .iter()
        .map(|triangle| triangle.map(|point| point.to_array().map(f32::to_bits)))
        .collect();
    for triangle in &geometry.triangles {
        let normal = (triangle[1] - triangle[0])
            .cross(triangle[2] - triangle[0])
            .normalize_or_zero();
        let slope = normal.y.abs().clamp(0.0, 1.0).acos().to_degrees();
        let obstacle =
            obstacles.contains(&triangle.map(|point| point.to_array().map(f32::to_bits)));
        write!(output, "{},{},{slope},{obstacle}", tile.0, tile.1).unwrap();
        for point in triangle {
            write!(output, ",{},{},{}", point.x, point.y, point.z).unwrap();
        }
        writeln!(output).unwrap();
    }
    geometry.triangles.len()
}

#[test]
#[ignore = "requires local Halls4290690 assets and FOLLOWER_GOREK_DIAGNOSTICS"]
fn halls_full_map_raw_collision_support_export() {
    const WDT: u32 = 4290690;
    let data = PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_ASSETS").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut trace =
        BufWriter::new(std::fs::File::create(output.join("halls-full-raw.txt")).unwrap());
    let mut csv =
        BufWriter::new(std::fs::File::create(output.join("halls-full-raw-collision.csv")).unwrap());
    writeln!(
        csv,
        "row,col,slope_degrees,m2_obstacle,x0,y0,z0,x1,y1,z1,x2,y2,z2"
    )
    .unwrap();
    let mut baker = los::bake::Baker::for_navigation(&data);
    let mut count = 0;
    if let Some(global) = baker.collect_global(WDT).unwrap() {
        for (tile, geometry) in global {
            let added = write_collision_triangles(&geometry, tile, &mut csv);
            writeln!(trace, "global_tile={tile:?} triangles={added}").unwrap();
            count += added;
        }
    } else {
        let tiles = baker.map_tiles(WDT).unwrap();
        for (index, files) in tiles.iter().enumerate() {
            if files.root == 0 && files.obj0 == 0 {
                continue;
            }
            let row = index as u32 % los::MAP_TILES;
            let col = index as u32 / los::MAP_TILES;
            if let Some((geometry, _)) = baker.collect_tile(&tiles, (row, col)).unwrap() {
                let added = write_collision_triangles(&geometry, (row, col), &mut csv);
                writeln!(
                    trace,
                    "tile=({row},{col}) root={} obj0={} triangles={added}",
                    files.root, files.obj0
                )
                .unwrap();
                count += added;
            }
        }
    }
    csv.flush().unwrap();
    writeln!(
        trace,
        "wdt={WDT} triangles={count} problems={:?}",
        baker.problems
    )
    .unwrap();
    trace.flush().unwrap();
    assert!(count > 0, "Halls source geometry must be present");
    assert!(
        baker.problems.is_empty(),
        "full raw source coverage must be explicit"
    );
    eprintln!(
        "Halls raw collision: triangles={count} collector_problems={}",
        baker.problems.len()
    );
}
