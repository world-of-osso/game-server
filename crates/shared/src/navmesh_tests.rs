use super::*;

fn sample_tile() -> NavTile {
    // Simple square tile with one walkable quad polygon
    NavTile {
        map_id: 0,
        tile_x: 32,
        tile_y: 32,
        vertices: vec![
            NavPoint::new(0.0, 100.0, 0.0),
            NavPoint::new(10.0, 100.0, 0.0),
            NavPoint::new(10.0, 100.0, 10.0),
            NavPoint::new(0.0, 100.0, 10.0),
        ],
        polygons: vec![NavPolygon {
            id: 0,
            vertices: vec![0, 1, 2, 3],
            neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
            flags: NavFlags {
                walkable: true,
                ..Default::default()
            },
        }],
    }
}

#[test]
fn nav_point_distance() {
    let a = NavPoint::new(0.0, 0.0, 0.0);
    let b = NavPoint::new(3.0, 4.0, 0.0);
    assert!((a.distance_to(&b) - 5.0).abs() < 0.001);
}

#[test]
fn nav_point_distance_2d() {
    let a = NavPoint::new(0.0, 100.0, 0.0);
    let b = NavPoint::new(3.0, 200.0, 4.0);
    assert!((a.distance_2d(&b) - 5.0).abs() < 0.001);
}

#[test]
fn find_polygon_inside() {
    let tile = sample_tile();
    let point = NavPoint::new(5.0, 0.0, 5.0);
    assert_eq!(tile.find_polygon(&point), Some(0));
}

#[test]
fn find_polygon_outside() {
    let tile = sample_tile();
    let point = NavPoint::new(20.0, 0.0, 20.0);
    assert_eq!(tile.find_polygon(&point), None);
}

#[test]
fn height_at_inside_polygon() {
    let tile = sample_tile();
    let height = tile.height_at(5.0, 5.0).unwrap();
    assert!((height - 100.0).abs() < 0.001);
}

#[test]
fn height_at_outside_returns_none() {
    let tile = sample_tile();
    assert!(tile.height_at(50.0, 50.0).is_none());
}

#[test]
fn world_to_tile_center() {
    let (tx, ty) = world_to_tile(0.0, 0.0);
    assert_eq!(tx, 32);
    assert_eq!(ty, 32);
}

#[test]
fn nav_flags_default() {
    let flags = NavFlags::default();
    assert!(!flags.walkable);
    assert!(!flags.water);
    assert!(!flags.steep);
    assert!(!flags.indoor);
}

#[test]
fn tile_with_water_and_walkable() {
    let tile = NavTile {
        map_id: 0,
        tile_x: 0,
        tile_y: 0,
        vertices: vec![
            NavPoint::new(0.0, 50.0, 0.0),
            NavPoint::new(10.0, 50.0, 0.0),
            NavPoint::new(10.0, 50.0, 10.0),
        ],
        polygons: vec![NavPolygon {
            id: 0,
            vertices: vec![0, 1, 2],
            neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
            flags: NavFlags {
                walkable: true,
                water: true,
                ..Default::default()
            },
        }],
    };
    let poly = tile.find_polygon(&NavPoint::new(3.0, 0.0, 3.0));
    assert_eq!(poly, Some(0));
    assert!(tile.polygons[0].flags.water);
}

// --- A* pathfinding tests ---

fn walkable_nav_flags() -> NavFlags {
    NavFlags {
        walkable: true,
        ..Default::default()
    }
}

fn path_tile_vertices() -> Vec<NavPoint> {
    vec![
        NavPoint::new(0.0, 0.0, 0.0),   // 0
        NavPoint::new(10.0, 0.0, 0.0),  // 1
        NavPoint::new(0.0, 0.0, 10.0),  // 2
        NavPoint::new(20.0, 0.0, 0.0),  // 3
        NavPoint::new(10.0, 0.0, 10.0), // 4
        NavPoint::new(30.0, 0.0, 0.0),  // 5
        NavPoint::new(20.0, 0.0, 10.0), // 6
        NavPoint::new(30.0, 0.0, 10.0), // 7
    ]
}

