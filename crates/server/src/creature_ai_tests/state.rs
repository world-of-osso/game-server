use super::*;

#[test]
fn creature_state_default_is_idle() {
    assert_eq!(CreatureState::default(), CreatureState::Idle);
}

#[test]
fn pursuing_pauses_wander() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let entity = app
        .world_mut()
        .spawn((
            Position {
                x: 50.0,
                y: 0.0,
                z: 80.0,
            },
            SpawnPoint {
                x: 50.0,
                y: 0.0,
                z: 80.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            WanderState {
                phase: WanderPhase::Idle,
                target_x: 0.0,
                target_z: 0.0,
                idle_remaining: 0.0,
                seed: 42,
            },
            CreatureState::Pursuing { target: 1 },
        ))
        .id();

    for _ in 0..10 {
        let _ = app.world_mut().run_system_once(wander_system);
    }

    let pos = *app.world().entity(entity).get::<Position>().unwrap();
    assert_eq!(pos.x, 50.0);
    assert_eq!(pos.z, 80.0);
}

#[test]
fn returning_pauses_wander() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let entity = app
        .world_mut()
        .spawn((
            Position {
                x: 50.0,
                y: 0.0,
                z: 80.0,
            },
            SpawnPoint {
                x: 50.0,
                y: 0.0,
                z: 80.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            WanderState {
                phase: WanderPhase::Idle,
                target_x: 0.0,
                target_z: 0.0,
                idle_remaining: 0.0,
                seed: 42,
            },
            CreatureState::Returning,
        ))
        .id();

    for _ in 0..10 {
        let _ = app.world_mut().run_system_once(wander_system);
    }

    let pos = *app.world().entity(entity).get::<Position>().unwrap();
    assert_eq!(pos.x, 50.0);
    assert_eq!(pos.z, 80.0);
}

#[test]
fn idle_state_allows_wander() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let entity = app
        .world_mut()
        .spawn((
            Position {
                x: 50.0,
                y: 0.0,
                z: 80.0,
            },
            SpawnPoint {
                x: 50.0,
                y: 0.0,
                z: 80.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            WanderState {
                phase: WanderPhase::Idle,
                target_x: 0.0,
                target_z: 0.0,
                idle_remaining: 0.0,
                seed: 42,
            },
            CreatureState::Idle,
        ))
        .id();

    let _ = app.world_mut().run_system_once(wander_system);

    let wander = app.world().entity(entity).get::<WanderState>().unwrap();
    assert_eq!(
        wander.phase,
        WanderPhase::Walking,
        "idle_remaining=0 should have triggered a walk transition"
    );
}

#[test]
fn pursuing_pauses_waypoint() {
    use crate::world_data::{WaypointPath, WaypointPoint};
    use bevy::ecs::system::RunSystemOnce;
    use std::collections::HashMap;

    let make_wp = |x, y, z| WaypointPoint {
        position_x: x,
        position_y: y,
        position_z: z,
        delay: 0,
        ..Default::default()
    };

    let mut waypoints = HashMap::new();
    waypoints.insert(
        1u32,
        WaypointPath {
            points: vec![make_wp(10.0, 0.0, 0.0), make_wp(20.0, 0.0, 0.0)],
        },
    );

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(WorldData {
        templates: HashMap::new().into(),
        spawns: vec![].into(),
        spawns_by_map: HashMap::new(),
        waypoints,
        ..Default::default()
    });
    app.update();
    app.update();

    let entity = app
        .world_mut()
        .spawn((
            Position {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            WaypointState::new(1, 1.0, 1.0, (0.0, 0.0)),
            CreatureState::Pursuing { target: 1 },
        ))
        .id();

    for _ in 0..10 {
        let _ = app.world_mut().run_system_once(waypoint_system);
    }

    let state = app.world().entity(entity).get::<WaypointState>().unwrap();
    assert_eq!(
        state.current_index, 0,
        "waypoint index should not advance while pursuing"
    );
    let pos = *app.world().entity(entity).get::<Position>().unwrap();
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.z, 0.0);
}
