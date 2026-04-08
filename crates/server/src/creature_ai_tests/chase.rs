use super::*;

#[test]
fn chase_moves_toward_target() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let target_entity = app
        .world_mut()
        .spawn(Position {
            x: 30.0,
            y: 0.0,
            z: 0.0,
        })
        .id();

    let chaser = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
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
            CreatureState::Pursuing {
                target: target_entity.to_bits(),
            },
        ))
        .id();

    let _ = app.world_mut().run_system_once(chase_system);

    let pos = *app.world().entity(chaser).get::<Position>().unwrap();
    assert!(pos.x > 0.0, "creature should have moved toward target");
    assert_eq!(pos.z, 0.0, "no z movement when target is along x axis");
}

#[test]
fn chase_respects_walk_speed() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let target_bits = app
        .world_mut()
        .spawn(Position {
            x: 100.0,
            y: 0.0,
            z: 0.0,
        })
        .id()
        .to_bits();

    let chaser1 = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
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
            CreatureState::Pursuing {
                target: target_bits,
            },
        ))
        .id();

    let chaser2 = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
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
            CreatureState::Pursuing {
                target: target_bits,
            },
        ))
        .id();

    let _ = app.world_mut().run_system_once(chase_system);

    let pos1 = *app.world().entity(chaser1).get::<Position>().unwrap();
    let pos2 = *app.world().entity(chaser2).get::<Position>().unwrap();
    assert!(
        pos2.x > pos1.x,
        "faster creature (speed_walk=2) should move further: {} vs {}",
        pos2.x,
        pos1.x
    );
}

#[test]
fn chase_stops_when_idle() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let _target = app
        .world_mut()
        .spawn(Position {
            x: 30.0,
            y: 0.0,
            z: 0.0,
        })
        .id();

    let creature = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
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

    let _ = app.world_mut().run_system_once(chase_system);

    let pos = *app.world().entity(creature).get::<Position>().unwrap();
    assert_eq!(pos.x, 0.0, "idle creature should not move");
    assert_eq!(pos.z, 0.0, "idle creature should not move");
}

#[test]
fn chase_handles_despawned_target() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let creature = app
        .world_mut()
        .spawn((
            Position {
                x: 5.0,
                y: 0.0,
                z: 5.0,
            },
            SpawnPoint {
                x: 5.0,
                y: 0.0,
                z: 5.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
            },
            CreatureState::Pursuing { target: u64::MAX },
        ))
        .id();

    let _ = app.world_mut().run_system_once(chase_system);

    let pos = *app.world().entity(creature).get::<Position>().unwrap();
    assert_eq!(pos.x, 5.0, "position unchanged for invalid target");
    assert_eq!(pos.z, 5.0, "position unchanged for invalid target");
}
