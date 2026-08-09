use super::*;

#[test]
fn patrol_returns_to_nearest_waypoint_after_combat() {
    use crate::world_data::{WaypointPath, WaypointPoint};

    let make_wp = |x, y, z| WaypointPoint {
        position_x: x,
        position_y: y,
        position_z: z,
        delay: 0,
    };

    let path = WaypointPath {
        points: vec![
            make_wp(0.0, 0.0, 0.0),
            make_wp(20.0, 0.0, 0.0),
            make_wp(40.0, 0.0, 0.0),
            make_wp(60.0, 0.0, 0.0),
        ],
    };

    assert_eq!(
        nearest_waypoint(
            &Position {
                x: 35.0,
                y: 0.0,
                z: 0.0,
            },
            &path,
        ),
        2
    );
    assert_eq!(
        nearest_waypoint(
            &Position {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            &path,
        ),
        0
    );
    assert_eq!(
        nearest_waypoint(
            &Position {
                x: 55.0,
                y: 0.0,
                z: 0.0,
            },
            &path,
        ),
        3
    );
    assert_eq!(
        nearest_waypoint(
            &Position {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
            &path,
        ),
        1
    );
}

#[test]
fn waypoint_creature_follows_path_in_order() {
    use crate::world_data::{WaypointPath, WaypointPoint};

    let make_wp = |x, y, z| WaypointPoint {
        position_x: x,
        position_y: y,
        position_z: z,
        delay: 0,
    };

    let path = WaypointPath {
        points: vec![
            make_wp(10.0, 0.0, 0.0),
            make_wp(20.0, 0.0, 0.0),
            make_wp(30.0, 0.0, 0.0),
        ],
    };
    let mut pos = Position {
        x: 10.0,
        y: 0.0,
        z: 0.0,
    };
    let mut state = WaypointState {
        creature_guid: 1,
        current_index: 0,
        delay_remaining: 0.0,
        speed_walk: 1.0,
    };

    pos = tick_waypoint(pos, &mut state, &path, 0.1);
    assert_eq!(state.current_index, 1, "advanced past wp 0");

    for _ in 0..500 {
        if state.current_index != 1 {
            break;
        }
        pos = tick_waypoint(pos, &mut state, &path, 0.1);
    }
    assert_eq!(state.current_index, 2, "advanced past wp 1");
    assert!((pos.x - 20.0).abs() < 0.5, "reached wp 1 at x=20");

    for _ in 0..500 {
        if state.current_index != 2 {
            break;
        }
        pos = tick_waypoint(pos, &mut state, &path, 0.1);
    }
    assert_eq!(state.current_index, 0, "wrapped back to wp 0");
    assert!((pos.x - 30.0).abs() < 0.5, "reached wp 2 at x=30");
}

#[test]
fn waypoint_creature_pauses_at_delay() {
    use crate::world_data::{WaypointPath, WaypointPoint};

    let path = WaypointPath {
        points: vec![
            WaypointPoint {
                position_x: 10.0,
                position_y: 0.0,
                position_z: 0.0,
                delay: 3,
            },
            WaypointPoint {
                position_x: 20.0,
                position_y: 0.0,
                position_z: 0.0,
                delay: 0,
            },
        ],
    };
    let mut pos = Position {
        x: 10.0,
        y: 0.0,
        z: 0.0,
    };
    let mut state = WaypointState {
        creature_guid: 1,
        current_index: 0,
        delay_remaining: 0.0,
        speed_walk: 1.0,
    };

    pos = tick_waypoint(pos, &mut state, &path, 0.5);
    assert_eq!(state.current_index, 1);
    assert!(
        (state.delay_remaining - 3.0).abs() < 0.01,
        "delay set to 3s"
    );

    let pos_before = pos;
    pos = tick_waypoint(pos, &mut state, &path, 0.5);
    assert_eq!(pos.x, pos_before.x, "no movement during delay");
    assert!(state.delay_remaining > 0.0, "still delaying");

    for _ in 0..10 {
        pos = tick_waypoint(pos, &mut state, &path, 0.5);
    }
    assert!(state.delay_remaining <= 0.0, "delay expired");

    let pos_after_delay = pos.x;
    pos = tick_waypoint(pos, &mut state, &path, 0.5);
    assert!(pos.x > pos_after_delay, "moving toward wp 1 after delay");
}

#[test]
fn waypoint_at_current_position_does_not_dirty_position() {
    use crate::world_data::{WaypointPath, WaypointPoint};
    use bevy::ecs::system::RunSystemOnce;
    use std::collections::HashMap;

    let path = WaypointPath {
        points: vec![WaypointPoint {
            position_x: 0.0,
            position_y: 0.0,
            position_z: 0.0,
            delay: 0,
        }],
    };
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(WorldData {
        templates: HashMap::new(),
        spawns: vec![],
        spawns_by_map: HashMap::new(),
        waypoints: HashMap::from([(7, path)]),
    });
    app.update();
    app.update();

    let position = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let entity = app
        .world_mut()
        .spawn((
            position,
            WaypointState {
                creature_guid: 7,
                current_index: 0,
                delay_remaining: 0.0,
                speed_walk: 1.0,
            },
        ))
        .id();
    app.update();

    let tick_before = app
        .world()
        .entity(entity)
        .get_ref::<Position>()
        .unwrap()
        .last_changed();

    app.world_mut().run_system_once(waypoint_system).unwrap();

    assert_eq!(
        *app.world().entity(entity).get::<Position>().unwrap(),
        position
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get_ref::<Position>()
            .unwrap()
            .last_changed(),
        tick_before,
        "an unchanged waypoint target and height must not dirty Position"
    );
}

#[test]
fn advance_waypoint_wraps() {
    let mut state = WaypointState {
        creature_guid: 1,
        current_index: 3,
        delay_remaining: 0.0,
        speed_walk: 1.0,
    };
    advance_waypoint(&mut state, 2.0, 4);
    assert_eq!(state.current_index, 0);
    assert_eq!(state.delay_remaining, 2.0);
}
