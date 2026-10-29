#![cfg(feature = "bake")]
//! Authored hull bakes, not render-face or navigation classification tests.
use los::bake::Baker;
use los::{LosTile, Vec3};
use std::path::PathBuf;

fn data() -> PathBuf {
    std::env::var_os("GAME_SERVER_GROUND_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
        })
}

#[test]
fn m2_floor_baker_real_eots_and_northshire_heights() {
    for (wdt, tile, feet, height) in [
        (
            788893,
            (29, 27),
            Vec3::new(2523.76, 1247.7, -1596.13),
            1247.7,
        ),
        (
            788893,
            (29, 28),
            Vec3::new(1803.02, 1249.19, -1539.97),
            1249.19,
        ),
        (
            775971,
            (32, 48),
            Vec3::new(-8922.681, 87.57, 139.566),
            87.57,
        ),
    ] {
        let mut baker = Baker::for_m2_floors(&data());
        let tiles = baker.map_tiles(wdt).unwrap();
        let (baked, _) = baker.bake_tile(&tiles, tile).unwrap().unwrap();
        let loaded = LosTile::from_bytes(&baked.to_bytes()).unwrap();
        let actual = loaded
            .floor_height(feet)
            .expect("real floor in collision bake");
        assert!((actual - height).abs() < 1.0, "{feet:?}: {actual}");
    }
}

fn chunk(tag: &[u8; 4], bytes: &[u8]) -> Vec<u8> {
    let mut out = tag.iter().rev().copied().collect::<Vec<_>>();
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    out
}

fn global_wmo_fixture() -> tempfile::TempDir {
    // Synthetic WMO container, real198581 hull. No WMO group geometry: only its
    // active doodad set collides. Its translated/scaled/rotated floor must still bake.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("terrain")).unwrap();
    std::fs::create_dir(dir.path().join("models")).unwrap();
    std::os::unix::fs::symlink(
        data().join("models/198581.m2"),
        dir.path().join("models/198581.m2"),
    )
    .unwrap();
    let mut mphd = vec![0; 32];
    mphd[..4].copy_from_slice(&1u32.to_le_bytes());
    let mut modf = vec![0; 64];
    modf[..4].copy_from_slice(&999u32.to_le_bytes());
    modf[56..58].copy_from_slice(&8u16.to_le_bytes());
    modf[12..16].copy_from_slice(&100f32.to_le_bytes());
    modf[24..28].copy_from_slice(&180f32.to_le_bytes());
    let mut wdt = chunk(b"MPHD", &mphd);
    wdt.extend(chunk(b"MODF", &modf));
    std::fs::write(dir.path().join("terrain/999.wdt"), wdt).unwrap();
    let mut set = vec![0; 32];
    set[24..28].copy_from_slice(&1u32.to_le_bytes());
    let mut doodad = vec![0; 40];
    for (offset, value) in [
        (4, 1200f32),
        (8, 1300.0),
        (12, 10.0),
        (24, std::f32::consts::FRAC_1_SQRT_2),
        (28, std::f32::consts::FRAC_1_SQRT_2),
        (32, 2.0),
    ] {
        doodad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let mut root = chunk(b"MOHD", &[0; 64]);
    root.extend(chunk(b"MODI", &198581u32.to_le_bytes()));
    root.extend(chunk(b"MODS", &set));
    root.extend(chunk(b"MODD", &doodad));
    std::fs::write(dir.path().join("models/999.wmo"), root).unwrap();
    dir
}

fn global_floor(dir: &std::path::Path, feet: Vec3) -> Option<f32> {
    let mut baker = Baker::for_m2_floors(dir);
    let global = baker.collect_global(999).unwrap().unwrap();
    global
        .into_iter()
        .filter_map(|(tile, geometry)| {
            let (vertices, indices) = los::bake::index_triangles(&geometry.triangles);
            LosTile::build(tile.0, tile.1, vertices, indices).floor_height(feet)
        })
        .max_by(f32::total_cmp)
}

#[test]
fn m2_floor_global_wmo_doodad_outside_group_bounds_supports_feet() {
    let dir = global_wmo_fixture();
    let feet = Vec3::new(1200.0, 123.54, -1300.0);
    let height =
        global_floor(dir.path(), feet).expect("active WMO doodad floor outside group bounds");
    assert!((height - 123.45688).abs() < 0.01, "{height}");
}

#[test]
fn m2_floor_authored_downward_normal_is_not_a_floor() {
    let dir = global_wmo_fixture();
    let path = dir.path().join("models/198581.m2");
    std::fs::remove_file(&path).unwrap();
    // Two collision planes: underside at0 with downward winding/normal, roof at4.
    // The same WMO placement puts them at110/118, beyond each other's step reach.
    let mut bytes = vec![0; 384];
    bytes[..4].copy_from_slice(b"MD20");
    for (offset, value) in [
        (0xD8, 6u32),
        (0xDC, 240),
        (0xE0, 6),
        (0xE4, 256),
        (0xE8, 2),
        (0xEC, 336),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    for (index, value) in [0u16, 2, 1, 3, 4, 5].into_iter().enumerate() {
        bytes[240 + index * 2..242 + index * 2].copy_from_slice(&value.to_le_bytes());
    }
    for (index, vertex) in [
        [-4f32, -4.0, 0.0],
        [4.0, -4.0, 0.0],
        [0.0, 4.0, 0.0],
        [-4.0, -4.0, 4.0],
        [4.0, -4.0, 4.0],
        [0.0, 4.0, 4.0],
    ]
    .into_iter()
    .enumerate()
    {
        for (axis, value) in vertex.into_iter().enumerate() {
            let offset = 256 + index * 12 + axis * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
    bytes[344..348].copy_from_slice(&(-1f32).to_le_bytes());
    bytes[356..360].copy_from_slice(&1f32.to_le_bytes());
    std::fs::write(path, bytes).unwrap();
    assert_eq!(
        global_floor(dir.path(), Vec3::new(1200.0, 110.0, -1300.0)),
        None
    );
    let roof = global_floor(dir.path(), Vec3::new(1200.0, 118.0, -1300.0)).unwrap();
    assert!((roof - 118.0).abs() < 0.001, "{roof}");
    let mut baker = Baker::for_m2_solids(dir.path());
    let global = baker.collect_global(999).unwrap().unwrap();
    let start = Vec3::new(1200.0, 111.0, -1300.0);
    let end = Vec3::new(1200.0, 109.0, -1300.0);
    let hit = global.into_iter().find_map(|(tile, geometry)| {
        let (vertices, indices) = los::bake::index_triangles(&geometry.triangles);
        LosTile::build(tile.0, tile.1, vertices, indices).first_hit(start, end)
    });
    assert!(hit.is_some_and(|fraction| (fraction - 0.5).abs() < 0.001));
}