fn path_tile_polygons(flags: NavFlags) -> Vec<NavPolygon> {
    vec![
        NavPolygon {
            id: 0,
            vertices: vec![0, 1, 4, 2],
            neighbors: vec![NO_NEIGHBOR, 1, NO_NEIGHBOR, NO_NEIGHBOR],
            flags,
        },
        NavPolygon {
            id: 1,
            vertices: vec![1, 3, 6, 4],
            neighbors: vec![NO_NEIGHBOR, 2, NO_NEIGHBOR, 0],
            flags,
        },
        NavPolygon {
            id: 2,
            vertices: vec![3, 5, 7, 6],
            neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR, 1],
            flags,
        },
    ]
}

fn path_tile() -> NavTile {
    NavTile {
        map_id: 0,
        tile_x: 0,
        tile_y: 0,
        vertices: path_tile_vertices(),
        polygons: path_tile_polygons(walkable_nav_flags()),
    }
}

#[test]
fn path_same_polygon() {
    let tile = path_tile();
    let start = NavPoint::new(2.0, 0.0, 5.0);
    let end = NavPoint::new(8.0, 0.0, 5.0);
    let path = tile.find_path(&start, &end).unwrap();
    assert_eq!(path.waypoints.len(), 2);
    assert_eq!(path.waypoints[0], start);
    assert_eq!(path.waypoints[1], end);
}

#[test]
fn path_across_two_polygons() {
    let tile = path_tile();
    let start = NavPoint::new(2.0, 0.0, 5.0);
    let end = NavPoint::new(15.0, 0.0, 5.0);
    let path = tile.find_path(&start, &end).unwrap();
    assert!(path.waypoints.len() >= 2);
    assert_eq!(*path.waypoints.first().unwrap(), start);
    assert_eq!(*path.waypoints.last().unwrap(), end);
}

#[test]
fn path_across_three_polygons() {
    let tile = path_tile();
    let start = NavPoint::new(2.0, 0.0, 5.0);
    let end = NavPoint::new(28.0, 0.0, 5.0);
    let path = tile.find_path(&start, &end).unwrap();
    assert!(path.waypoints.len() >= 3);
}

#[test]
fn path_outside_returns_none() {
    let tile = path_tile();
    let outside = NavPoint::new(100.0, 0.0, 100.0);
    assert!(
        tile.find_path(&NavPoint::new(5.0, 0.0, 5.0), &outside)
            .is_none()
    );
}

#[test]
fn path_no_connection_returns_none() {
    // Two disconnected polygons (no neighbors)
    let tile = NavTile {
        map_id: 0,
        tile_x: 0,
        tile_y: 0,
        vertices: vec![
            NavPoint::new(0.0, 0.0, 0.0),
            NavPoint::new(5.0, 0.0, 0.0),
            NavPoint::new(0.0, 0.0, 5.0),
            NavPoint::new(20.0, 0.0, 0.0),
            NavPoint::new(25.0, 0.0, 0.0),
            NavPoint::new(20.0, 0.0, 5.0),
        ],
        polygons: vec![
            NavPolygon {
                id: 1,
                vertices: vec![0, 1, 2],
                neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
                flags: NavFlags {
                    walkable: true,
                    ..Default::default()
                },
            },
            NavPolygon {
                id: 2,
                vertices: vec![3, 4, 5],
                neighbors: vec![NO_NEIGHBOR, NO_NEIGHBOR, NO_NEIGHBOR],
                flags: NavFlags {
                    walkable: true,
                    ..Default::default()
                },
            },
        ],
    };
    let start = NavPoint::new(2.0, 0.0, 2.0);
    let end = NavPoint::new(22.0, 0.0, 2.0);
    assert!(tile.find_path(&start, &end).is_none());
}

// --- Chase tests ---

#[test]
fn chase_reached_when_in_range() {
    let mut chase = ChaseState::new(1, NavPoint::new(5.0, 0.0, 5.0), 7.0);
    chase.set_path(
        NavPath {
            waypoints: vec![NavPoint::new(0.0, 0.0, 0.0), NavPoint::new(5.0, 0.0, 5.0)],
        },
        NavPoint::new(5.0, 0.0, 5.0),
    );
    let result = chase.tick(
        0.016,
        &NavPoint::new(4.0, 0.0, 5.0),
        &NavPoint::new(5.0, 0.0, 5.0),
        5.0,
    );
    assert_eq!(result, ChaseTickResult::Reached);
}

