use super::*;
use crate::ground::{GroundMap, GroundProbe};
use std::collections::HashMap;

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
        .complete_path(wow(-8820.0, 660.0, 98.1), wow(-8816.0, 661.0, 98.1))
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
        .complete_path(wow(-8737.0, 993.0, 79.5), wow(-8737.0, 971.0, 85.3))
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
        .add_plugins(crate::creature_ai::CreatureAiPlugin);
    install_fixture(&mut app, Navigation::new(&ground_dir().join("navmesh")));
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
        .complete_path(a, b)
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

fn ready_step(nav: &mut Navigation, entity: Entity, request: MoveRequest) -> Step {
    let _ = nav.advance(entity, request);
    nav.drain_until_idle();
    nav.advance(entity, request)
}

#[test]
#[ignore = "requires canonical2519 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_gorek_partial_corridors_walk_and_repath_to_target() {
    assert_canonical_route_arrives(
        2519,
        Vec3::new(70.6448, 72.6027, 779.064),
        Vec3::new(-264.45807, 125.19722, 503.5),
    );
}

#[test]
#[ignore = "requires canonical2520 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_brackenhide_partial_corridors_walk_and_repath_three_bosses() {
    // Exact authored XY/source-floor Z from brackenhide-packet-2/receipt.json.
    let entrance = Vec3::new(-3300.65, 0.129981, -4681.51);
    for (entry, target) in [
        (186116, Vec3::new(-3168.1669, 100.49736, -3960.1665)),
        (186120, Vec3::new(-3650.2505, 99.49241, -3961.558)),
        (186121, Vec3::new(-3344.54, 111.17662, -3663.96)),
    ] {
        eprintln!("Brackenhide source placement entry={entry}");
        assert_canonical_route_arrives(2520, entrance, target);
    }
}

#[test]
#[ignore = "requires canonical2527 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_halls_partial_corridors_walk_and_repath_two_bosses() {
    // Exact halls-packet-3 WoW coordinates, converted with the existing axis adapter.
    let entrance = wow(-0.370431, 228.269, 20.8162);
    for (entry, target) in [
        (189722, wow(-353.12274, -155.7234, -67.9066)),
        (189727, wow(-315.97385, 161.89757, -100.404625)),
    ] {
        eprintln!("Halls source placement entry={entry}");
        assert_canonical_route_arrives(2527, entrance, target);
    }
}

#[test]
#[ignore = "requires canonical2526 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_vexamus_partial_corridor_walks_and_repaths() {
    // Exact vexamus-packet-3 inputs; no placement or search-bound changes.
    assert_canonical_route_arrives(
        2526,
        wow(1410.22, -2771.71, 955.637),
        wow(1815.2512, -2783.9756, 904.01917),
    );
}

#[test]
#[ignore = "requires canonical2516 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_nokhud_partial_corridors_walk_and_repath_three_bosses() {
    let entrance = wow(-570.405, 2189.37, 431.296);
    for (entry, target) in [
        (186151, wow(-536.05316, 4163.211, 381.20233)),
        (186338, wow(-2095.4968, 4335.8853, 536.04694)),
        (186616, wow(-1167.5642, 1356.6361, 390.72348)),
    ] {
        eprintln!("Nokhud source placement entry={entry}");
        assert_canonical_route_arrives(2516, entrance, target);
    }
}

#[test]
#[ignore = "requires canonical2521 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_ruby_partial_corridor_walks_and_repaths() {
    assert_canonical_route_arrives(
        2521,
        wow(1386.2, -169.719, 131.0),
        wow(1773.8756, 18.689396, 313.123),
    );
}

#[test]
#[ignore = "requires canonical2651 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_darkflame_208743_partial_corridor_walks_and_repaths() {
    assert_canonical_route_arrives(
        2651,
        wow(239.839, 75.8715, 94.7386),
        wow(406.7775, -155.27142, 56.68355),
    );
}

#[test]
#[ignore = "requires canonical2651 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_darkflame_208747_partial_corridor_walks_and_repaths() {
    assert_canonical_route_arrives(
        2651,
        wow(239.839, 75.8715, 94.7386),
        wow(188.47888, -547.81714, 59.8),
    );
}

#[test]
#[ignore = "requires canonical2661 assets and FOLLOWER_NAV_CANONICAL"]
fn navigation_cinderbrew_goldie_partial_corridor_walks_and_repaths() {
    assert_canonical_route_arrives(
        2661,
        wow(2650.88, -4835.88, 101.042),
        wow(2649.0535, -5013.1484, 110.600006),
    );
}

