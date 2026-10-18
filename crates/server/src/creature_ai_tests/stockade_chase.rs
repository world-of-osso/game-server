//! Riverpaw Slayer 20375682 (world.db `content_creature`, map 34) chasing a player through
//! the Stockade, on the real Stockade WMO ground (`stormwindjail.wdt`, one global WMO).
use super::*;
use crate::ground::{GroundMap, GroundProbe};
use shared::components::CreatureMotion;

/// `world/maps/stormwindjail/stormwindjail.wdt`.
const STOCKADE_WDT_FDID: u32 = 791_060;
const STOCKADE: u32 = 34;
/// Riverpaw Slayer 20375682, WoW spawn position.
const SLAYER_SPAWN: (f32, f32, f32) = (115.646, 46.6108, -34.8562);
/// On the cell block floor beside the Riverpaws (WMO floor -33.86).
const PLAYER_ON_FLOOR: (f32, f32, f32) = (128.5, 52.0, -33.86);
/// On the upper corridor over the cell block, where Petty Criminal 46382 stands (floor -26.44).
const PLAYER_ON_CORRIDOR: (f32, f32, f32) = (111.0, 24.6, -26.44);

fn stockade_ground() -> GroundMap {
    GroundMap::new(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        std::collections::HashMap::from([(STOCKADE, STOCKADE_WDT_FDID)]),
    )
}

fn bevy((x, y, z): (f32, f32, f32)) -> Position {
    let (x, y, z) = wow_to_bevy(x, y, z);
    Position { x, y, z }
}

fn stockade_app() -> App {
    let mut app = ai_test_app();
    app.insert_resource(stockade_ground());
    app
}

fn player(app: &mut App, at: (f32, f32, f32)) -> Entity {
    app.world_mut()
        .spawn((bevy(at), crate::maps::InMap::continent(STOCKADE)))
        .id()
}

fn slayer(app: &mut App, state: CreatureState) -> Entity {
    let spawn = bevy(SLAYER_SPAWN);
    app.world_mut()
        .spawn((
            spawn,
            SpawnPoint {
                x: spawn.x,
                y: spawn.y,
                z: spawn.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.142_86,
            },
            state,
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            CreatureMotion::default(),
            crate::maps::InMap::continent(STOCKADE),
        ))
        .id()
}

fn nav_ground_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground")
}

fn nav_stockade_app() -> App {
    let mut app = stockade_app();
    app.insert_resource(crate::navigation::Navigation::new(
        &nav_ground_dir().join("navmesh"),
    ));
    app
}

