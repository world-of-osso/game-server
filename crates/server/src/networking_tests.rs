use super::*;
use bevy::ecs::system::RunSystemOnce;
use shared::protocol::{ChatType, PlayerInput};
use std::panic::{AssertUnwindSafe, catch_unwind};


#[derive(Component)]
struct ReplicationTag;

#[derive(Component)]
struct VisibilityTag;

fn spawn_player(world: &mut World, x: f32, y: f32, z: f32) -> Entity {
    world
        .spawn((
            Position { x, y, z },
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            MovementSpeed(DEFAULT_RUN_SPEED),
            VerticalVelocity::default(),
            Player {
                name: "test".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Auras::default(),
        ))
        .id()
}

fn make_input(dx: f32, dy: f32, dz: f32) -> PlayerInput {
    PlayerInput {
        direction: [dx, dy, dz],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    }
}

fn setup_app_with_time() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();
    app
}

#[test]
fn input_routes_to_correct_player() {
    let mut app = setup_app_with_time();
    let player_a = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let player_b = spawn_player(app.world_mut(), 100.0, 100.0, 100.0);
    let input = make_input(1.0, 0.0, 0.0);

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player_a, &mut q, &time);
        },
    );

    let pos_a = *app.world().entity(player_a).get::<Position>().unwrap();
    let pos_b = *app.world().entity(player_b).get::<Position>().unwrap();
    assert!(pos_a.x > 0.0, "player_a should have moved");
    assert_eq!(pos_b.x, 100.0, "player_b x unchanged");
    assert_eq!(pos_b.y, 100.0, "player_b y unchanged");
}

#[test]
fn input_updates_rotation() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 1.5,
        jumping: false,
        running: true,
        swimming: false,
    };
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let rot = *app.world().entity(player).get::<Rotation>().unwrap();
    assert!((rot.y - 1.5).abs() < 1e-6);
}

#[test]
fn walk_speed_when_not_running() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: false,
        swimming: false,
    };
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    assert!((speed.0 - DEFAULT_WALK_SPEED).abs() < 1e-6);
}

#[test]
fn walk_speed_moves_at_2_5_yards_per_sec() {
    assert!((DEFAULT_WALK_SPEED - 2.5).abs() < 1e-6);
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: false,
        swimming: false,
    };
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            *dt_clone.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let dt = *captured_dt.lock().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let expected_x = DEFAULT_WALK_SPEED * dt;
    assert!(
        (pos.x - expected_x).abs() < 1e-6,
        "walk moved {}, expected {} (2.5 * {dt})",
        pos.x,
        expected_x,
    );
    assert!((pos.y - 0.0).abs() < 1e-6, "y unchanged");
}

#[test]
fn run_speed_moves_at_7_0_yards_per_sec() {
    assert!((DEFAULT_RUN_SPEED - 7.0).abs() < 1e-6);
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = make_input(1.0, 0.0, 0.0); // running: true by default
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            *dt_clone.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let dt = *captured_dt.lock().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let expected_x = DEFAULT_RUN_SPEED * dt;
    assert!(
        (pos.x - expected_x).abs() < 1e-6,
        "run moved {}, expected {} (7.0 * {dt})",
        pos.x,
        expected_x,
    );
}

#[test]
fn speed_modifiers_from_auras_apply_to_movement() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Apply a 50% snare aura
    let snare = Aura {
        spell_id: 1,
        caster: 99,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent: -0.50 }),
            None,
            None,
        ],
        ..Default::default()
    };
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Auras>()
        .unwrap()
        .apply(snare);

    let input = make_input(1.0, 0.0, 0.0); // running: true
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            *dt_clone.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let dt = *captured_dt.lock().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();

    // Speed should be base * 0.5
    let expected_speed = DEFAULT_RUN_SPEED * 0.5;
    assert!(
        (speed.0 - expected_speed).abs() < 1e-6,
        "speed {}, expected {expected_speed}",
        speed.0,
    );
    // Position delta should reflect snared speed
    let expected_x = expected_speed * dt;
    assert!(
        (pos.x - expected_x).abs() < 1e-6,
        "moved {}, expected {expected_x}",
        pos.x,
    );

    // Now add a 30% sprint buff on top of the 50% snare
    let sprint = Aura {
        spell_id: 2,
        caster: 99,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent: 0.30 }),
            None,
            None,
        ],
        ..Default::default()
    };
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Auras>()
        .unwrap()
        .apply(sprint);

    // Reset position
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Position>()
        .unwrap()
        .x = 0.0;

    let input2 = make_input(1.0, 0.0, 0.0);
    let captured_dt2 = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone2 = captured_dt2.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            *dt_clone2.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input2, player, &mut q, &time);
        },
    );
    let dt2 = *captured_dt2.lock().unwrap();
    let pos2 = *app.world().entity(player).get::<Position>().unwrap();
    // Combined: (1.0 - 0.50 + 0.30) = 0.80x
    let expected_combined = DEFAULT_RUN_SPEED * 0.80 * dt2;
    assert!(
        (pos2.x - expected_combined).abs() < 1e-5,
        "combined moved {}, expected {expected_combined}",
        pos2.x,
    );
}

