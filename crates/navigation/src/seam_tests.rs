//! Actual map34 seam regression and read-only native portal evidence.
use super::*;

fn seam_floor(boundary: f32, rotate: bool) -> Vec<[Vec3; 3]> {
    let point = |x, z| {
        if rotate {
            Vec3::new(z, 0.0, x)
        } else {
            Vec3::new(x, 0.0, z)
        }
    };
    let a = point(boundary - 100.0, -10.0);
    let b = point(boundary - 100.0, 10.0);
    let c = point(boundary + 12.0, 10.0);
    let d = point(boundary + 12.0, -10.0);
    vec![[a, b, c], [a, c, d]]
}

#[test]
fn generated_open_floor_routes_stay_short_near_subtile_and_wow_tile_corners() {
    let mut failures = Vec::new();
    for boundary in [
        SUBTILE_SIZE,
        SUBTILE_SIZE * 2.0,
        SUBTILE_SIZE * 3.0,
        TILE_SIZE,
    ] {
        for rotate in [false, true] {
            let triangles = seam_floor(boundary, rotate);
            let a = tile_of(triangles[0][0]);
            let b = tile_of(triangles[0][2]);
            let dir = tempfile::tempdir().unwrap();
            for row in a.0.min(b.0)..=a.0.max(b.0) {
                for col in a.1.min(b.1)..=a.1.max(b.1) {
                    let bytes = crate::bake::bake_tile(&triangles, (row, col)).unwrap();
                    std::fs::write(dir.path().join(format!("{row}_{col}.nav")), bytes).unwrap();
                }
            }
            let point = |side| {
                if rotate {
                    Vec3::new(side, 0.0, boundary - 13.0)
                } else {
                    Vec3::new(boundary - 13.0, 0.0, side)
                }
            };
            let mut map = NavigationMap::open(dir.path()).unwrap();
            for direction in [1.0, -1.0] {
                let points = map.path(point(direction), point(-direction)).unwrap();
                let length: f32 = points.windows(2).map(|p| p[0].distance(p[1])).sum();
                eprintln!(
                    "generated seam boundary{boundary} rotate{rotate} direction{direction} length{length}"
                );
                if length > 2.1 {
                    failures.push((boundary, rotate, direction, length));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "open floor corner detours: {failures:?}"
    );
}

#[test]
#[ignore = "requires GAME_SERVER_GROUND_DIR with real Stockade navigation and LOS bakes"]
fn stockade_two_yard_seam_routes_are_short_in_both_directions() {
    let root = PathBuf::from(
        std::env::var_os("GAME_SERVER_GROUND_DIR").expect("explicit real ground assets"),
    );
    let mut map = NavigationMap::open(&root.join("navmesh/34")).unwrap();
    let mut los = los::LosMap::new(&root.join("los/34"));
    let mut failures = Vec::new();
    for x in [115.0, 120.0, 130.0, 135.0] {
        for direction in [1.0, -1.0] {
            let start = Vec3::new(x, -25.6062, direction);
            let end = Vec3::new(x, -25.6062, -direction);
            assert!(
                los.clear(start + Vec3::Y, end + Vec3::Y).unwrap(),
                "actual collision must establish a traversable seam case {start:?}->{end:?}"
            );
            let surface_end = map.first_collision(start, end).unwrap();
            eprintln!("mesh-surface ray {start:?}->{end:?} reaches {surface_end:?}");
            assert!(
                (surface_end.x - end.x).hypot(surface_end.z - end.z) < 0.01,
                "the native mesh itself must permit the direct crossing"
            );
            let corridor = map.query_corridor(start, end).unwrap();
            let points = map
                .pull_corners(&corridor.start, &corridor.end, &corridor.polygons)
                .unwrap();
            let length: f32 = points.windows(2).map(|p| p[0].distance(p[1])).sum();
            eprintln!("seam {start:?}->{end:?} length{length} points{points:?}");
            if length > 2.1 {
                failures.push((start, end, length));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "unobstructed2yd seam routes detoured: {failures:?}"
    );
}
