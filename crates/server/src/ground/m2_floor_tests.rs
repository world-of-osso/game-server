//! Real authored M2 hulls at their actual ADT placements, through GroundMap.
use super::*;
use los::bake::{adt, m2::M2File};

fn data_dir() -> PathBuf {
    std::env::var_os("GAME_SERVER_GROUND_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
        })
}

fn real_floor_map(
    map: u32,
    wdt: u32,
    obj: u32,
    model: u32,
    unique: u32,
) -> (tempfile::TempDir, GroundMap) {
    let data = data_dir();
    let dir = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(data.join("terrain"), dir.path().join("terrain")).unwrap();
    std::os::unix::fs::symlink(data.join("models"), dir.path().join("models")).unwrap();
    let bytes = std::fs::read(data.join(format!("models/{model}.m2"))).unwrap();
    let hull = M2File::parse(&bytes)
        .unwrap()
        .collision_triangles()
        .unwrap();
    assert!(!hull.is_empty(), "real model has no collision hull");
    let obj = std::fs::read(data.join(format!("terrain/{obj}.adt"))).unwrap();
    let (placements, _) = adt::placements(&obj).unwrap();
    let placement = placements.iter().find(|p| p.unique_id == unique).unwrap();
    assert_eq!(placement.model, adt::ModelRef::Fdid(model));
    let world = placement.world_from_local();
    let vertices: Vec<_> = hull
        .into_iter()
        .flatten()
        .map(|v| world.transform_point3(v))
        .collect();
    let triangles = (0..vertices.len() as u32 / 3)
        .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
        .collect();
    let tile = los::tile_of(world.translation.x, world.translation.z);
    let baked = los::LosTile::build(tile.0, tile.1, vertices, triangles);
    let out = dir.path().join(format!("los/{map}/m2"));
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join(los::tile_file_name(tile)), baked.to_bytes()).unwrap();
    let ground = GroundMap::new(dir.path(), HashMap::from([(map, wdt)]));
    (dir, ground)
}

#[test]
fn m2_floor_eots_real_alliance_start_is_supported() {
    let (_dir, mut ground) = real_floor_map(566, 788893, 788995, 193193, 995848);
    let feet = Vec3::new(2523.76, 1247.7, -1596.13);
    match ground.ground_at(566, feet) {
        GroundProbe::Supported(height) => assert!(
            (height - 1247.7).abs() < 1.0,
            "EotS M2 floor is {height}, not terrain 1174"
        ),
        other => panic!("EotS start unsupported: {other:?}"),
    }
}

#[test]
fn m2_floor_eots_real_horde_start_is_supported() {
    let (_dir, mut ground) = real_floor_map(566, 788893, 789000, 193193, 995856);
    let feet = Vec3::new(1803.02, 1249.19, -1539.97);
    match ground.ground_at(566, feet) {
        GroundProbe::Supported(height) => {
            assert!((height - 1249.19).abs() < 1.0, "Horde start: {height}")
        }
        other => panic!("Horde start unsupported: {other:?}"),
    }
}

#[test]
fn m2_floor_northshire_real_stonepyre_top_is_supported() {
    let (_dir, mut ground) = real_floor_map(0, 775971, 778028, 198581, 10325);
    let feet = Vec3::new(-8922.681, 100.0, 139.566);
    match ground.ground_at(0, feet) {
        GroundProbe::Supported(height) => assert!(
            height > 85.0,
            "M2 top is {height}; terrain must not replace it"
        ),
        other => panic!("stonepyre top unsupported: {other:?}"),
    }
}