#[test]
fn chase_moves_toward_waypoint() {
    let mut chase = ChaseState::new(1, NavPoint::new(20.0, 0.0, 0.0), 7.0);
    chase.set_path(
        NavPath {
            waypoints: vec![
                NavPoint::new(0.0, 0.0, 0.0),
                NavPoint::new(10.0, 0.0, 0.0),
                NavPoint::new(20.0, 0.0, 0.0),
            ],
        },
        NavPoint::new(20.0, 0.0, 0.0),
    );
    let result = chase.tick(
        0.016,
        &NavPoint::new(0.0, 0.0, 0.0),
        &NavPoint::new(20.0, 0.0, 0.0),
        5.0,
    );
    assert!(matches!(result, ChaseTickResult::MoveTo(_)));
}

#[test]
fn chase_need_repath_no_path() {
    let mut chase = ChaseState::new(1, NavPoint::new(20.0, 0.0, 0.0), 7.0);
    let result = chase.tick(
        0.016,
        &NavPoint::new(0.0, 0.0, 0.0),
        &NavPoint::new(20.0, 0.0, 0.0),
        5.0,
    );
    assert_eq!(result, ChaseTickResult::NeedRepath);
}

#[test]
fn chase_need_repath_target_moved() {
    let mut chase = ChaseState::new(1, NavPoint::new(10.0, 0.0, 0.0), 7.0);
    chase.set_path(
        NavPath {
            waypoints: vec![NavPoint::new(0.0, 0.0, 0.0), NavPoint::new(10.0, 0.0, 0.0)],
        },
        NavPoint::new(10.0, 0.0, 0.0),
    );
    // Target moved significantly
    let result = chase.tick(
        0.016,
        &NavPoint::new(0.0, 0.0, 0.0),
        &NavPoint::new(20.0, 0.0, 0.0),
        5.0,
    );
    assert_eq!(result, ChaseTickResult::NeedRepath);
}

#[test]
fn chase_need_repath_timer_expired() {
    let mut chase = ChaseState::new(1, NavPoint::new(20.0, 0.0, 0.0), 7.0);
    chase.set_path(
        NavPath {
            waypoints: vec![NavPoint::new(0.0, 0.0, 0.0), NavPoint::new(20.0, 0.0, 0.0)],
        },
        NavPoint::new(20.0, 0.0, 0.0),
    );
    // Tick past repath interval
    let result = chase.tick(
        2.0,
        &NavPoint::new(0.0, 0.0, 0.0),
        &NavPoint::new(20.0, 0.0, 0.0),
        5.0,
    );
    assert_eq!(result, ChaseTickResult::NeedRepath);
}

#[test]
fn chase_recalculates_path_when_target_moves() {
    let mob_pos = NavPoint::new(0.0, 0.0, 0.0);
    let initial_target = NavPoint::new(20.0, 0.0, 0.0);
    let mut chase = ChaseState::new(1, initial_target, 7.0);

    // Set initial path toward target at (20, 0, 0)
    chase.set_path(
        NavPath {
            waypoints: vec![NavPoint::new(10.0, 0.0, 0.0), initial_target],
        },
        initial_target,
    );

    // First tick: follows path normally
    let result = chase.tick(0.1, &mob_pos, &initial_target, 5.0);
    assert_eq!(
        result,
        ChaseTickResult::MoveTo(NavPoint::new(10.0, 0.0, 0.0))
    );

    // Target moves significantly to (20, 0, 30)
    let new_target = NavPoint::new(20.0, 0.0, 30.0);
    let result = chase.tick(0.1, &mob_pos, &new_target, 5.0);
    assert_eq!(result, ChaseTickResult::NeedRepath, "target moved → repath");

    // Caller provides new path to updated target
    chase.set_path(
        NavPath {
            waypoints: vec![NavPoint::new(10.0, 0.0, 15.0), new_target],
        },
        new_target,
    );

    // Now follows new path toward updated position
    let result = chase.tick(0.1, &mob_pos, &new_target, 5.0);
    assert_eq!(
        result,
        ChaseTickResult::MoveTo(NavPoint::new(10.0, 0.0, 15.0)),
        "follows new path after repath",
    );
}

