use super::*;

#[test]
fn return_moves_toward_spawn() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let creature = app
        .world_mut()
        .spawn((
            Position {
                x: 30.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
            },
            CreatureState::Returning,
        ))
        .id();

    let _ = app.world_mut().run_system_once(return_to_spawn_system);

    let pos = *app.world().entity(creature).get::<Position>().unwrap();
    assert!(pos.x < 30.0, "creature should have moved toward spawn");
    assert!(
        pos.x > 0.0,
        "creature should not have reached spawn in one tick"
    );
}

#[test]
fn return_transitions_to_idle_at_spawn() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let creature = app
        .world_mut()
        .spawn((
            Position {
                x: 0.005,
                y: 5.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
            },
            CreatureState::Returning,
        ))
        .id();

    let _ = app.world_mut().run_system_once(return_to_spawn_system);

    let state = app.world().entity(creature).get::<CreatureState>().unwrap();
    assert_eq!(*state, CreatureState::Idle);
    let pos = *app.world().entity(creature).get::<Position>().unwrap();
    assert_eq!(pos.y, 0.0, "y should snap to spawn height on arrival");
}

#[test]
fn return_ignores_idle_creatures() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let creature = app
        .world_mut()
        .spawn((
            Position {
                x: 30.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
            },
            CreatureState::Idle,
        ))
        .id();

    let _ = app.world_mut().run_system_once(return_to_spawn_system);

    let pos = *app.world().entity(creature).get::<Position>().unwrap();
    assert_eq!(pos.x, 30.0, "idle creature should not move");
}

#[test]
fn return_respects_walk_speed() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let slow = app
        .world_mut()
        .spawn((
            Position {
                x: 50.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
            },
            CreatureState::Returning,
        ))
        .id();

    let fast = app
        .world_mut()
        .spawn((
            Position {
                x: 50.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 2.0,
            },
            CreatureState::Returning,
        ))
        .id();

    let _ = app.world_mut().run_system_once(return_to_spawn_system);

    let slow_pos = app.world().entity(slow).get::<Position>().unwrap().x;
    let fast_pos = app.world().entity(fast).get::<Position>().unwrap().x;
    assert!(
        fast_pos < slow_pos,
        "faster creature should be closer to spawn: fast={fast_pos}, slow={slow_pos}"
    );
}
