use super::*;

#[test]
fn input_routes_to_correct_player() {
    let mut app = setup_app_with_time();
    let player_a = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let player_b = spawn_player(app.world_mut(), 100.0, 100.0, 100.0);
    let input = make_input(1.0, 0.0, 0.0);

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let rot = *app.world().entity(player).get::<Rotation>().unwrap();
    assert!((rot.y - 1.5).abs() < 1e-6);
}

#[test]
fn identical_input_does_not_dirty_rotation_or_speed_but_real_changes_do() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let first_input = PlayerInput {
        direction: [0.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&first_input, player, &mut q, &time);
        },
    );
    let rotation_tick_after_first = app
        .world()
        .entity(player)
        .get_ref::<Rotation>()
        .unwrap()
        .last_changed();
    let speed_tick_after_first = app
        .world()
        .entity(player)
        .get_ref::<MovementSpeed>()
        .unwrap()
        .last_changed();

    app.update();

    let second_input = PlayerInput {
        direction: [0.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&second_input, player, &mut q, &time);
        },
    );
    let rotation_tick_after_second = app
        .world()
        .entity(player)
        .get_ref::<Rotation>()
        .unwrap()
        .last_changed();
    let speed_tick_after_second = app
        .world()
        .entity(player)
        .get_ref::<MovementSpeed>()
        .unwrap()
        .last_changed();

    assert_eq!(
        rotation_tick_after_second, rotation_tick_after_first,
        "identical input must not dirty Rotation"
    );
    assert_eq!(
        speed_tick_after_second, speed_tick_after_first,
        "identical input must not dirty MovementSpeed"
    );

    app.update();

    let changed_input = PlayerInput {
        direction: [0.0, 0.0, 0.0],
        facing_yaw: 1.5,
        jumping: false,
        running: false,
        swimming: false,
    };
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&changed_input, player, &mut q, &time);
        },
    );
    let rotation_tick_after_change = app
        .world()
        .entity(player)
        .get_ref::<Rotation>()
        .unwrap()
        .last_changed();
    let speed_tick_after_change = app
        .world()
        .entity(player)
        .get_ref::<MovementSpeed>()
        .unwrap()
        .last_changed();
    let rotation = app.world().entity(player).get::<Rotation>().unwrap();
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();

    assert_ne!(
        rotation_tick_after_change, rotation_tick_after_second,
        "changed yaw must dirty Rotation"
    );
    assert_ne!(
        speed_tick_after_change, speed_tick_after_second,
        "changed movement mode must dirty MovementSpeed"
    );
    assert!((rotation.y - 1.5).abs() < 1e-6);
    assert!((speed.0 - DEFAULT_WALK_SPEED).abs() < 1e-6);
}

#[test]
fn walk_speed_when_not_running() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: std::f32::consts::FRAC_PI_2,
        jumping: false,
        running: false,
        swimming: false,
    };
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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
        facing_yaw: std::f32::consts::FRAC_PI_2,
        jumping: false,
        running: false,
        swimming: false,
    };
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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
    let input = make_input(1.0, 0.0, 0.0);
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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

    let input = make_input(1.0, 0.0, 0.0);
    let captured_dt = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone = captured_dt.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            *dt_clone.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let dt = *captured_dt.lock().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let expected_speed = DEFAULT_RUN_SPEED * 0.5;
    assert!(
        (speed.0 - expected_speed).abs() < 1e-6,
        "speed {}, expected {expected_speed}",
        speed.0,
    );
    let expected_x = expected_speed * dt;
    assert!(
        (pos.x - expected_x).abs() < 1e-6,
        "moved {}, expected {expected_x}",
        pos.x,
    );

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
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Position>()
        .unwrap()
        .x = 0.0;

    let input2 = make_input(1.0, 0.0, 0.0);
    let captured_dt2 = std::sync::Arc::new(std::sync::Mutex::new(0.0_f32));
    let dt_clone2 = captured_dt2.clone();
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            *dt_clone2.lock().unwrap() = time.delta_secs();
            apply_movement_input(&input2, player, &mut q, &time);
        },
    );
    let dt2 = *captured_dt2.lock().unwrap();
    let pos2 = *app.world().entity(player).get::<Position>().unwrap();
    let expected_combined = DEFAULT_RUN_SPEED * 0.80 * dt2;
    assert!(
        (pos2.x - expected_combined).abs() < 1e-5,
        "combined moved {}, expected {expected_combined}",
        pos2.x,
    );
}