#[test]
fn chase_stops_when_target_unreachable() {
    let mob_pos = NavPoint::new(0.0, 0.0, 0.0);
    let target_pos = NavPoint::new(50.0, 0.0, 0.0);
    let mut chase = ChaseState::new(1, target_pos, 7.0);

    // No path set → NeedRepath
    let result = chase.tick(0.1, &mob_pos, &target_pos, 5.0);
    assert_eq!(result, ChaseTickResult::NeedRepath);

    // Caller tries pathfinding but fails (target behind wall / LoS broken).
    // Path remains None — next tick still returns NeedRepath.
    let result = chase.tick(0.1, &mob_pos, &target_pos, 5.0);
    assert_eq!(result, ChaseTickResult::NeedRepath, "still no path → stuck");

    // After repeated failures, server should evade. Verify the pattern:
    // consecutive NeedRepath with no path set = unreachable target.
    assert!(chase.path.is_none(), "path never set — target unreachable");
}

// --- Leash tests ---

#[test]
fn leash_triggers_when_too_far() {
    let mut chase = ChaseState::new(1, NavPoint::new(50.0, 0.0, 0.0), 7.0);
    chase.set_path(
        NavPath {
            waypoints: vec![NavPoint::new(45.0, 0.0, 0.0), NavPoint::new(50.0, 0.0, 0.0)],
        },
        NavPoint::new(50.0, 0.0, 0.0),
    );
    let spawn = NavPoint::new(0.0, 0.0, 0.0);
    let current = NavPoint::new(45.0, 0.0, 0.0); // 45y from spawn
    let result = chase.tick_with_leash(
        0.016,
        &current,
        &NavPoint::new(50.0, 0.0, 0.0),
        5.0,
        &spawn,
        40.0,
    );
    assert_eq!(result, ChaseTickResult::Leashed);
}

#[test]
fn leash_does_not_trigger_within_range() {
    let mut chase = ChaseState::new(1, NavPoint::new(30.0, 0.0, 0.0), 7.0);
    chase.set_path(
        NavPath {
            waypoints: vec![NavPoint::new(20.0, 0.0, 0.0), NavPoint::new(30.0, 0.0, 0.0)],
        },
        NavPoint::new(30.0, 0.0, 0.0),
    );
    let spawn = NavPoint::new(0.0, 0.0, 0.0);
    let current = NavPoint::new(20.0, 0.0, 0.0); // 20y from spawn, within 40y leash
    let result = chase.tick_with_leash(
        0.016,
        &current,
        &NavPoint::new(30.0, 0.0, 0.0),
        5.0,
        &spawn,
        40.0,
    );
    assert!(matches!(result, ChaseTickResult::MoveTo(_)));
}

#[test]
fn return_to_spawn_follows_path() {
    let mut ret = ReturnToSpawn::new(7.0);
    ret.set_path(NavPath {
        waypoints: vec![
            NavPoint::new(30.0, 0.0, 0.0),
            NavPoint::new(15.0, 0.0, 0.0),
            NavPoint::new(0.0, 0.0, 0.0),
        ],
    });
    let pos = NavPoint::new(30.0, 0.0, 0.0);
    let next = ret.tick(0.016, &pos);
    assert!(next.is_some());
}

#[test]
fn return_to_spawn_arrives() {
    let mut ret = ReturnToSpawn::new(100.0); // very fast
    ret.set_path(NavPath {
        waypoints: vec![NavPoint::new(1.0, 0.0, 0.0), NavPoint::new(0.0, 0.0, 0.0)],
    });
    let pos = NavPoint::new(0.5, 0.0, 0.0);
    // First tick advances past first waypoint
    ret.tick(0.016, &pos);
    // Second tick from near spawn
    let result = ret.tick(0.016, &NavPoint::new(0.1, 0.0, 0.0));
    assert!(result.is_none()); // arrived
}

#[test]
fn return_to_spawn_no_path() {
    let mut ret = ReturnToSpawn::new(7.0);
    assert!(ret.tick(0.016, &NavPoint::new(10.0, 0.0, 0.0)).is_none());
}

// --- Patrol tests ---

