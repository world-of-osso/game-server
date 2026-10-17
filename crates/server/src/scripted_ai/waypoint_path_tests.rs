//! Generic movement-validation regressions, not invented encounter path data.
use super::*;
use crate::scripted_ai::{CreatureAi, CreatureAiState, Ctx};
use bevy::ecs::system::RunSystemOnce;
use std::time::Duration;

#[test]
fn waypoint_source_short_initial_segment_still_moves_to_the_real_destination() {
    // TC MoveSpline.cpp::_checkPathLengths starts at segment1, not segment0.
    let spline = launch(
        vec![
            Vec3::ZERO,
            Vec3::new(0.05, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
        ],
        false,
        0.0,
        5.0,
    )
    .expect("source admits a short first crossing");
    // Native cumulative millisecond truncation permits 5mm per segment here.
    assert!((spline.position(501).x - 2.5).abs() < 0.01);
    assert_eq!(spline.position(spline.end_ms), Vec3::new(5.0, 0.0, 0.0));
}

#[test]
fn waypoint_short_height_corner_remains_rejected() {
    let short_height_step = vec![
        Vec3::ZERO,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.05, 0.0),
        Vec3::new(3.0, 0.05, 0.0),
    ];
    assert!(
        launch(short_height_step, false, 0.0, 2.0).is_none(),
        "a real short height corner remains rejected, never flattened"
    );
}

