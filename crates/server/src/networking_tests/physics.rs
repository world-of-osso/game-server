use super::*;

#[test]
fn speed_recalculates_when_aura_removed() {
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
    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.apply(snare);
    }

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    let snared_speed = app.world().entity(player).get::<MovementSpeed>().unwrap().0;
    assert!(
        (snared_speed - DEFAULT_RUN_SPEED * 0.5).abs() < 1e-5,
        "snared: {snared_speed}",
    );

    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.active.clear();
    }

    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&input, player, &mut q, &time);
        },
    );
    assert_eq!(app.world().entity(player).get::<Position>().unwrap().x, 0.0);
    assert_eq!(
        app.world().entity(player).get::<MovementSpeed>().unwrap().0,
        0.0
    );

    {
        let mut entity = app.world_mut().entity_mut(player);
        let mut auras = entity.get_mut::<Auras>().unwrap();
        auras.active.clear();
    }

    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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

    let normal_vel = app
        .world()
        .entity(normal)
        .get::<VerticalVelocity>()
        .unwrap()
        .0;
    let snared_vel = app
        .world()
        .entity(snared)
        .get::<VerticalVelocity>()
        .unwrap()
        .0;
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

    app.world_mut()
        .entity_mut(casting_player)
        .insert(CastState::normal(133, 1, 2.0, true));

    let mut cwm_cast = CastState::normal(100, 1, 2.0, true);
    cwm_cast.cast_while_moving = true;
    app.world_mut().entity_mut(cwm_player).insert(cwm_cast);

    let input = make_input(1.0, 0.0, 0.0);
    let input2 = make_input(1.0, 0.0, 0.0);
    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&input, casting_player, &mut q, &time);
            apply_movement_input(&input2, cwm_player, &mut q, &time);
        },
    );

    let casting_pos = app
        .world()
        .entity(casting_player)
        .get::<Position>()
        .unwrap();
    assert_eq!(casting_pos.x, 0.0, "casting player didn't move");
    let casting_speed = app
        .world()
        .entity(casting_player)
        .get::<MovementSpeed>()
        .unwrap();
    assert_eq!(casting_speed.0, 0.0, "casting speed is zero");

    let cwm_pos = app.world().entity(cwm_player).get::<Position>().unwrap();
    assert!(cwm_pos.x > 0.0, "cast-while-moving player moved");
}

#[test]
fn swim_speed_differs_from_ground_speed() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = PlayerInput {
        direction: [1.0, 0.0, 0.0],
        facing_yaw: std::f32::consts::FRAC_PI_2,
        jumping: false,
        running: true,
        swimming: true,
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
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();

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
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
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
    let mut app = setup_app_with_time();
    let normal_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let hack_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    let normal_input = make_input(1.0, 0.0, 0.0);
    let hack_input = PlayerInput {
        direction: [10.0, 0.0, 0.0],
        facing_yaw: std::f32::consts::FRAC_PI_2,
        jumping: false,
        running: true,
        swimming: false,
    };

    let _ = app.world_mut().run_system_once(
        move |mut q: Query<PlayerMovement, With<Player>>, time: Res<Time>| {
            apply_movement_input(&normal_input, normal_player, &mut q, &time);
            apply_movement_input(&hack_input, hack_player, &mut q, &time);
        },
    );

    let normal_pos = *app.world().entity(normal_player).get::<Position>().unwrap();
    let hack_pos = *app.world().entity(hack_player).get::<Position>().unwrap();
    assert!(
        (normal_pos.x - hack_pos.x).abs() < 1e-6,
        "normal={}, hack={} — should match",
        normal_pos.x,
        hack_pos.x,
    );
}

#[test]
fn terrain_clamps_below_ground() {
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
    let mut pos = Position {
        x: 90.0,
        y: 30.0,
        z: -90.0,
    };
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    clamp_to_terrain(
        &map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        0.05,
    );
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
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
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
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);

    clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
    let after_one = 60.0 - GRAVITY * dt;
    assert!((pos.y - after_one).abs() < 0.01);

    clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
    let after_two = after_one - GRAVITY * dt;
    assert!((pos.y - after_two).abs() < 0.01);
    assert!(pos.y > ground_y, "still airborne");

    for _ in 0..200 {
        clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
    }
    assert!(
        (pos.y - ground_y).abs() < 0.01,
        "landed at ground: y={}, ground={ground_y}",
        pos.y,
    );

    clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
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
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    clamp_to_terrain(
        &map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        0.05,
    );
    assert!((pos.y - 50.05).abs() < 0.01);
}