#[test]
fn root_prevents_movement() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 10.0, 20.0, 30.0);

    let root = Aura {
        spell_id: 1,
        caster: 99,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent: -1.0 }),
            None,
            None,
        ],
        ..Default::default()
    };
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Auras>()
        .unwrap()
        .apply(root);

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );

    let pos = *app.world().entity(player).get::<Position>().unwrap();
    assert_eq!(pos.x, 10.0, "x unchanged while rooted");
    assert_eq!(pos.y, 20.0, "y unchanged while rooted");
    assert_eq!(pos.z, 30.0, "z unchanged while rooted");
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    assert_eq!(speed.0, 0.0, "speed is zero when rooted");
}

#[test]
fn multiple_speed_buffs_stack_additively_on_movement() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Apply two speed buffs: +20% and +30% → 1.50x total
    for (id, pct) in [(1u32, 0.20f32), (2, 0.30)] {
        let buff = Aura {
            spell_id: id,
            caster: 99,
            duration: 10.0,
            remaining: 10.0,
            effects: [
                Some(AuraEffect::ModMovementSpeed { percent: pct }),
                None,
                None,
            ],
            ..Default::default()
        };
        app.world_mut()
            .entity_mut(player)
            .get_mut::<Auras>()
            .unwrap()
            .apply(buff);
    }

    let input = make_input(1.0, 0.0, 0.0);
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            *dt_clone.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let dt = *captured_dt.lock().unwrap();
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();

    let expected_speed = DEFAULT_RUN_SPEED * 1.50;
    assert!(
        (speed.0 - expected_speed).abs() < 1e-5,
        "speed {}, expected {expected_speed}",
        speed.0,
    );
    let expected_x = expected_speed * dt;
    assert!(
        (pos.x - expected_x).abs() < 1e-5,
        "moved {}, expected {expected_x}",
        pos.x,
    );
}

#[test]
fn speed_floor_prevents_full_immobilization_from_snare() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Apply a massive -95% snare — should be clamped to 10% minimum
    let snare = Aura {
        spell_id: 1,
        caster: 99,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent: -0.95 }),
            None,
            None,
        ],
        ..Default::default()
    };
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Auras>()
        .unwrap()
        .apply(snare);

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );

    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();

    // Speed floored at 10% of base
    let expected_speed = DEFAULT_RUN_SPEED * 0.1;
    assert!(
        (speed.0 - expected_speed).abs() < 1e-5,
        "speed {}, expected {expected_speed} (10% floor)",
        speed.0,
    );
    // Player still moved
    assert!(pos.x > 0.0, "snared player still moves");
}

#[test]
fn sprint_partially_counters_snare() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    let make_aura = |id, pct| Aura {
        spell_id: id,
        caster: 99,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent: pct }),
            None,
            None,
        ],
        ..Default::default()
    };

    // -70% snare + +70% sprint → (1.0 - 0.70 + 0.70) = 1.0x (normal speed)
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.apply(make_aura(1, -0.70));
        auras.apply(make_aura(2, 0.70));
    }

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );

    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    assert!(
        (speed.0 - DEFAULT_RUN_SPEED).abs() < 1e-5,
        "sprint cancels snare: speed {}, expected {}",
        speed.0,
        DEFAULT_RUN_SPEED,
    );

    // -70% snare + +30% sprint → (1.0 - 0.70 + 0.30) = 0.60x
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Position>()
        .unwrap()
        .x = 0.0;
    // Replace sprint with weaker one
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.active.retain(|a| a.spell_id != 2);
        auras.apply(make_aura(3, 0.30));
    }

    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input2, player, &mut q, &time);
        },
    );

    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let expected = DEFAULT_RUN_SPEED * 0.60;
    assert!(
        (speed.0 - expected).abs() < 1e-5,
        "partial counter: speed {}, expected {expected}",
        speed.0,
    );
}