#[test]
#[ignore = "requires WAILING_NAV_ROOT containing the private authored map43 bake"]
fn waypoint_authored_wailing_node2_launches_and_reaches_collision_floor() {
    let root = std::path::PathBuf::from(std::env::var_os("WAILING_NAV_ROOT").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let paths = load_script_paths(&db).unwrap();
    let path = &paths.0[&367801];
    let node = path.nodes.iter().position(|(id, _)| *id == 2).unwrap();
    // Captured real launch boundary after walking path0 and killing its two Ravagers.
    let from = Vec3::new(-100.10522, 231.44879, -90.95079);
    let spawn = SpawnPoint {
        x: from.x,
        y: from.z,
        z: -from.y,
        wander_distance: 0.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    let source = Launch {
        map: crate::maps::InMap::continent(43),
        position: from,
        orientation: 0.0,
        spawn: &spawn,
        walking: true,
        flying: false,
    };
    let navigation = crate::navigation::Navigation::new(&root);
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        HashMap::from([(43, maps.map(43).unwrap().wdt_file_data_id)]),
    );
    let controls =
        query_path_controls(path, node, &source, Some(&navigation), Some(&mut ground)).unwrap();
    let spline = launch(controls.clone(), false, 0.0, 2.5)
        .unwrap_or_else(|| panic!("authored node2 must launch: {controls:?}"));
    assert_eq!(spline.position(MINIMAL_DURATION_MS), from);
    let destination = spline.position(spline.end_ms);
    assert!((destination.x - path.nodes[node].1.x).abs() < 0.01);
    assert!((destination.y - path.nodes[node].1.y).abs() < 0.01);
    let feet = Vec3::new(destination.x, destination.z, -destination.y);
    let crate::ground::GroundProbe::Supported(floor) = ground.creature_ground_at(43, feet) else {
        panic!("node2 destination requires exact collision support");
    };
    assert!(
        (destination.z - floor).abs() < 0.05,
        "node2 must normalize to exact collision, not approximate nav height: {destination:?} floor={floor}"
    );
    println!("authored node2 reached {destination:?}: {controls:?}");
}

#[test]
#[ignore = "requires WAILING_NAV_ROOT containing the private authored map43 bake"]
fn waypoint_authored_wailing_all_nodes_preserve_collision() {
    let root = std::path::PathBuf::from(std::env::var_os("WAILING_NAV_ROOT").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let paths = load_script_paths(&db).unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        HashMap::from([(43, maps.map(43).unwrap().wdt_file_data_id)]),
    );
    let navigation = crate::navigation::Navigation::new(&root);
    let mut los = los::LosMap::new(&root.parent().unwrap().parent().unwrap().join("los/43"));
    let mut at = Vec3::new(-134.965, 125.402, -78.0945);
    let mut nodes = 0;
    for id in [367800, 367801, 367802] {
        let path = &paths.0[&id];
        for node in 0..path.nodes.len() {
            let spawn = SpawnPoint {
                x: at.x,
                y: at.z,
                z: -at.y,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            };
            let source = Launch {
                map: crate::maps::InMap::continent(43),
                position: at,
                orientation: 0.0,
                spawn: &spawn,
                walking: true,
                flying: false,
            };
            let controls =
                query_path_controls(path, node, &source, Some(&navigation), Some(&mut ground))
                    .unwrap_or_else(|error| {
                        panic!("path{id}/node{} generation: {error}", path.nodes[node].0)
                    });
            let spline =
                launch(controls, false, 0.0, 2.5).expect("strict source spline validation");
            let mut previous = at;
            for time in (MINIMAL_DURATION_MS..spline.end_ms)
                .step_by(50)
                .chain(std::iter::once(spline.end_ms))
            {
                let point = spline.position(time);
                let a = Vec3::new(previous.x, previous.z, -previous.y);
                let b = Vec3::new(point.x, point.z, -point.y);
                assert!(
                    los.clear(a + Vec3::Y, b + Vec3::Y).unwrap(),
                    "script controls cross collision path{id}/node{}: {previous:?}->{point:?}",
                    path.nodes[node].0
                );
                previous = point;
            }
            at = spline.position(spline.end_ms);
            nodes += 1;
        }
    }
    assert_eq!(nodes, 49);
    println!(
        "all {nodes} authored ground-waypoint nodes validated and collision-clear; end={at:?}"
    );
}

#[test]
fn waypoint_flying_and_exact_controls_do_not_require_ground_navigation() {
    let spawn = SpawnPoint {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        wander_distance: 0.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    let mut path = ScriptPath {
        id: 1,
        move_type: MoveType::Run,
        flags: 0,
        velocity: None,
        has_node_timing: false,
        nodes: vec![(0, Vec3::new(2.0, 3.0, 4.0)), (1, Vec3::new(4.0, 5.0, 6.0))],
    };
    let source = Launch {
        map: crate::maps::InMap::continent(43),
        position: Vec3::ZERO,
        orientation: 0.0,
        spawn: &spawn,
        walking: false,
        flying: true,
    };
    assert_eq!(
        query_path_controls(&path, 0, &source, None, None).unwrap(),
        vec![Vec3::ZERO, path.nodes[0].1]
    );
    path.flags = FLAG_EXACT_SPLINE;
    assert_eq!(
        query_path_controls(&path, 0, &source, None, None).unwrap(),
        vec![Vec3::ZERO, path.nodes[0].1, path.nodes[1].1]
    );
    let source = Launch {
        flying: false,
        ..source
    };
    assert_eq!(
        query_path_controls(&path, 0, &source, None, None).unwrap(),
        vec![Vec3::ZERO, path.nodes[0].1, path.nodes[1].1]
    );
}

#[test]
#[ignore = "requires WAILING_NAV_ROOT containing the private authored map43 bake"]
fn waypoint_wailing_single_chase_snap_samples_both_real_floors() {
    let root = std::path::PathBuf::from(std::env::var_os("WAILING_NAV_ROOT").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        HashMap::from([(43, maps.map(43).unwrap().wdt_file_data_id)]),
    );
    let navigation = crate::navigation::Navigation::new(&root);
    let mut los = los::LosMap::new(&root.parent().unwrap().parent().unwrap().join("los/43"));
    for (label, endpoints) in [
        (
            "single-chase handoff",
            [
                Vec3::new(-55.3435, -95.25101, -205.98676),
                Vec3::new(-54.952423, -95.577065, -206.07079),
            ],
        ),
        (
            "camp2 nav edge",
            [
                Vec3::new(-52.340485, -92.84426, -267.44452),
                Vec3::new(-52.42458, -92.84426, -267.5363),
            ],
        ),
    ] {
        let floors: Vec<_> = endpoints
            .iter()
            .map(|&feet| {
                let crate::ground::GroundProbe::Supported(floor) =
                    ground.creature_ground_at(43, feet)
                else {
                    panic!("recorded endpoint has no collision support: {feet:?}");
                };
                // Existing nearest-poly policy admits only WALK_FLAG polygons, baked
                // under AGENT_SLOPE/AGENT_CLIMB; no new looser search or slope policy.
                let support = navigation.query_path(43, feet, feet).unwrap().unwrap();
                println!("recorded feet={feet:?} actual_floor={floor} walkable_nav={support:?}");
                floor
            })
            .collect();
        assert!((endpoints[1].y - floors[1]).abs() < 0.05);
        let horizontal = (endpoints[1].x - endpoints[0].x).hypot(endpoints[1].z - endpoints[0].z);
        let slope = (floors[1] - floors[0]).abs().atan2(horizontal).to_degrees();
        assert!(
            slope <= navigation::AGENT_SLOPE,
            "real floor segment slope={slope}, baked limit={}",
            navigation::AGENT_SLOPE
        );
        println!(
            "horizontal={horizontal} real_floor_slope={slope}deg; prior feet offset={}",
            endpoints[0].y - floors[0]
        );
        println!(
            "{label}: surface-ray={:?} LOS={:?}",
            navigation.first_collision(43, endpoints[0], endpoints[1]),
            los.clear(endpoints[0] + Vec3::Y, endpoints[1] + Vec3::Y)
        );
    }
}

fn motion_ownership_world() -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.init_resource::<ArrivalReceipts>();
    world.insert_resource(ScriptPaths(HashMap::from([(
        1,
        Arc::new(ScriptPath {
            id: 1,
            move_type: MoveType::Walk,
            flags: FLAG_EXACT_SPLINE,
            velocity: None,
            has_node_timing: false,
            nodes: vec![
                (0, Vec3::new(6.0, 0.0, 0.0)),
                (1, Vec3::new(12.0, 0.0, 0.0)),
            ],
        }),
    )])));
    let actor = world
        .spawn((
            bevy(Vec3::ZERO),
            crate::maps::InMap::continent(0),
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            crate::creature_ai::CreatureState::Idle,
            CreatureMotion::Still,
            MovementSpeed(2.5),
            CreatureAiState {
                ai: Some(Box::new(ArrivalConsumer)),
                pending: Default::default(),
            },
        ))
        .id();
    let target = world
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            crate::maps::InMap::continent(0),
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    assert!(move_path(&mut world, actor, 1));
    (world, actor, target)
}

fn tick_owned_motion(world: &mut World) {
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(50));
    path_movement_system(world);
    world
        .run_system_once(crate::creature_ai::chase_system)
        .unwrap();
}

