use super::*;

#[test]
fn input_routes_to_correct_player() {
    let mut app = setup_app_with_time();
    let player_a = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let player_b = spawn_player(app.world_mut(), 100.0, 100.0, 100.0);
    let input = make_input(1.0, 0.0, 0.0);

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &input,
                player_a,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });

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
        facing_yaw: 1.5,
        ..make_input(1.0, 0.0, 0.0)
    };
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let rot = *app.world().entity(player).get::<Rotation>().unwrap();
    assert!((rot.y - 1.5).abs() < 1e-6);
}

#[test]
fn identical_input_does_not_dirty_rotation_or_speed_but_real_changes_do() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let first_input = make_input(0.0, 0.0, 0.0);

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &first_input,
                player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });
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

    let second_input = make_input(0.0, 0.0, 0.0);
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &second_input,
                player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });
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
        facing_yaw: 1.5,
        running: false,
        ..make_input(0.0, 0.0, 0.0)
    };
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &changed_input,
                player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });
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
        running: false,
        ..make_input(1.0, 0.0, 0.0)
    };
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    assert!((speed.0 - DEFAULT_WALK_SPEED).abs() < 1e-6);
}

#[test]
fn the_reported_position_is_adopted_across_the_ground() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    // The client ran diagonally (W+D) while its wire direction is its facing.
    let input = reporting([0.2, 5.0, 0.2], make_input(1.0, 0.0, 0.0));
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    assert_eq!((pos.x, pos.z), (0.2, 0.2));
    assert_eq!(
        pos.y, 0.0,
        "height stays the server's gravity and ground clamp"
    );
}

#[test]
fn walking_moves_at_most_2_5_yards_per_sec() {
    assert!((DEFAULT_WALK_SPEED - 2.5).abs() < 1e-6);
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = reporting(
        FAR_X,
        PlayerInput {
            running: false,
            ..make_input(1.0, 0.0, 0.0)
        },
    );
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let expected_x = DEFAULT_WALK_SPEED * MAX_INPUT_STEP_SECS;
    assert!(
        (pos.x - expected_x).abs() < 1e-6,
        "walk moved {}, expected {expected_x}",
        pos.x,
    );
    assert!((pos.y - 0.0).abs() < 1e-6, "y unchanged");
}

#[test]
fn running_moves_at_most_7_0_yards_per_sec() {
    assert!((DEFAULT_RUN_SPEED - 7.0).abs() < 1e-6);
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let input = reporting(FAR_X, make_input(1.0, 0.0, 0.0));
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let expected_x = DEFAULT_RUN_SPEED * MAX_INPUT_STEP_SECS;
    assert!(
        (pos.x - expected_x).abs() < 1e-6,
        "run moved {}, expected {expected_x}",
        pos.x,
    );
}

#[test]
fn spell_auras_191_catalog_application_limits_reported_movement() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::harness;
    use crate::spell_info::SpellStore;
    const PILLAR_OF_FROST: u32 = 51271;
    let mut h = harness(Vec::new(), &[PILLAR_OF_FROST], 6).unwrap();
    let player = spawn_player(h.app.world_mut(), 0.0, 0.0, 0.0);
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(PILLAR_OF_FROST)
        .unwrap();
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: player,
            target: player,
            spell_id: PILLAR_OF_FROST,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
    let input = reporting(FAR_X, make_input(1.0, 0.0, 0.0));
    h.app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        })
        .unwrap();
    let position = h.app.world().get::<Position>(player).unwrap();
    let speed = h.app.world().get::<MovementSpeed>(player).unwrap();
    assert!((speed.0 - 2.0).abs() < 0.0001);
    assert!((position.x - 2.0 * MAX_INPUT_STEP_SECS).abs() < 0.0001);
}

#[test]
fn aura_speed_multiplier_scales_movement() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    set_aura_speed(app.world_mut(), player, 0.50);

    let input = reporting(FAR_X, make_input(1.0, 0.0, 0.0));
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let pos = *app.world().entity(player).get::<Position>().unwrap();
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let expected_speed = DEFAULT_RUN_SPEED * 0.5;
    assert!(
        (speed.0 - expected_speed).abs() < 1e-6,
        "speed {}, expected {expected_speed}",
        speed.0,
    );
    let expected_x = expected_speed * MAX_INPUT_STEP_SECS;
    assert!(
        (pos.x - expected_x).abs() < 1e-6,
        "moved {}, expected {expected_x}",
        pos.x,
    );

    set_aura_speed(app.world_mut(), player, 1.30);
    app.world_mut()
        .entity_mut(player)
        .get_mut::<Position>()
        .unwrap()
        .x = 0.0;

    let input2 = reporting(FAR_X, make_input(1.0, 0.0, 0.0));
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &input2,
                player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });
    let pos2 = *app.world().entity(player).get::<Position>().unwrap();
    let expected_hasted = DEFAULT_RUN_SPEED * 1.30 * MAX_INPUT_STEP_SECS;
    assert!(
        (pos2.x - expected_hasted).abs() < 1e-5,
        "hasted moved {}, expected {expected_hasted}",
        pos2.x,
    );
}

#[test]
fn zero_effective_speed_does_not_dirty_position() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 10.0, 20.0, 30.0);
    set_aura_speed(app.world_mut(), player, 0.00);
    app.update();

    let initial_position = *app.world().entity(player).get::<Position>().unwrap();
    let initial_position_tick = app
        .world()
        .entity(player)
        .get_ref::<Position>()
        .unwrap()
        .last_changed();
    let input = make_input(1.0, 0.0, 0.0);

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });

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
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 10.0, 20.0, 30.0);

    set_aura_speed(app.world_mut(), player, 0.00);

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });

    let pos = *app.world().entity(player).get::<Position>().unwrap();
    assert_eq!(pos.x, 10.0, "x unchanged while rooted");
    assert_eq!(pos.y, 20.0, "y unchanged while rooted");
    assert_eq!(pos.z, 30.0, "z unchanged while rooted");
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    assert_eq!(speed.0, 0.0, "speed is zero when rooted");
}

#[test]
fn backward_movement_at_reduced_speed() {
    let mut app = setup_app_with_time();
    let fwd_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let bwd_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    let fwd_input = far_facing_forward([0.0, 0.0, 1.0]);
    let bwd_input = far_facing_forward([0.0, 0.0, -1.0]);

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &fwd_input,
                fwd_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
            apply_movement_input(
                &bwd_input,
                bwd_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });

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

    let fwd_input = far_facing_forward([0.0, 0.0, 1.0]);
    let strafe_input = far_facing_forward([1.0, 0.0, 0.0]);
    let backpedal_input = far_facing_forward([0.0, 0.0, -1.0]);

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &fwd_input,
                fwd_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
            apply_movement_input(
                &strafe_input,
                strafe_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
            apply_movement_input(
                &backpedal_input,
                backpedal_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });

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

/// Running along `direction` while facing +z, reporting 10 yd that way: farther than
/// any speed covers in one input.
fn far_facing_forward(direction: [f32; 3]) -> PlayerInput {
    PlayerInput {
        transport: None,
        direction,
        facing_yaw: 0.0,
        position: direction.map(|axis| axis * 10.0),
        ..make_input(0.0, 0.0, 0.0)
    }
}