fn clear_step(los: &mut los::LosMap, from: Position, to: Position) {
    let point = |p: Position| Vec3::new(p.x, p.y + 1.0, p.z);
    assert!(
        los.clear(point(from), point(to)).unwrap(),
        "movement crossed a wall: {from:?} -> {to:?}"
    );
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_stockade_chase_routes_around_walls() {
    let mut app = nav_stockade_app();
    let target = player(&mut app, PLAYER_ON_FLOOR);
    let chaser = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    let mut los = los::LosMap::new(&nav_ground_dir().join("los/34"));
    let mut last = bevy(SLAYER_SPAWN);
    let mut travelled = 0.0;
    for _ in 0..200 {
        app.update();
        let pos = *app.world().get::<Position>(chaser).unwrap();
        clear_step(&mut los, last, pos);
        travelled += (pos.x - last.x).hypot(pos.z - last.z);
        last = pos;
    }
    let goal = bevy(PLAYER_ON_FLOOR);
    assert!(
        (Vec3::new(last.x - goal.x, last.y - goal.y, last.z - goal.z).length() - 2.0).abs() < 0.05,
        "stopped {last:?}, target {goal:?}"
    );
    let spawn = bevy(SLAYER_SPAWN);
    let direct = (goal.x - spawn.x).hypot(goal.z - spawn.z) - 2.0;
    assert!(
        travelled > direct + 1.0,
        "wall detour length {travelled} vs direct {direct}"
    );
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_stockade_home_return_routes_around_walls() {
    let mut app = nav_stockade_app();
    let mob = slayer(&mut app, CreatureState::Returning);
    *app.world_mut().get_mut::<Position>(mob).unwrap() = bevy(PLAYER_ON_FLOOR);
    let mut los = los::LosMap::new(&nav_ground_dir().join("los/34"));
    let mut last = bevy(PLAYER_ON_FLOOR);
    let mut ground = stockade_ground();
    for _ in 0..200 {
        app.update();
        let pos = *app.world().get::<Position>(mob).unwrap();
        clear_step(&mut los, last, pos);
        let floor = floor_under(&mut ground, pos).expect("supported routed home position");
        assert!(
            (pos.y - floor).abs() < 0.05,
            "home feet {pos:?} vs floor{floor}"
        );
        last = pos;
    }
    assert_eq!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Idle
    );
    let home = bevy(SLAYER_SPAWN);
    assert!((last.x - home.x).hypot(last.z - home.z) < 0.05);
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_unreachable_stockade_target_evades_and_clears_threat() {
    use shared::components::ThreatTable;
    let mut app = nav_stockade_app();
    // Real unsupported rock column from ground_tests::stockade_fall_points.
    let target = player(&mut app, (97.0, 76.0, -34.86));
    let mob = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    let mut threat = ThreatTable::default();
    threat.add_damage_threat(target.to_bits(), 100.0, 1.0);
    app.world_mut().entity_mut(mob).insert(threat);
    for _ in 0..99 {
        app.update();
        assert!(matches!(
            *app.world().get::<CreatureState>(mob).unwrap(),
            CreatureState::Pursuing { .. }
        ));
    }
    app.update();
    assert!(
        !matches!(
            *app.world().get::<CreatureState>(mob).unwrap(),
            CreatureState::Pursuing { .. }
        ),
        "unreachable chase must evade after five seconds"
    );
    assert!(app.world().get::<ThreatTable>(mob).unwrap().is_empty());
    for _ in 0..200 {
        app.update();
    }
    assert_eq!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Idle
    );
}

/// Live-supported narrow arch, above the cell floor and on a disconnected nav island.
const ARCH_PERCH: (f32, f32, f32) = (125.0, 46.0, -30.936_882);
/// Exact floor under PLAYER_ON_FLOOR, observed in the private rendered run.
const CELL_FEET: (f32, f32, f32) = (128.5, 52.0, -33.939_564);

fn no_path_perch_fixture() -> (App, Entity, Entity) {
    let mut app = nav_stockade_app();
    let target = player(&mut app, ARCH_PERCH);
    let mob = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    *app.world_mut().get_mut::<Position>(mob).unwrap() = bevy(CELL_FEET);
    let mut threat = shared::components::ThreatTable::default();
    threat.add_damage_threat(target.to_bits(), 100.0, 1.0);
    app.world_mut().entity_mut(mob).insert(threat);
    (app, target, mob)
}

fn assert_waiting_on_perch(app: &App, target: Entity, mob: Entity) {
    assert_eq!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Pursuing {
            target: target.to_bits()
        },
        "Retail no-path grace must not evade before five seconds"
    );
    assert_eq!(*app.world().get::<Position>(mob).unwrap(), bevy(CELL_FEET));
    assert!(
        !app.world()
            .get::<shared::components::ThreatTable>(mob)
            .unwrap()
            .is_empty()
    );
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_no_path_stationary_arch_evades_after_five_seconds_then_returns_home() {
    let (mut app, target, mob) = no_path_perch_fixture();
    let mut ground = stockade_ground();
    let floor = floor_under(&mut ground, bevy(ARCH_PERCH)).expect("arch supports stationary feet");
    assert!((floor - ARCH_PERCH.2).abs() < 0.05);
    for _ in 0..99 {
        app.update();
        assert_waiting_on_perch(&app, target, mob);
        assert_eq!(
            *app.world().get::<Position>(target).unwrap(),
            bevy(ARCH_PERCH)
        );
    }
    app.update();
    assert_eq!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Returning
    );
    assert!(
        app.world()
            .get::<shared::components::ThreatTable>(mob)
            .unwrap()
            .is_empty()
    );
    for _ in 0..200 {
        app.update();
    }
    assert_eq!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Idle
    );
    let at = *app.world().get::<Position>(mob).unwrap();
    let home = bevy(SLAYER_SPAWN);
    assert!((at.x - home.x).hypot(at.z - home.z) < 0.05);
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_no_path_reachable_target_resets_the_timeout() {
    let (mut app, target, mob) = no_path_perch_fixture();
    for _ in 0..80 {
        app.update();
    }
    *app.world_mut().get_mut::<Position>(target).unwrap() = bevy(PLAYER_ON_FLOOR);
    app.update();
    *app.world_mut().get_mut::<Position>(target).unwrap() = bevy(ARCH_PERCH);
    for _ in 0..99 {
        app.update();
        assert_waiting_on_perch(&app, target, mob);
    }
    app.update();
    assert_eq!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Returning
    );
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_no_path_new_victim_starts_a_fresh_timeout() {
    let (mut app, _, mob) = no_path_perch_fixture();
    for _ in 0..80 {
        app.update();
    }
    let target = player(&mut app, ARCH_PERCH);
    *app.world_mut().get_mut::<CreatureState>(mob).unwrap() = CreatureState::Pursuing {
        target: target.to_bits(),
    };
    for _ in 0..99 {
        app.update();
        assert_waiting_on_perch(&app, target, mob);
    }
    app.update();
    assert_eq!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Returning
    );
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_no_path_raid_does_not_apply_the_timeout() {
    use crate::maps::store::{InstanceType, MapEntry, MapStore};
    let (mut app, target, mob) = no_path_perch_fixture();
    let mut maps = MapStore::default();
    maps.maps.insert(
        STOCKADE,
        MapEntry {
            id: STOCKADE,
            directory: "stormwindjail".into(),
            name: "Raid policy fixture on real Stockade geometry".into(),
            instance_type: InstanceType::Raid,
            area_table_id: 0,
            parent_map_id: -1,
            corpse_map_id: 0,
            corpse: (0.0, 0.0),
            max_players: 40,
            wdt_file_data_id: STOCKADE_WDT_FDID,
            expansion_id: 0,
        },
    );
    app.insert_resource(maps);
    for _ in 0..200 {
        app.update();
        assert_waiting_on_perch(&app, target, mob);
    }
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_stockade_waypoint_leg_routes_around_walls() {
    let mut app = nav_stockade_app();
    app.world_mut()
        .resource_mut::<WorldData>()
        .waypoints
        .insert(
            1,
            WaypointPath {
                points: vec![WaypointPoint {
                    position_x: PLAYER_ON_FLOOR.0,
                    position_y: PLAYER_ON_FLOOR.1,
                    position_z: PLAYER_ON_FLOOR.2,
                    delay: 10000,
                    move_type: WaypointMoveType::Run,
                    ..Default::default()
                }],
            },
        );
    let mob = slayer(&mut app, CreatureState::Idle);
    let spawn = bevy(SLAYER_SPAWN);
    app.world_mut()
        .entity_mut(mob)
        .insert(WaypointState::new(1, 1.0, 1.14286, (spawn.x, spawn.z)));
    let mut los = los::LosMap::new(&nav_ground_dir().join("los/34"));
    let mut last = spawn;
    let mut ground = stockade_ground();
    for _ in 0..150 {
        app.update();
        let pos = *app.world().get::<Position>(mob).unwrap();
        clear_step(&mut los, last, pos);
        let floor = floor_under(&mut ground, pos).expect("supported routed waypoint position");
        assert!(
            (pos.y - floor).abs() < 0.05,
            "waypoint feet {pos:?} vs floor{floor}"
        );
        last = pos;
    }
    assert!(app.world().get::<WaypointState>(mob).unwrap().arrived);
    let goal = bevy(PLAYER_ON_FLOOR);
    assert!((last.x - goal.x).hypot(last.z - goal.z) < 0.1);
}

#[test]
#[ignore = "requires nav-bake map34 from local Stockade geometry"]
fn navigation_stockade_repaths_when_target_moves() {
    let mut app = nav_stockade_app();
    let target = player(&mut app, PLAYER_ON_FLOOR);
    let mob = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    for _ in 0..30 {
        app.update();
    }
    let new_goal = bevy((103.0, 76.0, -34.86));
    *app.world_mut().get_mut::<Position>(target).unwrap() = new_goal;
    let mut los = los::LosMap::new(&nav_ground_dir().join("los/34"));
    let mut last = *app.world().get::<Position>(mob).unwrap();
    for _ in 0..300 {
        app.update();
        let pos = *app.world().get::<Position>(mob).unwrap();
        clear_step(&mut los, last, pos);
        last = pos;
    }
    assert!(matches!(
        *app.world().get::<CreatureState>(mob).unwrap(),
        CreatureState::Pursuing { .. }
    ));
    assert!(
        (Vec3::new(
            last.x - new_goal.x,
            last.y - new_goal.y,
            last.z - new_goal.z
        )
        .length()
            - 2.0)
            .abs()
            < 0.1,
        "stopped {last:?}, target {new_goal:?}"
    );
}

/// The floor under `pos`, `None` in the columns of the cell walls and bars, which have no
/// floor face (the straight chase line crosses them).
fn floor_under(ground: &mut GroundMap, pos: Position) -> Option<f32> {
    match ground.ground_at(STOCKADE, Vec3::new(pos.x, pos.y, pos.z)) {
        GroundProbe::Supported(floor) => Some(floor),
        GroundProbe::Unsupported => None,
        GroundProbe::Unloaded => panic!("Stockade ground not loaded"),
    }
}

/// `Rotation.y` of a creature facing along Bevy `(dx, dz)` (WoW orientation + π/2).
fn facing(dx: f32, dz: f32) -> f32 {
    (-dz).atan2(dx) + std::f32::consts::FRAC_PI_2
}

fn same_angle(a: f32, b: f32) -> bool {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    d < 0.01 || d > std::f32::consts::TAU - 0.01
}

/// The slayer runs across the floor to the player: it stands on the floor each tick (the
/// player is higher), faces where it runs with `Run` motion, then stops2yd short in3D,
/// `Still`, facing the player (`ChaseMovementGenerator` `SetInFront`).
#[test]
fn riverpaw_runs_on_the_floor_facing_its_way_and_stops_facing_the_player() {
    let mut app = stockade_app();
    let target = player(&mut app, PLAYER_ON_FLOOR);
    let chaser = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    let mut ground = stockade_ground();
    let mut last = *app.world().get::<Position>(chaser).unwrap();
    let mut ran = 0;
    for tick in 0..60 {
        app.update();
        let pos = *app.world().get::<Position>(chaser).unwrap();
        let rotation = app.world().get::<Rotation>(chaser).unwrap().y;
        let motion = *app.world().get::<CreatureMotion>(chaser).unwrap();
        let (dx, dz) = (pos.x - last.x, pos.z - last.z);
        if dx.hypot(dz) > 0.001 {
            ran += 1;
            assert_eq!(motion, CreatureMotion::Run, "tick {tick}");
            assert!(
                same_angle(rotation, facing(dx, dz)),
                "tick {tick}: {rotation}"
            );
            if let Some(floor) = floor_under(&mut ground, pos) {
                assert!(
                    (pos.y - floor).abs() < 0.05,
                    "tick {tick}: y {} floor {floor}",
                    pos.y
                );
            }
        }
        last = pos;
    }
    assert!(ran > 5, "ran {ran} ticks");
    let target_pos = bevy(PLAYER_ON_FLOOR);
    // TC a352b1fa Position.h121–127/155–157 and PathGenerator.cpp982–1010 use3D.
    // Keep the original2yd value and0.01yd tolerance, not its XZ projection.
    let gap = Vec3::new(
        target_pos.x - last.x,
        target_pos.y - last.y,
        target_pos.z - last.z,
    )
    .length();
    assert!((gap - 2.0).abs() < 0.01, "stops2yd short in3D: {gap}");
    assert_eq!(
        *app.world().get::<CreatureMotion>(chaser).unwrap(),
        CreatureMotion::Still
    );
    let rotation = app.world().get::<Rotation>(chaser).unwrap().y;
    assert!(
        same_angle(
            rotation,
            facing(target_pos.x - last.x, target_pos.z - last.z)
        ),
        "faces the player: {rotation}"
    );
}

/// A player on the corridor 8 yd above: the slayer stays on the cell block floor under it
/// instead of rising through the air toward the player's height.
#[test]
fn riverpaw_chasing_a_player_above_stays_on_its_floor() {
    let mut app = stockade_app();
    let target = player(&mut app, PLAYER_ON_CORRIDOR);
    let chaser = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    let mut ground = stockade_ground();
    for tick in 0..80 {
        app.update();
        let pos = *app.world().get::<Position>(chaser).unwrap();
        if let Some(floor) = floor_under(&mut ground, pos) {
            assert!(
                (pos.y - floor).abs() < 0.05,
                "tick {tick}: WoW z {} over the floor at {floor}",
                pos.y
            );
        }
    }
}

/// Evading from the player back to its spawn a yard lower, the slayer runs on the floor
/// (it kept the player's floor height until arrival), `Run` until home, then `Still`.
#[test]
fn riverpaw_returns_home_on_the_floor() {
    let mut app = stockade_app();
    let chaser = slayer(&mut app, CreatureState::Returning);
    *app.world_mut().get_mut::<Position>(chaser).unwrap() = bevy(PLAYER_ON_FLOOR);
    let mut ground = stockade_ground();
    let mut last = bevy(PLAYER_ON_FLOOR);
    for tick in 0..60 {
        app.update();
        let pos = *app.world().get::<Position>(chaser).unwrap();
        let motion = *app.world().get::<CreatureMotion>(chaser).unwrap();
        let returning =
            *app.world().get::<CreatureState>(chaser).unwrap() == CreatureState::Returning;
        if returning && (pos.x - last.x).hypot(pos.z - last.z) > 0.001 {
            assert_eq!(motion, CreatureMotion::Run, "tick {tick}");
        }
        if let Some(floor) = floor_under(&mut ground, pos) {
            assert!(
                (pos.y - floor).abs() < 0.05,
                "tick {tick}: WoW z {} over the floor at {floor}",
                pos.y
            );
        }
        last = pos;
    }
    assert_eq!(
        *app.world().get::<CreatureState>(chaser).unwrap(),
        CreatureState::Idle
    );
    assert_eq!(
        *app.world().get::<CreatureMotion>(chaser).unwrap(),
        CreatureMotion::Still
    );
}

/// Run22 geometry with a constructed valid cached destination (<1yd victim drift).
/// This is a production chase RED, not a claim that the exact cached route was logged.
#[test]
#[ignore = "requires real Stockade navigation and server LOS bake"]
fn current_victim_stockade_cached_corner_does_not_pass_in_range_visible_victim() {
    use bevy::ecs::system::RunSystemOnce;
    let (mut app, victim, chaser, from) = current_victim::fixture();
    let current = *app.world().get::<Position>(victim).unwrap();
    let map = crate::maps::InMap::continent(STOCKADE);
    assert!(
        Vec3::new(current.x - from.x, current.y - from.y, current.z - from.z).length()
            < CHASE_REACH
    );
    let mut bodies = app.world_mut().query::<crate::line_of_sight::BodyParts>();
    let world = app.world();
    assert!(
        crate::line_of_sight::unit_sees(
            Some(world.resource::<crate::line_of_sight::LineOfSight>()),
            map,
            bodies.get(world, chaser).unwrap(),
            bodies.get(world, victim).unwrap()
        ),
        "current victim has authoritative server LOS"
    );
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(50));
    app.world_mut().run_system_once(chase_system).unwrap();
    let after = *app.world().get::<Position>(chaser).unwrap();
    assert_eq!(
        after, from,
        "already in reach+LOS: cached pre-terminal corners must not move the chaser"
    );
}

#[path = "current_victim.rs"]
mod current_victim;