#[test]
fn waypoint_chase_owns_tick_then_path_resumes_from_real_position_without_replaying_nodes() {
    let (mut world, actor, target) = motion_ownership_world();
    for _ in 0..80 {
        tick_owned_motion(&mut world);
    }
    let at = *world.get::<Position>(actor).unwrap();
    assert!(
        at.x > 6.0 && at.x < 12.0,
        "past first authored node before preemption: {at:?}"
    );
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::CreatureState::Pursuing {
            target: target.to_bits(),
        });
    let target = *world.get::<Position>(target).unwrap();
    let target = Vec3::new(target.x, target.y, target.z);
    for active_tick in 0..20 {
        let before = *world.get::<Position>(actor).unwrap();
        let before = Vec3::new(before.x, before.y, before.z);
        let expected = if active_tick == 0 {
            before // MotionMaster initializes a newly selected top with diff0.
        } else {
            before + (target - before).normalize() * (7.0 * 0.05)
        };
        tick_owned_motion(&mut world);
        let actual = *world.get::<Position>(actor).unwrap();
        let actual = Vec3::new(actual.x, actual.y, actual.z);
        assert!(
            actual.distance(expected) < 0.0001,
            "only Chase may drive while active; before={before:?} actual={actual:?} expected={expected:?}"
        );
    }
    assert!(world.resource::<ArrivalReceipts>().0.is_empty());
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::CreatureState::Idle);
    let destination = Vec3::new(12.0, 0.0, 0.0);
    for resumed_tick in 0..40 {
        let before = *world.get::<Position>(actor).unwrap();
        let before = Vec3::new(before.x, before.y, before.z);
        tick_owned_motion(&mut world);
        let actual = *world.get::<Position>(actor).unwrap();
        let actual = Vec3::new(actual.x, actual.y, actual.z);
        let step = actual - before;
        if resumed_tick < 2 {
            assert_eq!(
                actual, before,
                "reset gets diff0, then source1ms wait launches without a position step"
            );
        }
        assert!(
            step.length() <= 0.126,
            "no replay/relocation on resume: {before:?}->{actual:?}"
        );
        if step.length() > 0.001 {
            assert!(
                step.normalize().dot((destination - before).normalize()) > 0.9999,
                "resume the remaining node, not the already-visited first node: {before:?}->{actual:?}"
            );
        }
    }
    for _ in 0..200 {
        tick_owned_motion(&mut world);
    }
    assert_eq!(*world.get::<Position>(actor).unwrap(), bevy(destination));
    assert_eq!(world.resource::<ArrivalReceipts>().0, vec![(1, 1)]);
}