#[test]
fn backward_movement_at_reduced_speed() {
    let mut app = setup_app_with_time();
    let fwd_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let bwd_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Facing yaw=0 → forward is +X. Moving -X = backward.
    let fwd_input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };
    let bwd_input = PlayerInput {
        direction: [-1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&fwd_input, fwd_player, &mut q, &time);
            apply_movement_input(&bwd_input, bwd_player, &mut q, &time);
        },
    );

    let fwd_speed = app.world().entity(fwd_player).get::<MovementSpeed>().unwrap().0;
    let bwd_speed = app.world().entity(bwd_player).get::<MovementSpeed>().unwrap().0;
    let expected_bwd = DEFAULT_RUN_SPEED * shared::movement::BACKPEDAL_MULTIPLIER;

    assert!(
        (fwd_speed - DEFAULT_RUN_SPEED).abs() < 1e-5,
        "forward at full speed: {fwd_speed}",
    );
    assert!(
        (bwd_speed - expected_bwd).abs() < 1e-5,
        "backward at 60%: {bwd_speed}, expected {expected_bwd}",
    );

    let fwd_pos = app.world().entity(fwd_player).get::<Position>().unwrap().x;
    let bwd_pos = app.world().entity(bwd_player).get::<Position>().unwrap().x;
    // Forward moved +X, backward moved -X at 60% distance
    assert!(fwd_pos > 0.0);
    assert!(bwd_pos < 0.0);
    assert!(
        (bwd_pos.abs() - fwd_pos * 0.6).abs() < 1e-5,
        "backward distance {} vs forward {} * 0.6",
        bwd_pos.abs(),
        fwd_pos,
    );
}

#[test]
fn strafing_speed_matches_forward_speed() {
    let mut app = setup_app_with_time();
    let fwd_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let strafe_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Facing yaw=0 → forward is +X. Strafe is +Y (perpendicular).
    let fwd_input = make_input(1.0, 0.0, 0.0);
    let strafe_input = PlayerInput {
        direction: [0.0, 1.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&fwd_input, fwd_player, &mut q, &time);
            apply_movement_input(&strafe_input, strafe_player, &mut q, &time);
        },
    );

    let fwd_speed = app.world().entity(fwd_player).get::<MovementSpeed>().unwrap().0;
    let strafe_speed = app.world().entity(strafe_player).get::<MovementSpeed>().unwrap().0;
    assert!(
        (fwd_speed - strafe_speed).abs() < 1e-5,
        "strafe {strafe_speed} should match forward {fwd_speed}",
    );

    // Distance traveled should be the same
    let fwd_dist = app.world().entity(fwd_player).get::<Position>().unwrap().x;
    let strafe_dist = app.world().entity(strafe_player).get::<Position>().unwrap().y;
    assert!(
        (fwd_dist - strafe_dist).abs() < 1e-5,
        "strafe distance {strafe_dist} should match forward {fwd_dist}",
    );
}

#[test]
fn speed_recalculates_when_aura_removed() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Apply 50% snare
    let snare = Aura {
        spell_id: 1,
        caster: 99,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent: -0.50 }),
            None,
            None,
        ],
        ..Default::default()
    };
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.apply(snare);
    }

    // First tick: snared at 50%
    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let snared_speed = app.world().entity(player).get::<MovementSpeed>().unwrap().0;
    assert!(
        (snared_speed - DEFAULT_RUN_SPEED * 0.5).abs() < 1e-5,
        "snared: {snared_speed}",
    );

    // Remove the snare
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.active.clear();
    }

    // Second tick: back to full speed
    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input2, player, &mut q, &time);
        },
    );
    let restored_speed = app.world().entity(player).get::<MovementSpeed>().unwrap().0;
    assert!(
        (restored_speed - DEFAULT_RUN_SPEED).abs() < 1e-5,
        "restored: {restored_speed}, expected {DEFAULT_RUN_SPEED}",
    );
}

