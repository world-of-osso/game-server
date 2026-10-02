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
                speed_run: 1.0,
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
fn chase_at_target_does_not_dirty_position() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let target_position = Position {
        x: 10.0,
        y: 0.0,
        z: -4.0,
    };
    let target = app.world_mut().spawn(target_position).id();
    let chaser = app
        .world_mut()
        .spawn((
            target_position,
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureState::Pursuing {
                target: target.to_bits(),
            },
        ))
        .id();

    app.update();

    let position_before = *app.world().entity(chaser).get::<Position>().unwrap();
    let tick_before = app
        .world()
        .entity(chaser)
        .get_ref::<Position>()
        .unwrap()
        .last_changed();

    app.world_mut().run_system_once(chase_system).unwrap();

    let position_after = *app.world().entity(chaser).get::<Position>().unwrap();
    let tick_after = app
        .world()
        .entity(chaser)
        .get_ref::<Position>()
        .unwrap()
        .last_changed();
    assert_eq!(position_after, position_before);
    assert_eq!(tick_after, tick_before);
}

#[test]
fn chase_respects_run_speed() {
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
                speed_run: 1.0,
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
                speed_walk: 1.0,
                speed_run: 2.0,
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
        "faster creature (speed_run=2) should move further: {} vs {}",
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
                speed_run: 1.0,
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
                speed_run: 1.0,
            },
            CreatureState::Pursuing { target: u64::MAX },
        ))
        .id();

    let _ = app.world_mut().run_system_once(chase_system);

    let pos = *app.world().entity(creature).get::<Position>().unwrap();
    assert_eq!(pos.x, 5.0, "position unchanged for invalid target");
    assert_eq!(pos.z, 5.0, "position unchanged for invalid target");
}

/// A creature fighting another creature runs to it: the victim's own `SpawnPoint`
/// does not hide it from the chase (TrinityCore `ChaseMovementGenerator` follows any unit).
#[test]
fn chase_runs_toward_a_creature_victim() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let spawn_at = |x: f32| SpawnPoint {
        x,
        y: 0.0,
        z: 0.0,
        wander_distance: 0.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    let worg = app
        .world_mut()
        .spawn((
            Position {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
            spawn_at(20.0),
            CreatureState::Idle,
        ))
        .id();
    let soldier = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            spawn_at(0.0),
            CreatureState::Pursuing {
                target: worg.to_bits(),
            },
        ))
        .id();

    app.world_mut().run_system_once(chase_system).unwrap();

    let soldier_x = app.world().get::<Position>(soldier).unwrap().x;
    assert!(
        soldier_x > 0.0,
        "the soldier runs toward the worg, got x {soldier_x}"
    );
    assert_eq!(app.world().get::<Position>(worg).unwrap().x, 20.0);
}
