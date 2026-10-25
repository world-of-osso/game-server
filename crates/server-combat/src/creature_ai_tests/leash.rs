use super::*;
use shared::components::Health;

#[test]
fn beyond_leash_unit_tests() {
    let spawn = SpawnPoint {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        wander_distance: 5.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };

    assert!(!beyond_leash(
        &Position {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        },
        &spawn,
    ));
    assert!(!beyond_leash(
        &Position {
            x: 40.0,
            y: 0.0,
            z: 0.0,
        },
        &spawn,
    ));
    assert!(beyond_leash(
        &Position {
            x: 50.0,
            y: 0.0,
            z: 0.0,
        },
        &spawn,
    ));
    assert!(beyond_leash(
        &Position {
            x: 30.0,
            y: 0.0,
            z: 30.0,
        },
        &spawn,
    ));
    assert!(!beyond_leash(
        &Position {
            x: 0.0,
            y: 999.0,
            z: 0.0,
        },
        &spawn,
    ));

    let spawn2 = SpawnPoint {
        x: 100.0,
        y: 0.0,
        z: 100.0,
        wander_distance: 5.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    };
    assert!(beyond_leash(
        &Position {
            x: 150.0,
            y: 0.0,
            z: 100.0,
        },
        &spawn2,
    ));
    assert!(!beyond_leash(
        &Position {
            x: 110.0,
            y: 0.0,
            z: 100.0,
        },
        &spawn2,
    ));
}

#[test]
fn leash_evade_beyond_distance() {
    use bevy::ecs::system::RunSystemOnce;
    use shared::components::ThreatTable;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let mut threat = ThreatTable::default();
    threat.add_damage_threat(1, 100.0, 1.0);

    let creature = app
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
                speed_run: 1.0,
            },
            CreatureState::Pursuing { target: 1 },
            threat,
            Health {
                current: 50.0,
                max: 100.0,
            },
        ))
        .id();

    let _ = app.world_mut().run_system_once(leash_check_system);

    let state = app.world().entity(creature).get::<CreatureState>().unwrap();
    assert_eq!(*state, CreatureState::Returning);

    let threat = app
        .world()
        .entity(creature)
        .get::<shared::components::ThreatTable>()
        .unwrap();
    assert!(threat.is_empty(), "threat table should be cleared");

    let health = app.world().entity(creature).get::<Health>().unwrap();
    assert_eq!(health.current, 50.0, "evade leaves health to regeneration");
}

/// A real continent map keeps the existing 40yd spawn leash. Dungeon exemptions
/// must not change either the inclusive boundary or the open-world evade path.
#[test]
fn leash_open_world_map_keeps_forty_yard_boundary() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.insert_resource(MapStore::continents());
    app.update();
    for (distance, expected) in [
        (40.0, CreatureState::Pursuing { target: 1 }),
        (40.1, CreatureState::Returning),
    ] {
        let creature = app
            .world_mut()
            .spawn((
                InMap::continent(0),
                Position {
                    x: distance,
                    y: 0.0,
                    z: 0.0,
                },
                SpawnPoint {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    wander_distance: 0.0,
                    speed_walk: 1.0,
                    speed_run: 1.0,
                },
                CreatureState::Pursuing { target: 1 },
            ))
            .id();
        app.world_mut().run_system_once(leash_check_system).unwrap();
        assert_eq!(app.world().get::<CreatureState>(creature), Some(&expected));
        app.world_mut().despawn(creature);
    }
}

#[test]
fn leash_within_range_no_evade() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let creature = app
        .world_mut()
        .spawn((
            Position {
                x: 10.0,
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
            CreatureState::Pursuing { target: 1 },
        ))
        .id();

    let _ = app.world_mut().run_system_once(leash_check_system);

    let state = app.world().entity(creature).get::<CreatureState>().unwrap();
    assert!(matches!(*state, CreatureState::Pursuing { .. }));
}

#[test]
fn leash_at_boundary_no_evade() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let creature = app
        .world_mut()
        .spawn((
            Position {
                x: 40.0,
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
            CreatureState::Pursuing { target: 1 },
        ))
        .id();

    let _ = app.world_mut().run_system_once(leash_check_system);

    let state = app.world().entity(creature).get::<CreatureState>().unwrap();
    assert!(matches!(*state, CreatureState::Pursuing { .. }));
}

#[test]
fn leash_ignores_idle_creatures() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let creature = app
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
                speed_run: 1.0,
            },
            CreatureState::Idle,
        ))
        .id();

    let _ = app.world_mut().run_system_once(leash_check_system);

    let state = app.world().entity(creature).get::<CreatureState>().unwrap();
    assert_eq!(*state, CreatureState::Idle);
}