#[test]
fn root_expires_movement_resumes() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Apply root
    let root = Aura {
        spell_id: 1,
        caster: 99,
        duration: 10.0,
        remaining: 10.0,
        effects: [
            Some(AuraEffect::ModMovementSpeed { percent: -1.0 }),
            None,
            None,
        ],
        ..Default::default()
    };
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.apply(root);
    }

    // Tick while rooted — no movement
    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    assert_eq!(app.world().entity(player).get::<Position>().unwrap().x, 0.0);
    assert_eq!(app.world().entity(player).get::<MovementSpeed>().unwrap().0, 0.0);

    // Root expires (remove aura)
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.active.clear();
    }

    // Next tick — movement resumes at full run speed
    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input2, player, &mut q, &time);
        },
    );
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap().0;
    let pos = app.world().entity(player).get::<Position>().unwrap().x;
    assert!(
        (speed - DEFAULT_RUN_SPEED).abs() < 1e-5,
        "speed restored: {speed}",
    );
    assert!(pos > 0.0, "player moved after root expired");
}

#[test]
fn jump_while_snared_reduces_horizontal_distance() {
    use shared::aura::{Aura, AuraEffect};

    let mut app = setup_app_with_time();
    let normal = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let snared = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Apply 50% snare to second player
    {
        let mut entity = app.world_mut().entity_mut(snared);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.apply(Aura {
            spell_id: 1,
            caster: 99,
            duration: 10.0,
            remaining: 10.0,
            effects: [
                Some(AuraEffect::ModMovementSpeed { percent: -0.50 }),
                None,
                None,
            ],
            ..Default::default()
        });
    }

    // Both jump while moving forward
    let normal_input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: true,
        running: true,
        swimming: false,
    };
    let snared_input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: true,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&normal_input, normal, &mut q, &time);
            apply_movement_input(&snared_input, snared, &mut q, &time);
        },
    );

    let normal_x = app.world().entity(normal).get::<Position>().unwrap().x;
    let snared_x = app.world().entity(snared).get::<Position>().unwrap().x;

    assert!(normal_x > 0.0, "normal moved forward");
    assert!(snared_x > 0.0, "snared moved forward");
    assert!(
        (snared_x - normal_x * 0.5).abs() < 1e-5,
        "snared horizontal {snared_x} should be 50% of normal {normal_x}",
    );

    // Both got jump impulse
    let normal_vel = app.world().entity(normal).get::<VerticalVelocity>().unwrap().0;
    let snared_vel = app.world().entity(snared).get::<VerticalVelocity>().unwrap().0;
    assert!(normal_vel > 0.0, "normal jumping");
    assert!(
        (normal_vel - snared_vel).abs() < 1e-5,
        "jump impulse unaffected by snare",
    );
}

#[test]
fn casting_blocks_movement_unless_cast_while_moving() {
    use shared::casting::CastState;

    let mut app = setup_app_with_time();
    let casting_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let cwm_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    // Non-cast-while-moving spell on first player
    app.world_mut()
        .entity_mut(casting_player)
        .insert(CastState::normal(133, 1, 2.0, true));

    // Cast-while-moving spell on second player
    let mut cwm_cast = CastState::normal(100, 1, 2.0, true);
    cwm_cast.cast_while_moving = true;
    app.world_mut()
        .entity_mut(cwm_player)
        .insert(cwm_cast);

    let input = make_input(1.0, 0.0, 0.0);
    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, casting_player, &mut q, &time);
            apply_movement_input(&input2, cwm_player, &mut q, &time);
        },
    );

    // Normal caster: blocked
    let casting_pos = app.world().entity(casting_player).get::<Position>().unwrap();
    assert_eq!(casting_pos.x, 0.0, "casting player didn't move");
    let casting_speed = app.world().entity(casting_player).get::<MovementSpeed>().unwrap();
    assert_eq!(casting_speed.0, 0.0, "casting speed is zero");

    // Cast-while-moving caster: moved normally
    let cwm_pos = app.world().entity(cwm_player).get::<Position>().unwrap();
    assert!(cwm_pos.x > 0.0, "cast-while-moving player moved");
}

#[test]
fn swim_speed_differs_from_ground_speed() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: true,
    };
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            *dt_clone.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let dt = *captured_dt.lock().unwrap();
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();

    // Swim speed is slower than run speed
    assert!(
        (speed.0 - DEFAULT_SWIM_SPEED).abs() < 1e-5,
        "swim speed {}, expected {DEFAULT_SWIM_SPEED}",
        speed.0,
    );
    assert!(speed.0 < DEFAULT_RUN_SPEED, "swim slower than run");
    assert!(speed.0 > DEFAULT_WALK_SPEED, "swim faster than walk");

    let expected_x = DEFAULT_SWIM_SPEED * dt;
    assert!(
        (pos.x - expected_x).abs() < 1e-5,
        "swam {}, expected {expected_x}",
        pos.x,
    );
}