#[test]
fn waypoint_new_chase_initializes_without_second_position_step_in_handoff_tick() {
    let (mut world, actor, target) = motion_ownership_world();
    for _ in 0..80 {
        tick_owned_motion(&mut world);
    }
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(50));
    path_movement_system(&mut world);
    let after_waypoint = *world.get::<Position>(actor).unwrap();
    // Same observed ordering as Wailing: sight/aggro activates chase after the
    // default spline has already advanced, before the active mover updates.
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::CreatureState::Pursuing {
            target: target.to_bits(),
        });
    world
        .run_system_once(crate::creature_ai::chase_system)
        .unwrap();
    assert_eq!(
        *world.get::<Position>(actor).unwrap(),
        after_waypoint,
        "new active chase initializes; it cannot add a second movement step"
    );
    tick_owned_motion(&mut world);
    let actual = *world.get::<Position>(actor).unwrap();
    let distance = Vec3::new(
        actual.x - after_waypoint.x,
        actual.y - after_waypoint.y,
        actual.z - after_waypoint.z,
    )
    .length();
    assert!(
        (distance - 0.35).abs() < 0.0001,
        "next tick belongs to active chase: {distance}"
    );
}

#[test]
fn waypoint_new_home_initializes_without_second_position_step_in_handoff_tick() {
    let (mut world, actor, _) = motion_ownership_world();
    for _ in 0..80 {
        tick_owned_motion(&mut world);
    }
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(50));
    path_movement_system(&mut world);
    let after_waypoint = *world.get::<Position>(actor).unwrap();
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::CreatureState::Returning);
    world
        .run_system_once(crate::creature_ai::return_to_spawn_system)
        .unwrap();
    assert_eq!(
        *world.get::<Position>(actor).unwrap(),
        after_waypoint,
        "new active home initializes; it cannot add a second movement step"
    );
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(50));
    path_movement_system(&mut world);
    world
        .run_system_once(crate::creature_ai::return_to_spawn_system)
        .unwrap();
    let actual = *world.get::<Position>(actor).unwrap();
    assert!(
        (actual.x - (after_waypoint.x - 0.35)).abs() < 0.0001,
        "next tick belongs to active home: {after_waypoint:?}->{actual:?}"
    );
}

#[test]
fn waypoint_completion_handoff_does_not_add_chase_after_the_final_path_step() {
    let (mut world, actor, target) = motion_ownership_world();
    for _ in 0..200 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        path_movement_system(&mut world);
        if !world.resource::<ArrivalReceipts>().0.is_empty() {
            break;
        }
    }
    assert_eq!(world.resource::<ArrivalReceipts>().0, vec![(1, 1)]);
    let after_waypoint = *world.get::<Position>(actor).unwrap();
    assert_eq!(after_waypoint, bevy(Vec3::new(12.0, 0.0, 0.0)));
    // The completion callback may engage a target before active motion runs.
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::CreatureState::Pursuing {
            target: target.to_bits(),
        });
    world
        .run_system_once(crate::creature_ai::chase_system)
        .unwrap();
    assert_eq!(
        *world.get::<Position>(actor).unwrap(),
        after_waypoint,
        "finishing the default path cannot lose that tick's sole position owner"
    );
    tick_owned_motion(&mut world);
    let actual = *world.get::<Position>(actor).unwrap();
    let distance = Vec3::new(
        actual.x - after_waypoint.x,
        actual.y - after_waypoint.y,
        actual.z - after_waypoint.z,
    )
    .length();
    assert!((distance - 0.35).abs() < 0.0001);
    assert_eq!(world.resource::<ArrivalReceipts>().0, vec![(1, 1)]);
}