#[test]
fn ground_snap_within_threshold() {
    let ground_y = 50.0;
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, ground_y);
    let dt = 0.05;

    let mut below = Position {
        x: 90.0,
        y: 48.0,
        z: -90.0,
    };
    let mut below_vel = VerticalVelocity::default();
    let mut below_fall = FallTracker::grounded(below.y);
    let mut below_health = Health {
        current: 100.0,
        max: 100.0,
    };
    clamp_to_terrain(
        &map,
        &mut below,
        &mut below_vel,
        &mut below_fall,
        &mut below_health,
        dt,
    );
    assert!((below.y - ground_y).abs() < 0.01, "below snaps to ground");

    let mut exact = Position {
        x: 90.0,
        y: ground_y,
        z: -90.0,
    };
    let mut exact_vel = VerticalVelocity::default();
    let mut exact_fall = FallTracker::grounded(exact.y);
    let mut exact_health = Health {
        current: 100.0,
        max: 100.0,
    };
    clamp_to_terrain(
        &map,
        &mut exact,
        &mut exact_vel,
        &mut exact_fall,
        &mut exact_health,
        dt,
    );
    assert!((exact.y - ground_y).abs() < 0.01, "exact stays on ground");

    let within_y = ground_y + GROUND_SNAP_THRESHOLD - 0.01;
    let mut within = Position {
        x: 90.0,
        y: within_y,
        z: -90.0,
    };
    let mut within_vel = VerticalVelocity::default();
    let mut within_fall = FallTracker::grounded(within.y);
    let mut within_health = Health {
        current: 100.0,
        max: 100.0,
    };
    clamp_to_terrain(
        &map,
        &mut within,
        &mut within_vel,
        &mut within_fall,
        &mut within_health,
        dt,
    );
    assert!((within.y - within_y).abs() < 0.01, "within threshold stays");

    let above_y = ground_y + 10.0;
    let mut above = Position {
        x: 90.0,
        y: above_y,
        z: -90.0,
    };
    let mut above_vel = VerticalVelocity::default();
    let mut above_fall = FallTracker::grounded(above.y);
    let mut above_health = Health {
        current: 100.0,
        max: 100.0,
    };
    clamp_to_terrain(
        &map,
        &mut above,
        &mut above_vel,
        &mut above_fall,
        &mut above_health,
        dt,
    );
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
    let mut pos = Position {
        x: 90.0,
        y: ground_y,
        z: -90.0,
    };
    let mut vel = VerticalVelocity(shared::movement::JUMP_IMPULSE);
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    let dt = 0.05;

    clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
    let expected_y = ground_y + shared::movement::JUMP_IMPULSE * dt;
    assert!(
        (pos.y - expected_y).abs() < 0.1,
        "after jump tick 1: y={}, expected ~{expected_y}",
        pos.y,
    );
    assert!(pos.y > ground_y, "airborne after jump");

    let expected_vel = shared::movement::JUMP_IMPULSE - GRAVITY * dt;
    assert!(
        (vel.0 - expected_vel).abs() < 0.1,
        "vel={}, expected ~{expected_vel}",
        vel.0,
    );

    let mut peak_y = pos.y;
    for _ in 0..200 {
        clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
        if pos.y > peak_y {
            peak_y = pos.y;
        }
        if pos.y <= ground_y + 0.01 && vel.0 == 0.0 {
            break;
        }
    }

    assert!(peak_y > ground_y + 1.0, "jumped to peak {peak_y}");
    assert!(
        (pos.y - ground_y).abs() < 0.01,
        "landed: y={}, ground={ground_y}",
        pos.y,
    );
    assert_eq!(vel.0, 0.0, "velocity zeroed on landing");
}

#[test]
fn short_fall_does_not_deal_damage() {
    let ground_y = 50.0;
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, ground_y);
    let mut pos = Position {
        x: 90.0,
        y: ground_y + shared::movement::FALL_DAMAGE_THRESHOLD - 0.1,
        z: -90.0,
    };
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    let dt = 0.05;

    for _ in 0..200 {
        clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
        if pos.y <= ground_y + 0.01 && vel.0 == 0.0 {
            break;
        }
    }

    assert_eq!(health.current, 100.0);
}

#[test]
fn long_fall_deals_damage_on_landing_once() {
    let ground_y = 50.0;
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, ground_y);
    let start_y = ground_y + 20.0;
    let mut pos = Position {
        x: 90.0,
        y: start_y,
        z: -90.0,
    };
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    let expected_damage = 100.0 * shared::movement::fall_damage_percent(start_y - ground_y);
    let dt = 0.05;

    for _ in 0..400 {
        clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
        if pos.y <= ground_y + 0.01 && vel.0 == 0.0 {
            break;
        }
    }

    assert!(
        (health.current - (100.0 - expected_damage)).abs() < 0.05,
        "health={}, expected_damage={expected_damage}",
        health.current,
    );

    let landed_health = health.current;
    clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
    assert_eq!(
        health.current, landed_health,
        "damage should not repeat after landing"
    );
}

#[test]
fn lethal_fall_reduces_health_to_zero() {
    let ground_y = 50.0;
    let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, ground_y);
    let mut pos = Position {
        x: 90.0,
        y: ground_y + 70.0,
        z: -90.0,
    };
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    let dt = 0.05;

    for _ in 0..800 {
        clamp_to_terrain(&map, &mut pos, &mut vel, &mut fall_tracker, &mut health, dt);
        if pos.y <= ground_y + 0.01 && vel.0 == 0.0 {
            break;
        }
    }

    assert_eq!(health.current, 0.0);
}