fn sample_patrol() -> PatrolPath {
    PatrolPath {
        waypoints: vec![
            PatrolWaypoint {
                position: NavPoint::new(0.0, 0.0, 0.0),
                delay: 2.0,
            },
            PatrolWaypoint {
                position: NavPoint::new(10.0, 0.0, 0.0),
                delay: 0.0,
            },
            PatrolWaypoint {
                position: NavPoint::new(10.0, 0.0, 10.0),
                delay: 1.0,
            },
        ],
    }
}

#[test]
fn patrol_moves_toward_waypoint() {
    let path = sample_patrol();
    let mut state = PatrolState::new(7.0);
    let pos = NavPoint::new(5.0, 0.0, 0.0);
    let result = state.tick(0.016, &pos, &path);
    assert!(matches!(result, PatrolTickResult::MoveTo(_)));
}

#[test]
fn patrol_waits_at_waypoint() {
    let path = sample_patrol();
    let mut state = PatrolState::new(100.0); // very fast
    let pos = NavPoint::new(0.0, 0.0, 0.0); // at waypoint 0
    let result = state.tick(0.016, &pos, &path);
    // Reached wp0 (delay=2s), should wait
    assert_eq!(result, PatrolTickResult::Waiting);
    assert_eq!(state.index, 1); // advanced to next
}

#[test]
fn patrol_resumes_after_delay() {
    let path = sample_patrol();
    let mut state = PatrolState::new(100.0);
    let pos = NavPoint::new(0.0, 0.0, 0.0);
    state.tick(0.016, &pos, &path); // reach wp0, start 2s delay
    state.tick(2.1, &pos, &path); // delay expired
    let result = state.tick(0.016, &pos, &path);
    // Should now move toward wp1
    assert!(matches!(result, PatrolTickResult::MoveTo(_)));
}

#[test]
fn patrol_no_delay_skips_wait() {
    let path = sample_patrol();
    let mut state = PatrolState::new(100.0);
    state.index = 1; // at wp1 which has delay=0
    let pos = NavPoint::new(10.0, 0.0, 0.0);
    let result = state.tick(0.016, &pos, &path);
    // No delay → immediately MoveTo next
    assert!(matches!(result, PatrolTickResult::MoveTo(_)));
    assert_eq!(state.index, 2);
}

#[test]
fn patrol_loops_back_to_start() {
    let path = sample_patrol();
    let mut state = PatrolState::new(100.0);
    state.index = 2; // at wp2 (last)
    let pos = NavPoint::new(10.0, 0.0, 10.0);
    state.tick(0.016, &pos, &path); // reach wp2, delay=1s
    assert_eq!(state.index, 0); // wrapped to 0
}

#[test]
fn patrol_empty_path_waits() {
    let path = PatrolPath { waypoints: vec![] };
    let mut state = PatrolState::new(7.0);
    let result = state.tick(0.016, &NavPoint::new(0.0, 0.0, 0.0), &path);
    assert_eq!(result, PatrolTickResult::Waiting);
}

// --- Fear / knockback tests ---

#[test]
fn forced_move_target_on_navmesh() {
    let tile = sample_tile(); // 0-10 x 0-10 square
    let origin = NavPoint::new(5.0, 100.0, 5.0);
    let target = forced_move_target(&tile, &origin, 0.0, 3.0); // +X
    assert!((target.x - 8.0).abs() < 0.01);
    assert!(tile.find_polygon(&target).is_some());
}

#[test]
fn forced_move_clamped_to_navmesh() {
    let tile = sample_tile(); // 0-10 x 0-10
    let origin = NavPoint::new(5.0, 100.0, 5.0);
    // Push 50 yards — way outside the 10x10 tile
    let target = forced_move_target(&tile, &origin, 0.0, 50.0);
    // Should be clamped to somewhere within the navmesh
    assert!(tile.find_polygon(&target).is_some());
    assert!(target.x <= 10.0);
}

#[test]
fn knockback_state_expires() {
    let target = NavPoint::new(10.0, 0.0, 0.0);
    let mut state = ForcedMoveState::knockback(target, 0.5, 20.0);
    assert!(!state.tick(0.3));
    assert!(state.tick(0.3)); // expired
}

#[test]
fn fear_state_expires() {
    let target = NavPoint::new(10.0, 0.0, 0.0);
    let mut state = ForcedMoveState::fear(target, 3.0, 10.0);
    assert!(!state.tick(2.0));
    assert!(state.tick(1.5));
}

