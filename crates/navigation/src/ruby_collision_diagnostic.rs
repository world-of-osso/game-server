//! Raw collision evidence at Ruby's measured component boundary and target feet.
use super::*;
use std::io::{BufWriter, Write};

fn overlaps(triangle: &[Vec3; 3], min: Vec3, max: Vec3) -> bool {
    let lower = triangle[0].min(triangle[1]).min(triangle[2]);
    let upper = triangle[0].max(triangle[1]).max(triangle[2]);
    lower.cmple(max).all() && upper.cmpge(min).all()
}

fn write_collision_window(
    geometry: &los::bake::CollectedGeometry,
    min: Vec3,
    max: Vec3,
    output: &mut impl Write,
) -> usize {
    let mut count = 0;
    for triangle in &geometry.triangles {
        if !overlaps(triangle, min, max) {
            continue;
        }
        for (index, point) in triangle.iter().enumerate() {
            if index > 0 {
                write!(output, ",").unwrap();
            }
            write!(output, "{},{},{}", point.x, point.y, point.z).unwrap();
        }
        writeln!(output).unwrap();
        count += 1;
    }
    count
}

#[test]
#[ignore = "requires local Ruby4289279 geometry and FOLLOWER_GOREK_DIAGNOSTICS"]
fn ruby_raw_collision_at_component_gap_and_target() {
    const WDT: u32 = 4289279;
    let data = PathBuf::from(std::env::var_os("FOLLOWER_COMPONENT_ASSETS").unwrap());
    let output = PathBuf::from(std::env::var_os("FOLLOWER_GOREK_DIAGNOSTICS").unwrap());
    let mut trace =
        BufWriter::new(std::fs::File::create(output.join("ruby-raw-collision.txt")).unwrap());
    let mut baker = los::bake::Baker::for_navigation(&data);
    let global = baker.collect_global(WDT).unwrap();
    let tiles = baker.map_tiles(WDT).unwrap();
    // Bounds include the exact measured pair plus a three-yard XZ margin.
    for (label, min, max) in [
        (
            "nearest_gap",
            Vec3::new(1566.0, 255.0, 381.0),
            Vec3::new(1585.0, 336.0, 408.0),
        ),
        (
            "erkhart_support",
            Vec3::new(1770.0, 309.0, -22.0),
            Vec3::new(1778.0, 317.0, -15.0),
        ),
    ] {
        let mut csv = BufWriter::new(
            std::fs::File::create(output.join(format!("ruby-{label}-collision.csv"))).unwrap(),
        );
        writeln!(csv, "x0,y0,z0,x1,y1,z1,x2,y2,z2").unwrap();
        let mut count = 0;
        if let Some(global) = &global {
            for (_, geometry) in global {
                count += write_collision_window(geometry, min, max, &mut csv);
            }
        } else {
            let start = tile_of(min);
            let end = tile_of(max);
            for row in start.0.min(end.0)..=start.0.max(end.0) {
                for col in start.1.min(end.1)..=start.1.max(end.1) {
                    if let Some((geometry, _)) = baker.collect_tile(&tiles, (row, col)).unwrap() {
                        count += write_collision_window(&geometry, min, max, &mut csv);
                    }
                }
            }
        }
        csv.flush().unwrap();
        writeln!(
            trace,
            "window={label} wdt={WDT} min={min:?} max={max:?} triangles={count} problems={:?}",
            baker.problems
        )
        .unwrap();
        assert!(
            count > 0,
            "raw collision window {label} must contain source geometry"
        );
        assert!(
            baker.problems.is_empty(),
            "source collision must load without collector problems"
        );
    }
    trace.flush().unwrap();
}
