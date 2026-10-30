//! Real authored M2 hulls at their actual ADT placements, through GroundMap.
use super::*;
use los::bake::{adt, m2::M2File};

#[test]
fn m2_solid_companion_supports_steep_face_above_walkable_floor() {
    let dir = tempfile::tempdir().unwrap();
    let position = Vec3::new(0.0, 13.0, 0.0);
    let tile = los::tile_of(position.x, position.z);
    let floor = los::LosTile::build(
        tile.0,
        tile.1,
        vec![
            Vec3::new(-4.0, 12.0, -4.0),
            Vec3::new(4.0, 12.0, -4.0),
            Vec3::new(0.0, 12.0, 4.0),
        ],
        vec![[0, 1, 2]],
    );
    let solid = los::LosTile::build(
        tile.0,
        tile.1,
        vec![
            Vec3::new(-4.0, 8.88, -4.0),
            Vec3::new(4.0, 19.12, -4.0),
            Vec3::new(0.0, 14.0, 4.0),
        ],
        vec![[0, 1, 2]],
    );
    for (directory, baked) in [("m2", floor), ("m2-solid", solid)] {
        let output = dir.path().join("los/2519").join(directory);
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join(los::tile_file_name(tile)), baked.to_bytes()).unwrap();
    }
    let mut ground = GroundMap::new(dir.path(), HashMap::new());
    assert_eq!(
        ground.ground_at(2519, position),
        GroundProbe::Supported(12.0)
    );
    assert_eq!(
        ground.player_solid_ground_in(crate::maps::InMap::continent(2519), position),
        GroundProbe::Supported(14.0)
    );
    assert_eq!(
        ground.player_solid_ground_in(crate::maps::InMap::continent(2519), position.with_y(12.0)),
        GroundProbe::Supported(12.0)
    );
}

#[test]
fn missing_m2_solid_companion_keeps_solid_ground_unloaded() {
    let dir = tempfile::tempdir().unwrap();
    let floor = dir.path().join("los/2519/m2");
    std::fs::create_dir_all(&floor).unwrap();
    std::fs::write(floor.join("32_32.los"), []).unwrap();
    let mut ground = GroundMap::new(dir.path(), HashMap::new());
    assert_eq!(
        ground.player_solid_ground_in(crate::maps::InMap::continent(2519), Vec3::ZERO),
        GroundProbe::Unloaded
    );
}

#[test]
#[ignore = "requires a private Neltharus solid M2 bake in M2_REAL_GROUND"]
fn neltharus_real_m2_solid_companion_supports_player() {
    let data = PathBuf::from(std::env::var_os("M2_REAL_GROUND").unwrap());
    let mut ground = GroundMap::new(&data, HashMap::from([(2519, 4289128)]));
    let map = crate::maps::InMap::continent(2519);
    let candidates = [
        Vec3::new(-145.7191, 46.6855, 834.2512),
        Vec3::new(-41.106, 50.2199, 737.4225),
        Vec3::new(-46.971, 50.5717, 744.8711),
        Vec3::new(-46.2131, 50.7774, 746.6439),
        Vec3::new(-59.3248, 46.3166, 828.0685),
        Vec3::new(-73.4981, 39.8711, 826.7625),
        Vec3::new(-73.7469, 46.4709, 763.7499),
        Vec3::new(-106.7553, 40.0974, 709.3105),
    ];
    let mut witnessed = None;
    for feet in candidates {
        let walkable = ground.ground_in(map, feet);
        let solid = ground.player_solid_ground_in(map, feet);
        eprintln!("real M2 candidate {feet:?}: walkable={walkable:?}, solid={solid:?}");
        if let GroundProbe::Supported(height) = solid {
            let lower =
                !matches!(walkable, GroundProbe::Supported(other) if other >= height - 0.01);
            if lower && (height - feet.y).abs() < 0.02 {
                witnessed = Some((feet, height));
                break;
            }
        }
    }
    assert!(
        witnessed.is_some(),
        "no real steep M2 face selected by player ground"
    );
}

#[test]
#[ignore = "requires canonical Neltharus ground assets"]
fn neltharus_follower_keeps_upper_floor_at_recorded_drop() {
    let data = data_dir();
    let mut ground = GroundMap::new(&data, HashMap::from([(2519, 4289128)]));
    let map = crate::maps::InMap::continent(2519);
    let from = shared::components::Position {
        x: -122.77629,
        y: 122.80975,
        z: 580.558,
    };
    let to = shared::components::Position {
        x: -122.85992,
        y: 123.0,
        z: 580.92346,
    };
    let height =
        crate::creature_ai::follower_footprint_ground(Some(&mut ground), Some(&map), from, to)
            .expect("supported route");
    assert!(
        (height - 122.875114).abs() < 0.05,
        "follower dropped to {height}"
    );
}

#[test]
#[ignore = "requires canonical Neltharus ground assets"]
fn neltharus_stationary_follower_keeps_floor_at_sargha_seam() {
    let mut ground = GroundMap::new(&data_dir(), HashMap::from([(2519, 4289128)]));
    let map = crate::maps::InMap::continent(2519);
    let from = shared::components::Position {
        x: -89.583984,
        y: 72.513824,
        z: 828.08673,
    };
    let to = shared::components::Position {
        x: from.x,
        y: 72.46268,
        z: from.z,
    };
    let height =
        crate::creature_ai::follower_footprint_ground(Some(&mut ground), Some(&map), from, to)
            .expect("stationary route");
    assert!(
        (height - from.y).abs() < 0.01,
        "stationary follower fell to {height}"
    );
}

#[test]
fn follower_footprint_does_not_bridge_a_wide_drop() {
    let mut ground = GroundMap::flat_arena(94.0);
    let map = crate::maps::InMap::continent(2519);
    let from = shared::components::Position {
        x: 0.0,
        y: 123.0,
        z: 0.0,
    };
    let to = shared::components::Position {
        x: 0.0,
        y: 123.0,
        z: 1.0,
    };
    let height =
        crate::creature_ai::follower_footprint_ground(Some(&mut ground), Some(&map), from, to);
    assert_eq!(height, None);
}

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