#[test]
#[ignore = "requires WAILING_NAV_ROOT containing the private authored map43 bake"]
fn waypoint_missing_support_stops_actual_motion_without_reporting_arrival() {
    let (mut world, actor, _) = motion_ownership_world();
    let root = std::path::PathBuf::from(std::env::var_os("WAILING_NAV_ROOT").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let wdts = HashMap::from([(43, maps.map(43).unwrap().wdt_file_data_id)]);
    world.insert_resource(load_script_paths(&db).unwrap());
    world.insert_resource(crate::navigation::Navigation::new(&root));
    world.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        wdts.clone(),
    ));
    world.entity_mut(actor).insert((
        bevy(Vec3::new(-134.965, 125.402, -78.0945)),
        crate::maps::InMap::continent(43),
    ));
    assert!(move_path(&mut world, actor, 367800));
    for _ in 0..40 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        path_movement_system(&mut world);
    }
    assert_eq!(
        world.get::<CreatureMotion>(actor),
        Some(&CreatureMotion::Walk)
    );
    let last_supported = *world.get::<Position>(actor).unwrap();
    // Simulate loss of authoritative geometry, without changing/bypassing nav,
    // touching real assets or moving the actor to manufacture an arrival.
    let missing = tempfile::tempdir().unwrap();
    world.insert_resource(crate::ground::GroundMap::new(missing.path(), wdts));
    for _ in 0..100 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        path_movement_system(&mut world);
    }
    assert_eq!(*world.get::<Position>(actor).unwrap(), last_supported);
    assert_eq!(
        world.get::<CreatureMotion>(actor),
        Some(&CreatureMotion::Still)
    );
    assert!(world.resource::<ArrivalReceipts>().0.is_empty());
}

#[test]
fn waypoint_active_chase_to_home_switch_initializes_new_top_without_second_step() {
    let (mut world, actor, target) = motion_ownership_world();
    for _ in 0..80 {
        tick_owned_motion(&mut world);
    }
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::CreatureState::Pursuing {
            target: target.to_bits(),
        });
    tick_owned_motion(&mut world);
    tick_owned_motion(&mut world);
    let after_chase = *world.get::<Position>(actor).unwrap();
    // Same active-slot transition as a queued evade after the chase update.
    world
        .entity_mut(actor)
        .insert(crate::creature_ai::CreatureState::Returning);
    world
        .run_system_once(crate::creature_ai::return_to_spawn_system)
        .unwrap();
    assert_eq!(
        *world.get::<Position>(actor).unwrap(),
        after_chase,
        "home is a new top generator; it cannot add a step after chase"
    );
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(50));
    path_movement_system(&mut world);
    world
        .run_system_once(crate::creature_ai::return_to_spawn_system)
        .unwrap();
    let actual = *world.get::<Position>(actor).unwrap();
    let distance = Vec3::new(
        actual.x - after_chase.x,
        actual.y - after_chase.y,
        actual.z - after_chase.z,
    )
    .length();
    assert!((distance - 0.35).abs() < 0.0001);
}

#[derive(Resource, Default)]
struct ArrivalReceipts(Vec<(u32, u32)>);

struct ArrivalConsumer;
impl CreatureAi for ArrivalConsumer {
    fn waypoint_path_ended(&mut self, ctx: &mut Ctx, node: u32, path: u32) {
        ctx.world
            .resource_mut::<ArrivalReceipts>()
            .0
            .push((node, path));
    }
}

#[test]
fn waypoint_rejected_interior_segment_never_moves_or_reports_a_path_arrival() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.init_resource::<ArrivalReceipts>();
    world.insert_resource(ScriptPaths(HashMap::from([(
        1,
        Arc::new(ScriptPath {
            id: 1,
            move_type: MoveType::Run,
            flags: FLAG_EXACT_SPLINE,
            velocity: None,
            has_node_timing: false,
            nodes: vec![
                (0, Vec3::new(2.0, 0.0, 0.0)),
                (1, Vec3::new(2.05, 0.0, 0.0)),
                (2, Vec3::new(5.0, 0.0, 0.0)),
            ],
        }),
    )])));
    let actor = world
        .spawn((
            bevy(Vec3::ZERO),
            crate::maps::InMap::continent(0),
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureMotion::Still,
            CreatureAiState {
                ai: Some(Box::new(ArrivalConsumer)),
                pending: Default::default(),
            },
        ))
        .id();
    assert!(move_path(&mut world, actor, 1));
    for _ in 0..200 {
        world
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        path_movement_system(&mut world);
    }
    assert_eq!(*world.get::<Position>(actor).unwrap(), bevy(Vec3::ZERO));
    assert!(
        world.resource::<ArrivalReceipts>().0.is_empty(),
        "no synthetic script arrival after rejected controls"
    );
}