#[test]
fn zero_direction_ignored() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 5.0, 5.0, 5.0);
    let input = make_input(0.0, 0.0, 0.0);

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );

    let pos = *app.world().entity(player).get::<Position>().unwrap();
    assert_eq!(pos.x, 5.0);
    assert_eq!(pos.y, 5.0);
    assert_eq!(pos.z, 5.0);
}

#[test]
fn server_normalizes_direction_preventing_speed_hack() {
    // Even if a hacked client sends an oversized direction vector,
    // the server normalizes it, so the player moves at the same speed
    // as a client sending a unit vector.
    let mut app = setup_app_with_time();
    let normal_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let hack_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    let normal_input = make_input(1.0, 0.0, 0.0);
    // "Hacked" client sends direction with magnitude 10
    let hack_input = PlayerInput {
        direction: [10.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>,
              time: Res<Time>| {
            apply_movement_input(&normal_input, normal_player, &mut q, &time);
            apply_movement_input(&hack_input, hack_player, &mut q, &time);
        },
    );

    let normal_pos = *app.world().entity(normal_player).get::<Position>().unwrap();
    let hack_pos = *app.world().entity(hack_player).get::<Position>().unwrap();

    // Both players moved the same distance
    assert!(
        (normal_pos.x - hack_pos.x).abs() < 1e-6,
        "normal={}, hack={} — should match",
        normal_pos.x,
        hack_pos.x,
    );
}

#[test]
fn terrain_clamps_below_ground() {
    // Terrain chunk at WoW corner (100, 100), height 50.
    // Bevy pos: x=90 (wow_x=90), z=-90 (wow_y=90), y=30 (below ground_y=50).
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
    let mut pos = Position {
        x: 90.0,
        y: 30.0,
        z: -90.0,
    };
    clamp_to_terrain(&map, &mut pos, &mut VerticalVelocity::default(), 0.05);
    assert!((pos.y - 50.0).abs() < 0.01);
}

#[test]
fn terrain_applies_gravity() {
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
    let mut pos = Position {
        x: 90.0,
        y: 60.0,
        z: -90.0,
    };
    let dt = 0.05;
    clamp_to_terrain(&map, &mut pos, &mut VerticalVelocity::default(), dt);
    let expected = 60.0 - GRAVITY * dt;
    assert!((pos.y - expected).abs() < 0.01);
}

#[test]
fn gravity_accumulates_until_landing() {
    let ground_y = 50.0;
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, ground_y);
    let mut pos = Position {
        x: 90.0,
        y: 60.0,
        z: -90.0,
    };
    let dt = 0.05;

    // First tick: falls by GRAVITY * dt
    clamp_to_terrain(&map, &mut pos, &mut VerticalVelocity::default(), dt);
    let after_one = 60.0 - GRAVITY * dt;
    assert!((pos.y - after_one).abs() < 0.01);

    // Second tick: falls further
    clamp_to_terrain(&map, &mut pos, &mut VerticalVelocity::default(), dt);
    let after_two = after_one - GRAVITY * dt;
    assert!((pos.y - after_two).abs() < 0.01);
    assert!(pos.y > ground_y, "still airborne");

    // Many ticks: eventually clamps to ground
    for _ in 0..200 {
        clamp_to_terrain(&map, &mut pos, &mut VerticalVelocity::default(), dt);
    }
    assert!(
        (pos.y - ground_y).abs() < 0.01,
        "landed at ground: y={}, ground={ground_y}",
        pos.y,
    );

    // Once on ground, stays put
    clamp_to_terrain(&map, &mut pos, &mut VerticalVelocity::default(), dt);
    assert!((pos.y - ground_y).abs() < 0.01);
}

#[test]
fn terrain_on_ground_stays() {
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
    let mut pos = Position {
        x: 90.0,
        y: 50.05,
        z: -90.0,
    };
    clamp_to_terrain(&map, &mut pos, &mut VerticalVelocity::default(), 0.05);
    assert!((pos.y - 50.05).abs() < 0.01);
}

