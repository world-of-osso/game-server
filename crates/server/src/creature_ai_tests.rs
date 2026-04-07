    use super::*;

    #[test]
    fn pick_point_within_radius() {
        for seed in 0..100 {
            let (px, py) = pick_wander_point(10.0, 20.0, 5.0, seed);
            let dx = px - 10.0;
            let dy = py - 20.0;
            let dist = (dx * dx + dy * dy).sqrt();
            assert!(dist <= 5.01, "seed {seed}: dist {dist} exceeds radius 5.0");
        }
    }

    #[test]
    fn pick_point_zero_radius_stays_at_center() {
        let (px, py) = pick_wander_point(5.0, 5.0, 0.0, 42);
        assert!((px - 5.0).abs() < 0.001);
        assert!((py - 5.0).abs() < 0.001);
    }

    #[test]
    fn move_toward_reaches_close_target() {
        let (reached, x, y) = move_toward(0.0, 0.0, 0.5, 0.0, 1.0);
        assert!(reached);
        assert!((x - 0.5).abs() < 0.001);
        assert!(y.abs() < 0.001);
    }

    #[test]
    fn move_toward_partial_step() {
        let (reached, x, y) = move_toward(0.0, 0.0, 10.0, 0.0, 3.0);
        assert!(!reached);
        assert!((x - 3.0).abs() < 0.001);
        assert!(y.abs() < 0.001);
    }

    #[test]
    fn move_toward_diagonal() {
        let (reached, x, y) = move_toward(0.0, 0.0, 3.0, 4.0, 2.5);
        assert!(!reached);
        // Should move 2.5 units along the (3,4) direction (length 5)
        assert!((x - 1.5).abs() < 0.001);
        assert!((y - 2.0).abs() < 0.001);
    }

    #[test]
    fn lcg_produces_values_in_range() {
        for seed in 0..1000 {
            let v = lcg_float(seed);
            assert!(
                (0.0..1.0).contains(&v),
                "seed {seed}: value {v} out of range"
            );
        }
    }

    #[test]
    fn random_idle_in_range() {
        for seed in 0..100 {
            let t = random_idle_secs(seed);
            assert!(
                (IDLE_MIN_SECS..=IDLE_MAX_SECS).contains(&t),
                "seed {seed}: idle {t}"
            );
        }
    }

    #[test]
    fn idle_creature_stays_at_spawn_point() {
        use bevy::ecs::system::RunSystemOnce;
        use shared::components::Npc;

        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.update();
        app.update();

        // Spawn creature with no WanderState/WaypointState (movement_type=0)
        let entity = app
            .world_mut()
            .spawn((
                Position {
                    x: 10.0,
                    y: 20.0,
                    z: 30.0,
                },
                Npc { template_id: 1 },
            ))
            .id();

        // Run wander and waypoint systems — creature should be unaffected
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
                    idle_remaining: 0.0, // triggers walk immediately
                    seed: 42,
                },
            ))
            .id();

        // Run many ticks to let the creature wander and return multiple times
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
        let dt = 0.1;

        // Run many ticks — idle expires, picks target at center, walks there (0 dist)
        for _ in 0..200 {
            match state.phase {
                WanderPhase::Idle => tick_idle(&mut state, dt, &spawn),
                WanderPhase::Walking => tick_walk(&mut pos, &mut state, &spawn, dt),
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

        // Start walking toward a target within radius
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

        // Simulate ticks at fixed dt until creature reaches target
        let dt = 0.05;
        for _ in 0..500 {
            tick_walk(&mut pos, &mut state, &spawn, dt);
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
    fn wander_targets_respect_boundary_across_spawns() {
        // Verify all wander targets stay within radius for varied centers and radii
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

    /// Find the nearest waypoint index to resume patrol after combat.
    fn nearest_waypoint(pos: &Position, path: &crate::world_data::WaypointPath) -> usize {
        let mut best_idx = 0;
        let mut best_dist = f32::MAX;
        for (i, wp) in path.points.iter().enumerate() {
            let (bx, _by, bz) =
                crate::terrain_height::wow_to_bevy(wp.position_x, wp.position_y, wp.position_z);
            let dx = pos.x - bx;
            let dz = pos.z - bz;
            let dist_sq = dx * dx + dz * dz;
            if dist_sq < best_dist {
                best_dist = dist_sq;
                best_idx = i;
            }
        }
        best_idx
    }

    #[test]
    fn patrol_returns_to_nearest_waypoint_after_combat() {
        use crate::world_data::{WaypointPath, WaypointPoint};

        let make_wp = |x, y, z| WaypointPoint {
            point: 0,
            position_x: x,
            position_y: y,
            position_z: z,
            orientation: None,
            delay: 0,
            move_type: 0,
        };

        let path = WaypointPath {
            creature_guid: 1,
            points: vec![
                make_wp(0.0, 0.0, 0.0),  // Bevy: (0, 0, 0)
                make_wp(20.0, 0.0, 0.0), // Bevy: (20, 0, 0)
                make_wp(40.0, 0.0, 0.0), // Bevy: (40, 0, 0)
                make_wp(60.0, 0.0, 0.0), // Bevy: (60, 0, 0)
            ],
        };

        // Creature chased a player to (35, 0, 0) — nearest wp is #2 (40, 0, 0)
        let pos = Position {
            x: 35.0,
            y: 0.0,
            z: 0.0,
        };
        let idx = nearest_waypoint(&pos, &path);
        assert_eq!(idx, 2, "nearest to x=35 is wp 2 at x=40");

        // Near the start
        let pos = Position {
            x: 3.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(nearest_waypoint(&pos, &path), 0);

        // Near the end
        let pos = Position {
            x: 55.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(nearest_waypoint(&pos, &path), 3);

        // Exactly on a waypoint
        let pos = Position {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(nearest_waypoint(&pos, &path), 1);
    }

    #[test]
    fn waypoint_creature_follows_path_in_order() {
        use crate::world_data::{WaypointPath, WaypointPoint};

        let make_wp = |x, y, z| WaypointPoint {
            point: 0,
            position_x: x,
            position_y: y,
            position_z: z,
            orientation: None,
            delay: 0,
            move_type: 0,
        };

        let path = WaypointPath {
            creature_guid: 1,
            points: vec![
                make_wp(10.0, 0.0, 0.0), // Bevy: (10, 0, 0)
                make_wp(20.0, 0.0, 0.0), // Bevy: (20, 0, 0)
                make_wp(30.0, 0.0, 0.0), // Bevy: (30, 0, 0)
            ],
        };

        // Start at first waypoint in Bevy coords
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
        let dt = 0.1;

        // Walk to waypoint 0 (already there) → advances to 1
        tick_waypoint(&mut pos, &mut state, &path, dt);
        assert_eq!(state.current_index, 1, "advanced past wp 0");

        // Walk toward waypoint 1 (x=20)
        for _ in 0..500 {
            if state.current_index != 1 {
                break;
            }
            tick_waypoint(&mut pos, &mut state, &path, dt);
        }
        assert_eq!(state.current_index, 2, "advanced past wp 1");
        assert!((pos.x - 20.0).abs() < 0.5, "reached wp 1 at x=20");

        // Walk toward waypoint 2 (x=30)
        for _ in 0..500 {
            if state.current_index != 2 {
                break;
            }
            tick_waypoint(&mut pos, &mut state, &path, dt);
        }
        assert_eq!(state.current_index, 0, "wrapped back to wp 0");
        assert!((pos.x - 30.0).abs() < 0.5, "reached wp 2 at x=30");
    }

    #[test]
    fn waypoint_creature_pauses_at_delay() {
        use crate::world_data::{WaypointPath, WaypointPoint};

        let path = WaypointPath {
            creature_guid: 1,
            points: vec![
                WaypointPoint {
                    point: 0,
                    position_x: 10.0,
                    position_y: 0.0,
                    position_z: 0.0,
                    orientation: None,
                    delay: 3, // 3 second pause
                    move_type: 0,
                },
                WaypointPoint {
                    point: 1,
                    position_x: 20.0,
                    position_y: 0.0,
                    position_z: 0.0,
                    orientation: None,
                    delay: 0,
                    move_type: 0,
                },
            ],
        };

        // Start at wp 0, which will be immediately reached
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
        let dt = 0.5;

        // First tick: reaches wp 0, advances to wp 1 with 3s delay
        tick_waypoint(&mut pos, &mut state, &path, dt);
        assert_eq!(state.current_index, 1);
        assert!(
            (state.delay_remaining - 3.0).abs() < 0.01,
            "delay set to 3s"
        );

        // Tick during delay: creature doesn't move
        let pos_before = pos;
        tick_waypoint(&mut pos, &mut state, &path, dt);
        assert_eq!(pos.x, pos_before.x, "no movement during delay");
        assert!(state.delay_remaining > 0.0, "still delaying");

        // Burn through remaining delay
        for _ in 0..10 {
            tick_waypoint(&mut pos, &mut state, &path, dt);
        }
        assert!(state.delay_remaining <= 0.0, "delay expired");

        // Now creature should be moving toward wp 1 (x=20)
        let pos_after_delay = pos.x;
        tick_waypoint(&mut pos, &mut state, &path, dt);
        assert!(pos.x > pos_after_delay, "moving toward wp 1 after delay");
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
        assert_eq!(state.current_index, 0); // wraps from 3 -> 0
        assert_eq!(state.delay_remaining, 2.0);
    }

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
            point: 0,
            position_x: x,
            position_y: y,
            position_z: z,
            orientation: None,
            delay: 0,
            move_type: 0,
        };

        let mut waypoints = HashMap::new();
        waypoints.insert(
            1u32,
            WaypointPath {
                creature_guid: 1,
                points: vec![make_wp(10.0, 0.0, 0.0), make_wp(20.0, 0.0, 0.0)],
            },
        );
        let world_data = WorldData {
            templates: HashMap::new(),
            spawns: vec![],
            spawns_by_map: HashMap::new(),
            waypoints,
        };

        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.insert_resource(world_data);
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
                WaypointState {
                    creature_guid: 1,
                    current_index: 0,
                    delay_remaining: 0.0,
                    speed_walk: 1.0,
                },
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

    #[test]
    fn chase_moves_toward_target() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.update();
        app.update();

        let target_entity = app
            .world_mut()
            .spawn(Position { x: 30.0, y: 0.0, z: 0.0 })
            .id();

        let chaser = app
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: target_entity.to_bits() },
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

        let target_entity = app
            .world_mut()
            .spawn(Position { x: 100.0, y: 0.0, z: 0.0 })
            .id();
        let target_bits = target_entity.to_bits();

        let chaser1 = app
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: target_bits },
            ))
            .id();

        let chaser2 = app
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 2.0 },
                CreatureState::Pursuing { target: target_bits },
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
            .spawn(Position { x: 30.0, y: 0.0, z: 0.0 })
            .id();

        let creature = app
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
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
                Position { x: 5.0, y: 0.0, z: 5.0 },
                SpawnPoint { x: 5.0, y: 0.0, z: 5.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: u64::MAX },
            ))
            .id();

        // Should not panic
        let _ = app.world_mut().run_system_once(chase_system);

        let pos = *app.world().entity(creature).get::<Position>().unwrap();
        assert_eq!(pos.x, 5.0, "position unchanged for invalid target");
        assert_eq!(pos.z, 5.0, "position unchanged for invalid target");
    }

    // -- Leash tests --

    #[test]
    fn beyond_leash_unit_tests() {
        let spawn = SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 };

        // Clearly inside
        assert!(!beyond_leash(&Position { x: 10.0, y: 0.0, z: 0.0 }, &spawn));
        // Exactly at boundary (40^2 == 40^2, not strictly greater)
        assert!(!beyond_leash(&Position { x: 40.0, y: 0.0, z: 0.0 }, &spawn));
        // Clearly outside
        assert!(beyond_leash(&Position { x: 50.0, y: 0.0, z: 0.0 }, &spawn));
        // Diagonal outside (30^2 + 30^2 = 1800 > 1600)
        assert!(beyond_leash(&Position { x: 30.0, y: 0.0, z: 30.0 }, &spawn));
        // Y-axis ignored (height should not affect result)
        assert!(!beyond_leash(&Position { x: 0.0, y: 999.0, z: 0.0 }, &spawn));
        // Non-zero spawn origin
        let spawn2 = SpawnPoint { x: 100.0, y: 0.0, z: 100.0, wander_distance: 5.0, speed_walk: 1.0 };
        assert!(beyond_leash(&Position { x: 150.0, y: 0.0, z: 100.0 }, &spawn2));
        assert!(!beyond_leash(&Position { x: 110.0, y: 0.0, z: 100.0 }, &spawn2));
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
                Position { x: 50.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: 1 },
                threat,
                Health { current: 50.0, max: 100.0 },
            ))
            .id();

        let _ = app.world_mut().run_system_once(leash_check_system);

        let state = app.world().entity(creature).get::<CreatureState>().unwrap();
        assert_eq!(*state, CreatureState::Returning, "should transition to Returning");

        let threat = app.world().entity(creature).get::<ThreatTable>().unwrap();
        assert!(threat.is_empty(), "threat table should be cleared");

        let health = app.world().entity(creature).get::<Health>().unwrap();
        assert_eq!(health.current, health.max, "health should be fully restored");
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
                Position { x: 10.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: 1 },
            ))
            .id();

        let _ = app.world_mut().run_system_once(leash_check_system);

        let state = app.world().entity(creature).get::<CreatureState>().unwrap();
        assert!(
            matches!(*state, CreatureState::Pursuing { .. }),
            "should remain Pursuing within leash range"
        );
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
                Position { x: 40.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: 1 },
            ))
            .id();

        let _ = app.world_mut().run_system_once(leash_check_system);

        let state = app.world().entity(creature).get::<CreatureState>().unwrap();
        assert!(
            matches!(*state, CreatureState::Pursuing { .. }),
            "exactly at leash boundary should not evade (strict greater-than)"
        );
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
                Position { x: 50.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Idle,
            ))
            .id();

        let _ = app.world_mut().run_system_once(leash_check_system);

        let state = app.world().entity(creature).get::<CreatureState>().unwrap();
        assert_eq!(*state, CreatureState::Idle, "idle creature should not be affected by leash");
    }

    // -- Return-to-spawn tests --

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
                Position { x: 30.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Returning,
            ))
            .id();

        let _ = app.world_mut().run_system_once(return_to_spawn_system);

        let pos = *app.world().entity(creature).get::<Position>().unwrap();
        assert!(pos.x < 30.0, "creature should have moved toward spawn");
        assert!(pos.x > 0.0, "creature should not have reached spawn in one tick");
    }

    #[test]
    fn return_transitions_to_idle_at_spawn() {
        use bevy::ecs::system::RunSystemOnce;

        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.update();
        app.update();

        // Place creature very close to spawn so it reaches in one tick
        let creature = app
            .world_mut()
            .spawn((
                Position { x: 0.005, y: 5.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Returning,
            ))
            .id();

        let _ = app.world_mut().run_system_once(return_to_spawn_system);

        let state = app.world().entity(creature).get::<CreatureState>().unwrap();
        assert_eq!(*state, CreatureState::Idle, "should be idle after reaching spawn");
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
                Position { x: 30.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
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
                Position { x: 50.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Returning,
            ))
            .id();

        let fast = app
            .world_mut()
            .spawn((
                Position { x: 50.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 2.0 },
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

    // -- Integration tests: full chase → leash → return pipeline --

    /// Build an app with creature AI systems registered and time advancing.
    /// Uses FixedUpdate at 20Hz (0.05s timestep) like the real server.
    fn ai_test_app() -> App {
        use bevy::time::TimeUpdateStrategy;
        use std::collections::HashMap;

        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(50),
        ));
        // waypoint_system requires WorldData resource
        app.insert_resource(WorldData {
            templates: HashMap::new(),
            spawns: vec![],
            spawns_by_map: HashMap::new(),
            waypoints: HashMap::new(),
        });
        app.add_systems(
            FixedUpdate,
            (
                wander_system,
                waypoint_system,
                (chase_system, leash_check_system, return_to_spawn_system).chain(),
            ),
        );
        // Prime the app so Time resource is initialized
        app.update();
        app
    }

    #[test]
    fn chase_reaches_target_over_multiple_ticks() {
        let mut app = ai_test_app();

        let target = app
            .world_mut()
            .spawn(Position { x: 10.0, y: 0.0, z: 0.0 })
            .id();

        let chaser = app
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: target.to_bits() },
            ))
            .id();

        // speed = 2.5 units/s, dt = 0.05s → 0.125 units/tick, need ~80 ticks for 10 units
        for _ in 0..200 {
            app.update();
            let pos = *app.world().entity(chaser).get::<Position>().unwrap();
            if (pos.x - 10.0).abs() < 0.2 {
                break;
            }
        }

        let pos = *app.world().entity(chaser).get::<Position>().unwrap();
        assert!(
            (pos.x - 10.0).abs() < 0.2,
            "creature should have reached target at x=10, got x={}",
            pos.x
        );
    }

    #[test]
    fn chase_then_leash_evade() {
        let mut app = ai_test_app();

        // Target 60 units away — creature will chase past 40yd leash
        let target = app
            .world_mut()
            .spawn(Position { x: 60.0, y: 0.0, z: 0.0 })
            .id();

        let mut threat = ThreatTable::default();
        threat.add_damage_threat(target.to_bits(), 100.0, 1.0);

        let chaser = app
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: target.to_bits() },
                threat,
                Health { current: 50.0, max: 100.0 },
            ))
            .id();

        // Run until evade triggers
        for _ in 0..1000 {
            app.update();
            let state = app.world().entity(chaser).get::<CreatureState>().unwrap();
            if *state == CreatureState::Returning {
                break;
            }
        }

        let state = app.world().entity(chaser).get::<CreatureState>().unwrap();
        assert_eq!(*state, CreatureState::Returning, "should have evaded");

        let threat = app.world().entity(chaser).get::<ThreatTable>().unwrap();
        assert!(threat.is_empty(), "threat should be cleared on evade");

        let health = app.world().entity(chaser).get::<Health>().unwrap();
        assert_eq!(health.current, 100.0, "health should be restored on evade");
    }

    #[test]
    fn full_pipeline_pursue_evade_return_idle() {
        let mut app = ai_test_app();

        // Target far away — creature will chase, exceed leash, evade, walk back
        let target = app
            .world_mut()
            .spawn(Position { x: 80.0, y: 0.0, z: 0.0 })
            .id();

        let chaser = app
            .world_mut()
            .spawn((
                Position { x: 0.0, y: 0.0, z: 0.0 },
                SpawnPoint { x: 0.0, y: 0.0, z: 0.0, wander_distance: 5.0, speed_walk: 1.0 },
                CreatureState::Pursuing { target: target.to_bits() },
                Health { current: 60.0, max: 100.0 },
            ))
            .id();

        // Phase 1: Chase until leash evade
        let mut evaded = false;
        for _ in 0..1000 {
            app.update();
            if *app.world().entity(chaser).get::<CreatureState>().unwrap()
                == CreatureState::Returning
            {
                evaded = true;
                break;
            }
        }
        assert!(evaded, "creature should have evaded after exceeding leash");

        // return_to_spawn runs in same frame after leash evade, so creature
        // has already taken one step back from beyond-40 toward spawn
        let evade_x = app.world().entity(chaser).get::<Position>().unwrap().x;
        assert!(evade_x > 39.0, "evade pos should be near leash boundary: x={evade_x}");

        // Phase 2: Return to spawn until idle
        let mut returned = false;
        for _ in 0..2000 {
            app.update();
            if *app.world().entity(chaser).get::<CreatureState>().unwrap() == CreatureState::Idle {
                returned = true;
                break;
            }
        }
        assert!(returned, "creature should have returned to idle at spawn");

        let final_pos = *app.world().entity(chaser).get::<Position>().unwrap();
        assert!(
            final_pos.x.abs() < 0.1,
            "creature should be at spawn x=0, got x={}",
            final_pos.x
        );
    }