#[test]
fn forced_move_reached_target() {
    let target = NavPoint::new(10.0, 0.0, 0.0);
    let state = ForcedMoveState::knockback(target, 1.0, 20.0);
    assert!(!state.reached_target(&NavPoint::new(5.0, 0.0, 0.0)));
    assert!(state.reached_target(&NavPoint::new(10.0, 0.0, 0.0)));
}

#[test]
fn forced_move_different_angles() {
    let tile = sample_tile();
    let origin = NavPoint::new(5.0, 100.0, 5.0);

    let north = forced_move_target(&tile, &origin, std::f32::consts::FRAC_PI_2, 3.0);
    assert!(north.z > origin.z); // moved +Z

    let south = forced_move_target(&tile, &origin, -std::f32::consts::FRAC_PI_2, 3.0);
    assert!(south.z < origin.z); // moved -Z
}

// --- Charge / leap tests ---

#[test]
fn charge_arrives_at_target() {
    let mut charge = DirectMoveState::charge(
        NavPoint::new(0.0, 0.0, 0.0),
        NavPoint::new(20.0, 0.0, 0.0),
        40.0, // 40 y/s → 0.5s to travel 20y
    );
    assert!(!charge.tick(0.3));
    assert!(charge.tick(0.3)); // 0.6s > 0.5s
    assert!(charge.is_complete());
}

#[test]
fn charge_interpolation_midway() {
    let charge = DirectMoveState {
        move_type: DirectMoveType::Charge,
        start: NavPoint::new(0.0, 0.0, 0.0),
        destination: NavPoint::new(20.0, 0.0, 0.0),
        speed: 40.0,
        elapsed: 0.25,
        total_time: 0.5,
    };
    let pos = charge.current_position();
    assert!((pos.x - 10.0).abs() < 0.01); // halfway
}

#[test]
fn leap_fixed_travel_time() {
    let mut leap = DirectMoveState::leap(
        NavPoint::new(0.0, 0.0, 0.0),
        NavPoint::new(30.0, 10.0, 0.0),
        0.8, // 0.8s travel
    );
    assert!(!leap.tick(0.5));
    assert!(leap.tick(0.4));
    let pos = leap.current_position();
    assert!((pos.x - 30.0).abs() < 0.01);
    assert!((pos.y - 10.0).abs() < 0.01);
}

#[test]
fn charge_zero_distance_instant() {
    let mut charge = DirectMoveState::charge(
        NavPoint::new(5.0, 0.0, 5.0),
        NavPoint::new(5.0, 0.0, 5.0),
        40.0,
    );
    assert!(charge.tick(0.001)); // instant
}

#[test]
fn direct_move_position_at_completion() {
    let mut charge = DirectMoveState::charge(
        NavPoint::new(0.0, 0.0, 0.0),
        NavPoint::new(10.0, 5.0, 10.0),
        100.0,
    );
    charge.tick(10.0); // way past
    let pos = charge.current_position();
    // Clamped to t=1.0
    assert!((pos.x - 10.0).abs() < 0.01);
    assert!((pos.y - 5.0).abs() < 0.01);
}

#[test]
fn charge_follows_straight_line_ignoring_pathing() {
    // Charge from (0,0,0) to (100,0,50) — a diagonal path that would
    // normally require pathfinding around obstacles. DirectMoveState
    // ignores navmesh entirely: every intermediate position must lie
    // on the exact straight line between start and destination.
    let start = NavPoint::new(0.0, 0.0, 0.0);
    let dest = NavPoint::new(100.0, 0.0, 50.0);
    let mut charge = DirectMoveState::charge(start, dest, 50.0);
    let total = charge.total_time;

    // Sample 10 intermediate positions
    for i in 1..=10 {
        let fraction = i as f32 / 10.0;
        charge.elapsed = total * fraction;
        let pos = charge.current_position();
        let expected_x = 100.0 * fraction;
        let expected_z = 50.0 * fraction;
        assert!(
            (pos.x - expected_x).abs() < 0.01 && (pos.z - expected_z).abs() < 0.01,
            "at {fraction:.1}: ({}, {}) != ({expected_x}, {expected_z})",
            pos.x,
            pos.z,
        );
        // Y stays 0 — no vertical detour
        assert!((pos.y - 0.0).abs() < 0.01);
    }
}