fn assert_canonical_route_arrives(map_id: u32, mut from: Vec3, target: Vec3) {
    let root = PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").unwrap());
    // Arrival is at the existing native projection, not an invented tighter XY bound.
    let mut map = navigation::NavigationMap::open(&root.join(map_id.to_string())).unwrap();
    let target_floor = map.complete_path(target, target).unwrap();
    let projected_target = *target_floor.last().unwrap();
    eprintln!("map{map_id} authored_target={target:?} native_projection={projected_target:?}");
    let mut nav = Navigation::new(&root);
    let actor = Entity::from_bits(99);
    let mut arrived = false;
    for _ in 0..10_000 {
        let mut movement = request(map_id, from, target);
        movement.distance = 1.0;
        movement.reach = 0.0;
        let step = nav.advance(actor, movement);
        nav.drain_until_idle();
        match step {
            Step::Moved { position, reached } => {
                let next = point(position);
                assert!(next.distance(from) < 3.0, "no segment-end teleport");
                from = next;
                if reached {
                    assert!(
                        from.distance(projected_target) < 0.001,
                        "partial end is not arrival: feet={from:?} projected_target={projected_target:?} authored_target={target:?}"
                    );
                    arrived = true;
                    break;
                }
            }
            Step::Pending => {}
            Step::Blocked => {
                panic!("map{map_id} mover blocked before reaching {target:?} at {from:?}")
            }
            Step::Unbaked => panic!("canonical map{map_id} bake missing"),
        }
    }
    assert!(
        arrived,
        "map{map_id} mover failed to arrive in bounded walk: {from:?}"
    );
}

#[test]
fn navigation_completed_disconnected_search_stays_blocked_without_moving() {
    let dir = tempfile::tempdir().unwrap();
    let map_dir = dir.path().join("12");
    std::fs::create_dir(&map_dir).unwrap();
    let mut triangles = Vec::new();
    for x in [10.0, 70.0] {
        let a = Vec3::new(x, 0.0, 10.0);
        let b = Vec3::new(x, 0.0, 30.0);
        let c = Vec3::new(x + 30.0, 0.0, 30.0);
        let d = Vec3::new(x + 30.0, 0.0, 10.0);
        triangles.extend([[a, b, c], [a, c, d]]);
    }
    let tile = navigation::bake::bake_tile(&triangles, (32, 31)).unwrap();
    std::fs::write(map_dir.join("32_31.nav"), tile).unwrap();
    let mut nav = Navigation::new(dir.path());
    let actor = Entity::from_bits(99);
    let from = Vec3::new(20.0, 0.2, 20.0);
    let target = Vec3::new(80.0, 0.2, 20.0);
    // Previous partial-walking implementation demonstrably moved on these islands.
    // A completed search now proves the disconnect and must never return Moved.
    assert!(matches!(
        ready_step(&mut nav, actor, request(12, from, target)),
        Step::Blocked
    ));
    for _ in 0..5 {
        assert!(matches!(
            ready_step(&mut nav, actor, request(12, from, target)),
            Step::Blocked
        ));
    }
}

#[test]
fn navigation_complete_terminal_leg_tracks_a_nearby_moving_victim() {
    let dir = tempfile::tempdir().unwrap();
    let mut nav = Navigation::new(dir.path());
    let actor = Entity::from_bits(99);
    let from = Vec3::ZERO;
    let target = Vec3::new(10.0, 0.0, 0.0);
    let mut movement = request(12, from, target);
    movement.distance = 8.0;
    nav.adopt_candidate_route(actor, movement, vec![from, target]);
    let Step::Moved {
        position: at,
        reached: true,
    } = nav.advance(actor, movement)
    else {
        panic!("complete route must reach the ordinary two-yard sphere");
    };
    movement.from = at;
    movement.to = position(Vec3::new(10.5, 0.0, 0.0));
    movement.distance = 1.0;
    let Step::Moved {
        position: next,
        reached: true,
    } = nav.advance(actor, movement)
    else {
        panic!("sub-yard target motion must reuse the complete terminal leg");
    };
    assert!((next.x - 8.5).abs() < 0.001);
}

#[test]
fn navigation_unbaked_map_is_explicit_legacy_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let mut nav = Navigation::new(dir.path());
    assert!(matches!(
        ready_step(
            &mut nav,
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
        ready_step(
            &mut nav,
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
        ready_step(
            &mut nav,
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
        ready_step(
            &mut nav,
            Entity::from_bits(1),
            request(34, wow(115.6, 46.6, -34.8), wow(128.5, 52.0, -33.86))
        ),
        Step::Blocked
    ));
}
