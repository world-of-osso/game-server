use super::*;

#[test]
fn idle_creature_stays_at_spawn_point() {
    use bevy::ecs::system::RunSystemOnce;
    use shared::components::Npc;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let entity = app
        .world_mut()
        .spawn((
            Position {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            Npc {
                template_id: 1,
                name: "Fixture wolf".into(),
            },
        ))
        .id();

    let _ = app.world_mut().run_system_once(wander_system);
    let _ = app.world_mut().run_system_once(waypoint_system);

    let pos = *app.world().entity(entity).get::<Position>().unwrap();
    assert_eq!(pos.x, 10.0);
    assert_eq!(pos.y, 20.0);
    assert_eq!(pos.z, 30.0);
}

#[test]
fn wander_creature_moves_within_radius() {
    use bevy::ecs::system::RunSystemOnce;

    let spawn_x = 50.0;
    let spawn_z = 80.0;
    let radius = 5.0;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let entity = app
        .world_mut()
        .spawn((
            Position {
                x: spawn_x,
                y: 0.0,
                z: spawn_z,
            },
            SpawnPoint {
                x: spawn_x,
                y: 0.0,
                z: spawn_z,
                wander_distance: radius,
                speed_walk: 1.0,
            },
            WanderState {
                phase: WanderPhase::Idle,
                target_x: 0.0,
                target_z: 0.0,
                idle_remaining: 0.0,
                seed: 42,
            },
        ))
        .id();

    for _ in 0..500 {
        let _ = app.world_mut().run_system_once(wander_system);
    }

    let pos = *app.world().entity(entity).get::<Position>().unwrap();
    let dx = pos.x - spawn_x;
    let dz = pos.z - spawn_z;
    let dist = (dx * dx + dz * dz).sqrt();
    assert!(
        dist <= radius + 0.1,
        "creature at ({}, {}) is {dist:.2} from spawn, exceeds radius {radius}",
        pos.x,
        pos.z,
    );
}

#[test]
fn wander_zero_radius_stays_at_spawn() {
    let spawn_x = 50.0;
    let spawn_z = 80.0;
    let spawn = SpawnPoint {
        x: spawn_x,
        y: 0.0,
        z: spawn_z,
        wander_distance: 0.0,
        speed_walk: 1.0,
    };
    let mut pos = Position {
        x: spawn_x,
        y: 0.0,
        z: spawn_z,
    };
    let mut state = WanderState {
        phase: WanderPhase::Idle,
        target_x: 0.0,
        target_z: 0.0,
        idle_remaining: 0.0,
        seed: 42,
    };

    for _ in 0..200 {
        match state.phase {
            WanderPhase::Idle => tick_idle(&mut state, 0.1, &spawn),
            WanderPhase::Walking => pos = tick_walk(pos, &mut state, &spawn, 0.1),
        }
    }

    assert!(
        (pos.x - spawn_x).abs() < 0.01 && (pos.z - spawn_z).abs() < 0.01,
        "creature at ({}, {}), expected ({spawn_x}, {spawn_z})",
        pos.x,
        pos.z,
    );
}

#[test]
fn wander_creature_returns_to_idle_after_walk() {
    let spawn_x = 50.0;
    let spawn_z = 80.0;
    let radius = 5.0;
    let spawn = SpawnPoint {
        x: spawn_x,
        y: 0.0,
        z: spawn_z,
        wander_distance: radius,
        speed_walk: 1.0,
    };
    let mut pos = Position {
        x: spawn_x,
        y: 0.0,
        z: spawn_z,
    };
    let mut state = WanderState {
        phase: WanderPhase::Walking,
        target_x: spawn_x + 3.0,
        target_z: spawn_z + 2.0,
        idle_remaining: 0.0,
        seed: 42,
    };

    for _ in 0..500 {
        pos = tick_walk(pos, &mut state, &spawn, 0.05);
        if state.phase == WanderPhase::Idle {
            break;
        }
    }

    assert_eq!(state.phase, WanderPhase::Idle, "returned to idle");
    assert!(state.idle_remaining > 0.0, "idle timer set");

    let dx = pos.x - spawn_x;
    let dz = pos.z - spawn_z;
    let dist = (dx * dx + dz * dz).sqrt();
    assert!(
        dist <= radius + 0.1,
        "creature at ({}, {}) is {dist:.2} from spawn, within radius {radius}",
        pos.x,
        pos.z,
    );
}

#[test]
fn wander_at_target_does_not_dirty_position() {
    use bevy::ecs::system::RunSystemOnce;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();

    let position = Position {
        x: 50.0,
        y: 0.0,
        z: 80.0,
    };
    let entity = app
        .world_mut()
        .spawn((
            position,
            SpawnPoint {
                x: position.x,
                y: position.y,
                z: position.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
            },
            WanderState {
                phase: WanderPhase::Walking,
                target_x: position.x,
                target_z: position.z,
                idle_remaining: 0.0,
                seed: 42,
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

    app.world_mut().run_system_once(wander_system).unwrap();

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
        "reaching an unchanged wander target must not dirty Position"
    );
    assert_eq!(
        app.world()
            .entity(entity)
            .get::<WanderState>()
            .unwrap()
            .phase,
        WanderPhase::Idle,
        "arrival must still advance the wander state"
    );
}

#[test]
fn wander_targets_respect_boundary_across_spawns() {
    let cases = [
        (0.0, 0.0, 3.0),
        (100.0, -200.0, 10.0),
        (-50.0, 50.0, 1.0),
        (999.0, 999.0, 20.0),
    ];
    for (cx, cz, radius) in cases {
        for seed in 0..200 {
            let (px, pz) = pick_wander_point(cx, cz, radius, seed);
            let dx = px - cx;
            let dz = pz - cz;
            let dist = (dx * dx + dz * dz).sqrt();
            assert!(
                dist <= radius + 0.01,
                "center=({cx},{cz}) r={radius} seed={seed}: dist {dist:.3}",
            );
        }
    }
}

#[test]
fn dead_wanderer_does_not_move() {
    let mut app = ai_test_app();
    let spawn = SpawnPoint {
        x: 50.0,
        y: 0.0,
        z: 80.0,
        wander_distance: 5.0,
        speed_walk: 1.0,
    };
    let corpse = Position {
        x: 52.0,
        y: 0.0,
        z: 81.0,
    };
    let entity = app
        .world_mut()
        .spawn((
            corpse,
            spawn,
            WanderState {
                phase: WanderPhase::Walking,
                target_x: 55.0,
                target_z: 80.0,
                idle_remaining: 0.0,
                seed: 1,
            },
            crate::combat::DeathState {
                respawn_timer: 60.0,
            },
        ))
        .id();
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(*app.world().get::<Position>(entity).unwrap(), corpse);
}