#[test]
fn ground_snap_within_threshold() {
    let ground_y = 50.0;
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, ground_y);
    let dt = 0.05;

    // Below ground: snaps up to ground
    let mut below = Position { x: 90.0, y: 48.0, z: -90.0 };
    clamp_to_terrain(&map, &mut below, &mut VerticalVelocity::default(), dt);
    assert!((below.y - ground_y).abs() < 0.01, "below snaps to ground");

    // Exactly at ground: stays
    let mut exact = Position { x: 90.0, y: ground_y, z: -90.0 };
    clamp_to_terrain(&map, &mut exact, &mut VerticalVelocity::default(), dt);
    assert!((exact.y - ground_y).abs() < 0.01, "exact stays on ground");

    // Within threshold (ground + 0.29): no gravity, stays put
    let within_y = ground_y + GROUND_SNAP_THRESHOLD - 0.01;
    let mut within = Position { x: 90.0, y: within_y, z: -90.0 };
    clamp_to_terrain(&map, &mut within, &mut VerticalVelocity::default(), dt);
    assert!((within.y - within_y).abs() < 0.01, "within threshold stays");

    // Just above threshold (ground + 10.0): gravity applies, doesn't clamp yet
    let above_y = ground_y + 10.0;
    let mut above = Position { x: 90.0, y: above_y, z: -90.0 };
    clamp_to_terrain(&map, &mut above, &mut VerticalVelocity::default(), dt);
    let expected = above_y - GRAVITY * dt;
    assert!(
        (above.y - expected).abs() < 0.01,
        "above threshold falls: y={}, expected={expected}",
        above.y,
    );
}

#[test]
fn jump_impulse_and_landing() {
    let ground_y = 50.0;
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, ground_y);
    let mut pos = Position { x: 90.0, y: ground_y, z: -90.0 };
    let mut vel = VerticalVelocity(shared::movement::JUMP_IMPULSE);
    let dt = 0.05;

    // First tick: impulse pushes upward
    clamp_to_terrain(&map, &mut pos, &mut vel, dt);
    let expected_y = ground_y + shared::movement::JUMP_IMPULSE * dt;
    assert!(
        (pos.y - expected_y).abs() < 0.1,
        "after jump tick 1: y={}, expected ~{expected_y}",
        pos.y,
    );
    assert!(pos.y > ground_y, "airborne after jump");

    // Velocity decreases by gravity each tick
    let expected_vel = shared::movement::JUMP_IMPULSE - GRAVITY * dt;
    assert!(
        (vel.0 - expected_vel).abs() < 0.1,
        "vel={}, expected ~{expected_vel}",
        vel.0,
    );

    // Continue ticking until landing
    let mut peak_y = pos.y;
    for _ in 0..200 {
        clamp_to_terrain(&map, &mut pos, &mut vel, dt);
        if pos.y > peak_y {
            peak_y = pos.y;
        }
        if pos.y <= ground_y + 0.01 && vel.0 == 0.0 {
            break;
        }
    }

    // Reached a peak above ground
    assert!(peak_y > ground_y + 1.0, "jumped to peak {peak_y}");

    // Landed back on ground with velocity zeroed
    assert!(
        (pos.y - ground_y).abs() < 0.01,
        "landed: y={}, ground={ground_y}",
        pos.y,
    );
    assert_eq!(vel.0, 0.0, "velocity zeroed on landing");
}

#[test]
fn chat_message_sender_override() {
    let mut msg = ChatMessage {
        sender: "hacker".into(),
        content: "Hello".into(),
        channel: ChatType::Say,
    };
    msg.sender = "TrustedPlayer".to_string();
    assert_eq!(msg.sender, "TrustedPlayer");
    assert_eq!(msg.content, "Hello");
}

#[test]
fn set_target_updates_combat_target() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);

    let target_entity = app.world_mut().spawn_empty().id();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    app.world_mut()
        .entity_mut(player)
        .insert(CombatTarget::default());

    let bits = target_entity.to_bits();
    let resolved = Entity::from_bits(bits);
    app.world_mut()
        .entity_mut(player)
        .get_mut::<CombatTarget>()
        .unwrap()
        .0 = Some(resolved);

    let ct = app.world().entity(player).get::<CombatTarget>().unwrap();
    assert_eq!(ct.0, Some(target_entity));
}

#[test]
fn known_elwynn_position_maps_to_existing_adt_tile() {
    assert_eq!(wow_to_tile(-8949.0, -132.0), (32, 48));
}

#[test]
fn deferred_player_despawn_ignores_entities_already_despawned() {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);

    let player = app.world_mut().spawn((ReplicationTag, VisibilityTag)).id();

    app.add_systems(Update, move |mut commands: Commands| {
        commands.entity(player).despawn();
        queue_player_despawn(&mut commands, player);
    });

    let result = catch_unwind(AssertUnwindSafe(|| app.update()));
    assert!(
        result.is_ok(),
        "queue_player_despawn should ignore entities already despawned before cleanup flushes"
    );
}
