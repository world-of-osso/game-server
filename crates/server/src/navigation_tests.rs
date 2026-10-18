use super::*;
use crate::ground::{GroundMap, GroundProbe};

fn ground_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground")
}
fn engine_data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
}
fn wow(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, z, -y)
}
fn ground() -> GroundMap {
    GroundMap::new(&engine_data(), HashMap::from([(0, 775971)]))
}

fn clear_path(path: &[Vec3]) {
    let mut los = los::LosMap::new(&engine_data().join("los/0"));
    for pair in path.windows(2) {
        assert!(
            los.clear(pair[0] + Vec3::Y, pair[1] + Vec3::Y).unwrap(),
            "navigation crossed real Stormwind geometry: {pair:?}"
        );
    }
}

#[test]
#[ignore = "requires nav-bake map0 and real Stormwind assets"]
fn navigation_stormwind_auction_house_uses_wmo_floor() {
    let mut map = navigation::NavigationMap::open(&ground_dir().join("navmesh/0")).unwrap();
    let path = map
        .path(wow(-8820.0, 660.0, 98.1), wow(-8816.0, 661.0, 98.1))
        .unwrap();
    clear_path(&path);
    let mut ground = ground();
    for p in path {
        let GroundProbe::Supported(floor) = ground.ground_at(0, p) else {
            panic!("unsupported route {p:?}");
        };
        assert!(
            (floor - 98.06).abs() < 0.1,
            "WMO floor, not terrain94.65: {floor}"
        );
        assert!(
            (p.y - floor).abs() < 0.4,
            "navigation clearance origin {p:?} vs floor {floor}"
        );
    }
}

#[test]
#[ignore = "requires nav-bake map0 and real Stormwind assets"]
fn navigation_stormwind_stairs_reach_upper_landing() {
    let mut map = navigation::NavigationMap::open(&ground_dir().join("navmesh/0")).unwrap();
    let path = map
        .path(wow(-8737.0, 993.0, 79.5), wow(-8737.0, 971.0, 85.3))
        .unwrap();
    clear_path(&path);
    assert!(
        path.last().unwrap().y - path[0].y > 4.0,
        "stairs must rise to landing: {path:?}"
    );
    // Recast spans/contours approximate the support surface. The server's existing
    // ground correction must put the moving creature on the actual collision floor.
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(50),
        ))
        .insert_resource(crate::world_data::WorldData::default())
        .insert_resource(ground())
        .insert_resource(Navigation::new(&ground_dir().join("navmesh")))
        .add_plugins(crate::creature_ai::CreatureAiPlugin);
    app.update();
    let from = wow(-8737.0, 993.0, 79.5);
    let goal = wow(-8737.0, 971.0, 85.3);
    let target = app
        .world_mut()
        .spawn((position(goal), crate::maps::InMap::continent(0)))
        .id();
    let mob = app
        .world_mut()
        .spawn((
            position(from),
            crate::creature_ai::SpawnPoint {
                x: from.x,
                y: from.y,
                z: from.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            crate::creature_ai::CreatureState::Pursuing {
                target: target.to_bits(),
            },
            crate::maps::InMap::continent(0),
        ))
        .id();
    let mut ground = ground();
    let mut los = los::LosMap::new(&engine_data().join("los/0"));
    let mut last = from;
    for tick in 0..160 {
        app.update();
        let p = point(*app.world().get::<Position>(mob).unwrap());
        let GroundProbe::Supported(floor) = ground.ground_at(0, p) else {
            panic!("unsupported stair movement {p:?}");
        };
        assert!(
            (p.y - floor).abs() < 0.05,
            "tick{tick} actual feet {p:?} vs floor{floor}"
        );
        assert!(
            los.clear(last + Vec3::Y, p + Vec3::Y).unwrap(),
            "tick{tick} crossed stairs {last:?}->{p:?}"
        );
        last = p;
    }
    assert!(
        last.y - from.y > 4.0,
        "creature reached upper landing: {last:?}"
    );
    assert!(
        (last.distance(goal) - 2.0).abs() < 0.05,
        "stops in reach on the landing: {last:?}"
    );
}

#[test]
#[ignore = "requires nav-bake map0 and real Elwynn assets"]
fn navigation_real_terrain_connects_across_wow_tile_seam() {
    let mut ground = ground();
    let mut a = wow(-8970.0, -2.0, 400.0);
    let mut b = wow(-8970.0, 2.0, 400.0);
    for p in [&mut a, &mut b] {
        let GroundProbe::Supported(y) = ground.ground_at(0, *p) else {
            panic!("missing terrain at {p:?}");
        };
        p.y = y;
    }
    assert_ne!(navigation::tile_of(a), navigation::tile_of(b));
    let mut map = navigation::NavigationMap::open(&ground_dir().join("navmesh/0")).unwrap();
    let path = map
        .path(a, b)
        .expect("connected portals across real WoW tile seam");
    clear_path(&path);
    assert!(path[0].distance(a) < 0.5);
    assert!(path.last().unwrap().distance(b) < 0.5);
}

fn request(map_id: u32, from: Vec3, to: Vec3) -> MoveRequest {
    MoveRequest {
        map_id,
        goal: Goal::Chase(99),
        from: position(from),
        to: position(to),
        distance: 0.5,
        reach: 2.0,
    }
}

#[test]
fn navigation_unbaked_map_is_explicit_legacy_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let mut nav = Navigation::new(dir.path());
    assert!(matches!(
        nav.advance(
            Entity::from_bits(1),
            request(34, wow(115.6, 46.6, -34.8), wow(128.5, 52.0, -33.86))
        ),
        Step::Unbaked
    ));
}

#[test]
fn navigation_empty_baked_tile_is_blocked_not_legacy() {
    let dir = tempfile::tempdir().unwrap();
    let map_dir = dir.path().join("34");
    std::fs::create_dir(&map_dir).unwrap();
    std::fs::write(map_dir.join("31_31.nav"), []).unwrap();
    let mut nav = Navigation::new(dir.path());
    assert!(matches!(
        nav.advance(
            Entity::from_bits(1),
            request(34, wow(115.6, 46.6, -34.8), wow(128.5, 52.0, -33.86))
        ),
        Step::Blocked
    ));
}

#[test]
fn navigation_missing_tile_on_baked_map_is_blocked_not_legacy() {
    let dir = tempfile::tempdir().unwrap();
    let map_dir = dir.path().join("34");
    std::fs::create_dir(&map_dir).unwrap();
    std::fs::write(map_dir.join("31_31.nav"), []).unwrap();
    let mut nav = Navigation::new(dir.path());
    assert!(matches!(
        nav.advance(
            Entity::from_bits(1),
            request(34, wow(115.6, 46.6, -34.8), wow(115.6, -600.0, -33.86))
        ),
        Step::Blocked
    ));
}

#[test]
fn navigation_corrupt_tile_is_blocked_not_legacy() {
    let dir = tempfile::tempdir().unwrap();
    let map_dir = dir.path().join("34");
    std::fs::create_dir(&map_dir).unwrap();
    std::fs::write(map_dir.join("31_31.nav"), b"truncated navigation").unwrap();
    let mut nav = Navigation::new(dir.path());
    assert!(matches!(
        nav.advance(
            Entity::from_bits(1),
            request(34, wow(115.6, 46.6, -34.8), wow(128.5, 52.0, -33.86))
        ),
        Step::Blocked
    ));
}
