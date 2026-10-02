//! Line of sight against tiles baked from the local CASC extraction (game-engine `data/`):
//! Elwynn Forest and Stormwind, map 0. Positions are WoW coordinates.
#![cfg(feature = "bake")]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use los::bake::{Baker, DEFAULT_THICKNESS_CUTOFF, adt};
use los::{LosMap, LosTile, Vec3, tile_file_name, tile_of};

/// Map 0's `Map.db2` WdtFileDataID (Eastern Kingdoms).
const EASTERN_KINGDOMS_WDT: u32 = 775971;
/// `world/generic/human/passive doodads/stonepyres/stonepyre01.m2`.
const STONEPYRE01: u32 = 198581;

fn client_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
}

fn wow(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, z, -y)
}

/// Bakes `tiles` at `cutoff` into a fresh directory kept for the test run.
fn bake(tiles: &[(u32, u32)], cutoff: f32) -> PathBuf {
    let dir = tempfile::tempdir().unwrap().keep();
    let mut baker = Baker::new(&client_data_dir(), cutoff);
    let map_tiles = baker.map_tiles(EASTERN_KINGDOMS_WDT).unwrap();
    for &tile in tiles {
        let (baked, _) = baker.bake_tile(&map_tiles, tile).unwrap().unwrap();
        std::fs::write(dir.join(tile_file_name(tile)), baked.to_bytes()).unwrap();
    }
    dir
}

/// Stormwind's gate and stairs (rows 30-31) and Northshire (row 32) with a ring around
/// them, at the default cutoff.
fn baked() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let tiles: Vec<_> = (29..=33)
            .flat_map(|row| (47..=49).map(move |col| (row, col)))
            .collect();
        bake(&tiles, DEFAULT_THICKNESS_CUTOFF)
    })
}

fn clear(dir: &Path, a: Vec3, b: Vec3) -> bool {
    LosMap::new(dir).clear(a, b).unwrap()
}

#[test]
fn stormwind_outer_wall_blocks() {
    // From the Elwynn side of the wall west of the main gate to the street behind it, both
    // under open sky; the same line 50 yd up passes over the wall.
    let outside = wow(-8864.3, 469.6, 84.8);
    let inside = wow(-8823.8, 498.6, 101.0);
    assert!(!clear(baked(), outside, inside));
    assert!(!clear(baked(), inside, outside));
    assert!(clear(
        baked(),
        wow(-8864.3, 469.6, 170.0),
        wow(-8823.8, 498.6, 170.0)
    ));
}

#[test]
fn northshire_abbey_open_doorway_is_clear_and_its_wall_blocks() {
    // From the steps in front of the west door into the hall, then the same line 4 yd to
    // the side through the wall beside the door.
    assert!(clear(
        baked(),
        wow(-8909.2, -146.6, 83.0),
        wow(-8896.5, -173.8, 83.0)
    ));
    assert!(!clear(
        baked(),
        wow(-8905.575, -144.91, 83.0),
        wow(-8892.875, -172.11, 83.0)
    ));
}

#[test]
fn elwynn_fence_is_dropped_as_thin() {
    // Through `elwynnwoodfence01.m2` (unique id 10321) 0.7 yd above its base: the fence's
    // opaque triangles block the ray when no doodad is dropped.
    let a = wow(-8937.69, -107.99, 83.8);
    let b = wow(-8927.69, -107.99, 83.8);
    assert!(clear(baked(), a, b));
    let uncut = bake(&[(32, 48)], 0.0);
    assert!(!clear(&uncut, a, b));
}

#[test]
fn northshire_stonepyre_pillar_is_kept_and_blocks() {
    // Stonepyre01 (unique id 10325) beside Milly Osworth: thicker than the cutoff, so a
    // ray through its centre is blocked while parallel rays 3 yd to either side are clear.
    let mut baker = Baker::new(&client_data_dir(), DEFAULT_THICKNESS_CUTOFF);
    let thickness = baker.m2(STONEPYRE01).unwrap().thickness(1.0);
    assert!(
        thickness >= DEFAULT_THICKNESS_CUTOFF,
        "Stonepyre01 is {thickness} yd thick"
    );
    let through = |x: f32| (wow(x, -145.0, 84.0), wow(x, -134.0, 84.0));
    let (a, b) = through(-8922.68);
    assert!(!clear(baked(), a, b));
    for x in [-8925.68, -8919.68] {
        let (a, b) = through(x);
        assert!(clear(baked(), a, b), "ray at WoW x {x}");
    }
}

#[test]
fn stormwind_stairs_block_below_the_steps_not_above() {
    // `sw_staircase.wmo` (unique id 15524906) rising from WoW y 990 (z 79.5) to 972
    // (z 85.3): eyes 2 yd above the bottom and the top landing see each other; a level
    // ray at the bottom's chest height runs into the steps.
    assert!(clear(
        baked(),
        wow(-8737.0, 993.0, 81.5),
        wow(-8737.0, 971.0, 87.3)
    ));
    assert!(!clear(
        baked(),
        wow(-8737.0, 995.0, 82.0),
        wow(-8737.0, 970.0, 82.0)
    ));
}

#[test]
fn terrain_ridge_north_of_northshire_blocks() {
    // From the abbey courtyard north over the ridge (up to z 300) to the valley beyond it.
    let a = wow(-8923.9, -135.9, 83.0);
    let b = wow(-8363.9, -135.9, 161.0);
    assert!(!clear(baked(), a, b));
    // The terrain alone blocks it.
    let baker = Baker::new(&client_data_dir(), DEFAULT_THICKNESS_CUTOFF);
    let map_tiles = baker.map_tiles(EASTERN_KINGDOMS_WDT).unwrap();
    let blocked_by_terrain = [(32, 47), (32, 48)].into_iter().any(|(row, col)| {
        let files = map_tiles[(col * 64 + row) as usize];
        let root =
            std::fs::read(client_data_dir().join(format!("terrain/{}.adt", files.root))).unwrap();
        let triangles = adt::terrain_triangles(&root).unwrap();
        let vertices: Vec<Vec3> = triangles.iter().flatten().copied().collect();
        let indices = (0..triangles.len() as u32)
            .map(|tri| [tri * 3, tri * 3 + 1, tri * 3 + 2])
            .collect();
        LosTile::build(row, col, vertices, indices).blocks(a, b)
    });
    assert!(blocked_by_terrain);
    assert_eq!(tile_of(b.x, b.z), (32, 47));
}