#[test]
fn zero_effective_speed_does_not_dirty_position() {
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
    app.update();

    let initial_position = *app.world().entity(player).get::<Position>().unwrap();
    let initial_position_tick = app
        .world()
        .entity(player)
        .get_ref::<Position>()
        .unwrap()
        .last_changed();
    let input = make_input(1.0, 0.0, 0.0);

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );

    assert_eq!(
        *app.world().entity(player).get::<Position>().unwrap(),
        initial_position,
        "zero effective speed must not move the player"
    );
    assert_eq!(
        app.world()
            .entity(player)
            .get_ref::<Position>()
            .unwrap()
            .last_changed(),
        initial_position_tick,
        "zero effective speed must not dirty Position"
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
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );

    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let expected_speed = DEFAULT_RUN_SPEED * 0.1;
    assert!(
        (speed.0 - expected_speed).abs() < 1e-5,
        "speed {}, expected {expected_speed} (10% floor)",
        speed.0,
    );
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

    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.apply(make_aura(1, -0.70));
        auras.apply(make_aura(2, 0.70));
    }

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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

    app.world_mut()
        .entity_mut(player)
        .get_mut::<Position>()
        .unwrap()
        .x = 0.0;
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.active.retain(|a| a.spell_id != 2);
        auras.apply(make_aura(3, 0.30));
    }

    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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

    let fwd_input = PlayerInput {
        direction: [0.0, 0.0, 1.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };
    let bwd_input = PlayerInput {
        direction: [0.0, 0.0, -1.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&fwd_input, fwd_player, &mut q, &time);
            apply_movement_input(&bwd_input, bwd_player, &mut q, &time);
        },
    );

    let fwd_speed = app
        .world()
        .entity(fwd_player)
        .get::<MovementSpeed>()
        .unwrap()
        .0;
    let bwd_speed = app
        .world()
        .entity(bwd_player)
        .get::<MovementSpeed>()
        .unwrap()
        .0;
    let expected_bwd = DEFAULT_RUN_SPEED * shared::movement::BACKPEDAL_MULTIPLIER;

    assert!(
        (fwd_speed - DEFAULT_RUN_SPEED).abs() < 1e-5,
        "forward at full speed: {fwd_speed}",
    );
    assert!(
        (bwd_speed - expected_bwd).abs() < 1e-5,
        "backward at 60%: {bwd_speed}, expected {expected_bwd}",
    );

    let fwd_pos = app.world().entity(fwd_player).get::<Position>().unwrap().z;
    let bwd_pos = app.world().entity(bwd_player).get::<Position>().unwrap().z;
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
fn strafing_speed_is_reduced_but_faster_than_backpedal() {
    let mut app = setup_app_with_time();
    let fwd_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let strafe_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let backpedal_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    let fwd_input = PlayerInput {
        direction: [0.0, 0.0, 1.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };
    let strafe_input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };
    let backpedal_input = PlayerInput {
        direction: [0.0, 0.0, -1.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&fwd_input, fwd_player, &mut q, &time);
            apply_movement_input(&strafe_input, strafe_player, &mut q, &time);
            apply_movement_input(&backpedal_input, backpedal_player, &mut q, &time);
        },
    );

    let fwd_speed = app
        .world()
        .entity(fwd_player)
        .get::<MovementSpeed>()
        .unwrap()
        .0;
    let strafe_speed = app
        .world()
        .entity(strafe_player)
        .get::<MovementSpeed>()
        .unwrap()
        .0;
    let backpedal_speed = app
        .world()
        .entity(backpedal_player)
        .get::<MovementSpeed>()
        .unwrap()
        .0;
    let expected_strafe = DEFAULT_RUN_SPEED * shared::movement::STRAFE_MULTIPLIER;
    assert!(
        (strafe_speed - expected_strafe).abs() < 1e-5,
        "strafe {strafe_speed} should be reduced to {expected_strafe}",
    );
    assert!(strafe_speed < fwd_speed);
    assert!(strafe_speed > backpedal_speed);

    let fwd_dist = app.world().entity(fwd_player).get::<Position>().unwrap().z;
    let strafe_dist = app
        .world()
        .entity(strafe_player)
        .get::<Position>()
        .unwrap()
        .x;
    let backpedal_dist = app
        .world()
        .entity(backpedal_player)
        .get::<Position>()
        .unwrap()
        .z
        .abs();
    assert!(
        (strafe_dist - fwd_dist * shared::movement::STRAFE_MULTIPLIER).abs() < 1e-5,
        "strafe distance {strafe_dist} should be reduced from forward {fwd_dist}",
    );
    assert!(strafe_dist > backpedal_dist);
}
