use super::*;

fn no_ground(_: Vec3) -> GroundProbe {
    GroundProbe::Unloaded
}

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
                speed_run: 1.0,
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
        speed_run: 1.0,
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
            WanderPhase::Idle => tick_idle(&mut state, 0.1, &spawn, pos, &mut no_ground),
            WanderPhase::Walking => pos = tick_walk(pos, &mut state, &spawn, 0.1, &mut no_ground),
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
        speed_run: 1.0,
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
        pos = tick_walk(pos, &mut state, &spawn, 0.05, &mut no_ground);
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
                speed_run: 1.0,
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
        speed_run: 1.0,
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

/// Stormwind Rats (49540) by the Stockade entrance, world.db `content_creature`
/// guid and WoW position, all MovementType 1 with a 5 yd wander_distance.
const STORMWIND_RATS: [(u32, f32, f32, f32); 4] = [
    (20_314_034, -8753.56, 842.195, 97.6346),
    (20_313_784, -8785.96, 837.383, 98.191),
    (20_314_037, -8791.79, 827.465, 98.0077),
    (20_314_025, -8795.37, 834.453, 97.6346),
];

fn stormwind_ground() -> crate::ground::GroundMap {
    crate::ground::GroundMap::new(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        // `world/maps/azeroth/azeroth.wdt`.
        std::collections::HashMap::from([(0, 775_971)]),
    )
}

fn stormwind_rat(world: &mut World, (guid, x, y, z): (u32, f32, f32, f32)) -> Entity {
    let (x, y, z) = wow_to_bevy(x, y, z);
    world
        .spawn((
            Position { x, y, z },
            SpawnPoint {
                x,
                y,
                z,
                wander_distance: 5.0,
                speed_walk: 1.0,
                speed_run: 0.857_143,
            },
            WanderState {
                phase: WanderPhase::Idle,
                target_x: x,
                target_z: z,
                idle_remaining: 1.0,
                seed: guid,
            },
            crate::maps::InMap::continent(0),
        ))
        .id()
}

/// Wandering rats walk on the Stormwind floors (terrain and WMO, the player ground rule)
/// instead of keeping their spawn height: rat 20314034 walked through the wall onto the
/// canal side and hung 11 yd above the water (86.7), rat 20314037 stayed 0.37 yd above
/// the floor it spawned over (TDB z 98.01, WMO floor 97.64).
#[test]
fn stormwind_rats_wander_on_the_floor() {
    let mut app = ai_test_app();
    app.insert_resource(stormwind_ground());
    let rats: Vec<_> = STORMWIND_RATS
        .iter()
        .map(|rat| (rat.0, stormwind_rat(app.world_mut(), *rat)))
        .collect();
    let mut probe = stormwind_ground();
    let mut walked = vec![0.0_f32; rats.len()];
    let mut last: Vec<Position> = rats
        .iter()
        .map(|(_, e)| *app.world().get::<Position>(*e).unwrap())
        .collect();
    // Two minutes at 20 Hz.
    for tick in 0..2400 {
        app.update();
        for (i, (guid, entity)) in rats.iter().enumerate() {
            let pos = *app.world().get::<Position>(*entity).unwrap();
            walked[i] += (pos.x - last[i].x).hypot(pos.z - last[i].z);
            last[i] = pos;
            if walked[i] == 0.0 {
                continue;
            }
            let crate::ground::GroundProbe::Supported(floor) =
                probe.ground_at(0, Vec3::new(pos.x, pos.y, pos.z))
            else {
                panic!("rat {guid} tick {tick}: no ground under {pos:?}");
            };
            assert!(
                (pos.y - floor).abs() < 0.05,
                "rat {guid} tick {tick}: WoW z {} over the floor at {floor}, WoW ({}, {})",
                pos.y,
                pos.x,
                -pos.z
            );
        }
    }
    for (i, (guid, _)) in rats.iter().enumerate() {
        assert!(walked[i] > 5.0, "rat {guid} walked only {} yd", walked[i]);
    }
}
