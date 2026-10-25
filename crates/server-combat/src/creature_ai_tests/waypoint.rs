use super::*;

#[test]
fn patrol_returns_to_nearest_waypoint_after_combat() {
    use crate::world_data::{WaypointPath, WaypointPoint};

    let make_wp = |x, y, z| WaypointPoint {
        position_x: x,
        position_y: y,
        position_z: z,
        delay: 0,
        ..Default::default()
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
        ..Default::default()
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
    let mut state = WaypointState::new(1, 1.0, 1.0, (10.0, 0.0));
    let mut reached = Vec::new();
    for _ in 0..2000 {
        let was_arrived = state.arrived;
        pos = tick_waypoint(pos, &mut state, &path, 0.1).0;
        if state.arrived && !was_arrived {
            reached.push((state.current_index, pos.x));
        }
        if reached.len() == 4 {
            break;
        }
    }
    assert_eq!(
        reached,
        vec![(0, 10.0), (1, 20.0), (2, 30.0), (0, 10.0)],
        "points reached in order, then the path wraps"
    );
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
                delay: 3000,
                ..Default::default()
            },
            WaypointPoint {
                position_x: 20.0,
                position_y: 0.0,
                position_z: 0.0,
                delay: 0,
                ..Default::default()
            },
        ],
    };
    let mut pos = Position {
        x: 10.0,
        y: 0.0,
        z: 0.0,
    };
    let mut state = WaypointState::new(1, 1.0, 1.0, (10.0, 0.0));

    pos = tick_waypoint(pos, &mut state, &path, 0.5).0;
    assert!(state.arrived);
    assert_eq!(state.current_index, 0);
    assert!(
        (state.delay_remaining - 3.0).abs() < 0.01,
        "waypoint_data.delay is milliseconds: 3000 ms pauses 3 s"
    );

    for _ in 0..5 {
        let before = pos;
        pos = tick_waypoint(pos, &mut state, &path, 0.5).0;
        assert_eq!(pos, before, "no movement during delay");
    }
    assert_eq!(state.current_index, 0, "still paused after 2.5 s");

    pos = tick_waypoint(pos, &mut state, &path, 0.5).0;
    assert_eq!(
        state.current_index, 1,
        "next leg starts when the 3 s delay ends"
    );
    assert!(pos.x > 10.0, "moving toward wp 1 after delay");
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
            ..Default::default()
        }],
    };
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(WorldData {
        templates: HashMap::new().into(),
        spawns: vec![].into(),
        spawns_by_map: HashMap::new(),
        waypoints: HashMap::from([(7, path)]),
        ..Default::default()
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
        .spawn((position, WaypointState::new(7, 1.0, 1.0, (0.0, 0.0))))
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
fn next_leg_wraps_and_starts_from_the_reached_point() {
    let mut state = WaypointState {
        current_index: 3,
        arrived: true,
        ..WaypointState::new(1, 1.0, 1.0, (0.0, 0.0))
    };
    let at = Position {
        x: 5.0,
        y: 0.0,
        z: -2.0,
    };
    start_next_leg(&mut state, &at, 4);
    assert_eq!(state.current_index, 0);
    assert!(!state.arrived);
    assert_eq!(state.leg_origin, (5.0, -2.0));
}
