use super::*;
use crate::ground::GroundMap;

/// Probe `map` at `pos` and clamp to that ground, as the gravity system does.
fn probe_and_clamp(
    map: &mut GroundMap,
    pos: &mut Position,
    vel: &mut VerticalVelocity,
    fall_tracker: &mut FallTracker,
    health: &mut Health,
    dt: f32,
) {
    let probe = map.ground_at(0, Vec3::new(pos.x, pos.y, pos.z));
    clamp_to_ground(
        Entity::PLACEHOLDER,
        (probe, None),
        pos,
        vel,
        fall_tracker,
        health,
        dt,
        FallAuras::NONE,
    );
}

/// Real authored Cage hull and state through the production GO/gravity schedules.
#[test]
fn arena_live_trapdoor_authoritatively_supports_then_opening_drops_player() {
    use shared::protocol::{GameObjectCollision, GameObjectInfo, GameObjectVisible};
    use std::{collections::HashMap, path::PathBuf, time::Duration};
    let data = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        data.join("diagnostics/arena-coverage-2026-10-12/canonical-proof-data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let wdt: u32 = db
        .query_row("SELECT WdtFileDataID FROM map WHERE ID=2759", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut app = setup_app_with_time();
    app.insert_resource(GroundMap::new(&data, HashMap::from([(2759, wdt)])));
    app.insert_resource(crate::pvp::arena::Catalog::load(&db).unwrap());
    app.add_plugins(crate::game_objects::GameObjectPlugin);
    app.add_systems(FixedUpdate, apply_terrain_gravity);
    let map = InMap {
        map_id: 2759,
        instance_id: 786,
    };
    let player = spawn_player(app.world_mut(), 443.2981, -24.38845, -323.9937);
    app.world_mut()
        .entity_mut(player)
        .insert((map, shared::components::MovementControl::default()));
    let door = app
        .world_mut()
        .spawn((
            map,
            Position {
                x: 443.59,
                y: -24.8806,
                z: -321.262,
            },
            Rotation {
                x: 0.0,
                y: 1.5708 + std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
            GameObjectInfo {
                entry: 505683,
                go_type: 35,
                display_id: 99831,
                name: "Trapdoor".into(),
                scale: 1.0,
            },
            GameObjectCollision(true),
            GameObjectVisible(true),
        ))
        .id();
    for _ in 0..60 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0 / 60.0));
        app.world_mut().run_schedule(FixedUpdate);
    }
    let closed = app.world().get::<Position>(player).unwrap().y;
    assert!(
        (closed + 24.38845).abs() < 0.05,
        "closed live trapdoor must hold authoritative feet, got {closed}"
    );
    app.world_mut()
        .entity_mut(door)
        .insert(GameObjectCollision(false));
    for _ in 0..120 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.0 / 60.0));
        app.world_mut().run_schedule(FixedUpdate);
    }
    let opened = app.world().get::<Position>(player).unwrap().y;
    assert!(
        opened < -30.0,
        "opening must remove authoritative support: {opened}"
    );
}

#[test]
fn arena_live_collision_leaves_unrelated_map_input_and_gravity_unchanged() {
    use super::client_height::{flat_app, flat_spot, tick};
    use shared::protocol::{GameObjectCollision, GameObjectInfo};
    use std::path::PathBuf;
    let data = PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
    let db = rusqlite::Connection::open_with_flags(
        data.join("diagnostics/arena-coverage-2026-10-12/canonical-proof-data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let start = flat_spot(50.0);
    let (mut app, player, connection) = flat_app(50.0);
    app.insert_resource(crate::pvp::arena::Catalog::load(&db).unwrap());
    app.add_observer(crate::game_objects::collision::index_collision_model);
    app.add_systems(
        FixedUpdate,
        crate::game_objects::collision::sync_live_objects
            .before(process_player_inputs)
            .before(apply_terrain_gravity),
    );
    app.world_mut().spawn((
        InMap::continent(0),
        Position {
            x: start.x + 0.2919,
            y: 50.0,
            z: start.z + 2.7317,
        },
        Rotation {
            x: 0.0,
            y: 1.5708 + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
        GameObjectInfo {
            entry: 505683,
            go_type: 35,
            display_id: 99831,
            name: "Non-arena regression hull".into(),
            scale: 1.0,
        },
        GameObjectCollision(true),
    ));
    tick(
        &mut app,
        connection,
        vec![PlayerInput {
            position: [start.x + 0.35, start.y, start.z],
            ..make_input(1.0, 0.0, 0.0)
        }],
    );
    for _ in 0..30 {
        tick(&mut app, connection, vec![]);
    }
    let actual = app.world().get::<Position>(player).unwrap();
    assert!(
        actual.x > start.x + 0.3,
        "normal unrelated-map network report must move"
    );
    assert!(
        (actual.y - 50.0).abs() < 0.001,
        "unrelated-map gravity must stay on its static terrain: {actual:?}"
    );
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
}

#[test]
fn speed_recalculates_when_aura_removed() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    set_aura_speed(app.world_mut(), player, 0.50);

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let snared_speed = app.world().entity(player).get::<MovementSpeed>().unwrap().0;
    assert!(
        (snared_speed - DEFAULT_RUN_SPEED * 0.5).abs() < 1e-5,
        "snared: {snared_speed}",
    );

    set_aura_speed(app.world_mut(), player, 1.0);

    let input2 = make_input(1.0, 0.0, 0.0);
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
    let restored_speed = app.world().entity(player).get::<MovementSpeed>().unwrap().0;
    assert!(
        (restored_speed - DEFAULT_RUN_SPEED).abs() < 1e-5,
        "restored: {restored_speed}, expected {DEFAULT_RUN_SPEED}",
    );
}

#[test]
fn root_expires_movement_resumes() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    set_aura_speed(app.world_mut(), player, 0.00);

    let input = make_input(1.0, 0.0, 0.0);
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    assert_eq!(app.world().entity(player).get::<Position>().unwrap().x, 0.0);
    assert_eq!(
        app.world().entity(player).get::<MovementSpeed>().unwrap().0,
        0.0
    );

    set_aura_speed(app.world_mut(), player, 1.0);

    let input2 = make_input(1.0, 0.0, 0.0);
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
    let mut app = setup_app_with_time();
    let normal = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let snared = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    set_aura_speed(app.world_mut(), snared, 0.50);

    let jump = PlayerInput {
        facing_yaw: 0.0,
        jumping: true,
        ..reporting(FAR_X, make_input(1.0, 0.0, 0.0))
    };
    let normal_input = jump.clone();
    let snared_input = jump;

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &normal_input,
                normal,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
            apply_movement_input(
                &snared_input,
                snared,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });

    let normal_x = app.world().entity(normal).get::<Position>().unwrap().x;
    let snared_x = app.world().entity(snared).get::<Position>().unwrap().x;

    assert!(normal_x > 0.0, "normal moved forward");
    assert!(snared_x > 0.0, "snared moved forward");
    assert!(
        (snared_x - normal_x * 0.5).abs() < 1e-5,
        "snared horizontal {snared_x} should be 50% of normal {normal_x}",
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
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &input,
                casting_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
            apply_movement_input(
                &input2,
                cwm_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });

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
        swimming: true,
        flying: false,
        ..reporting(FAR_X, make_input(1.0, 0.0, 0.0))
    };
    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });
    let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
    let pos = *app.world().entity(player).get::<Position>().unwrap();

    assert!(
        (speed.0 - DEFAULT_SWIM_SPEED).abs() < 1e-5,
        "swim speed {}, expected {DEFAULT_SWIM_SPEED}",
        speed.0,
    );
    assert!(speed.0 < DEFAULT_RUN_SPEED, "swim slower than run");
    assert!(speed.0 > DEFAULT_WALK_SPEED, "swim faster than walk");

    let expected_x = DEFAULT_SWIM_SPEED * MAX_INPUT_STEP_SECS;
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
    let input = reporting([5.0, 5.0, 5.0], make_input(0.0, 0.0, 0.0));

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None);
        });

    let pos = *app.world().entity(player).get::<Position>().unwrap();
    assert_eq!(pos.x, 5.0);
    assert_eq!(pos.y, 5.0);
    assert_eq!(pos.z, 5.0);
}

#[test]
fn turn_in_place_reports_replicated_state_change() {
    let mut app = setup_app_with_time();
    let player = spawn_player(app.world_mut(), 5.0, 5.0, 5.0);
    let input = PlayerInput {
        facing_yaw: std::f32::consts::FRAC_PI_2,
        running: false,
        ..reporting([5.0, 5.0, 5.0], make_input(0.0, 0.0, 0.0))
    };

    let changed = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(&input, player, &mut q, &mut MovementClocks::default(), None)
        })
        .expect("movement system should run");

    assert!(
        changed.replicated_state_changed,
        "rotation/speed changes must wake replication"
    );
    assert!(!changed.position_changed);
    assert_eq!(app.world().entity(player).get::<Position>().unwrap().x, 5.0);
    assert_eq!(
        app.world().entity(player).get::<Rotation>().unwrap().y,
        std::f32::consts::FRAC_PI_2,
    );
    assert_eq!(
        app.world().entity(player).get::<MovementSpeed>().unwrap().0,
        DEFAULT_WALK_SPEED,
    );
}

#[test]
fn server_normalizes_direction_preventing_speed_hack() {
    let mut app = setup_app_with_time();
    let normal_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let hack_player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);

    let normal_input = reporting(FAR_X, make_input(1.0, 0.0, 0.0));
    let hack_input = reporting([100.0, 0.0, 0.0], make_input(10.0, 0.0, 0.0));

    let _ = app
        .world_mut()
        .run_system_once(move |mut q: Query<PlayerMovement, With<Player>>| {
            apply_movement_input(
                &normal_input,
                normal_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
            apply_movement_input(
                &hack_input,
                hack_player,
                &mut q,
                &mut MovementClocks::default(),
                None,
            );
        });

    let normal_pos = *app.world().entity(normal_player).get::<Position>().unwrap();
    let hack_pos = *app.world().entity(hack_player).get::<Position>().unwrap();
    assert!(
        (normal_pos.x - hack_pos.x).abs() < 1e-6,
        "normal={}, hack={} — should match",
        normal_pos.x,
        hack_pos.x,
    );
}

fn component_last_changed<T: Component>(
    app: &App,
    entity: Entity,
) -> bevy::ecs::change_detection::Tick {
    app.world()
        .entity(entity)
        .get_ref::<T>()
        .unwrap()
        .last_changed()
}

#[test]
fn grounded_gravity_does_not_dirty_unchanged_player_state() {
    let mut app = setup_app_with_time();
    app.insert_resource(crate::terrain_height::test_helpers::flat_ground(
        100.0, 100.0, 50.0,
    ));
    let player = spawn_player(app.world_mut(), 90.0, 50.0, -90.0);
    app.update();

    let position_tick = component_last_changed::<Position>(&app, player);
    let velocity_tick = component_last_changed::<VerticalVelocity>(&app, player);
    let tracker_tick = component_last_changed::<FallTracker>(&app, player);
    let health_tick = component_last_changed::<Health>(&app, player);

    app.world_mut()
        .run_system_once(apply_terrain_gravity)
        .unwrap();

    assert_eq!(
        component_last_changed::<Position>(&app, player),
        position_tick
    );
    assert_eq!(
        component_last_changed::<VerticalVelocity>(&app, player),
        velocity_tick
    );
    assert_eq!(
        component_last_changed::<FallTracker>(&app, player),
        tracker_tick
    );
    assert_eq!(component_last_changed::<Health>(&app, player), health_tick);
}

#[test]
fn gravity_system_applies_real_falling_state_changes_only() {
    let mut app = setup_app_with_time();
    app.insert_resource(crate::terrain_height::test_helpers::flat_ground(
        100.0, 100.0, 50.0,
    ));
    let player = spawn_player(app.world_mut(), 90.0, 60.0, -90.0);
    app.update();

    let position_tick = component_last_changed::<Position>(&app, player);
    let tracker_tick = component_last_changed::<FallTracker>(&app, player);
    let health_tick = component_last_changed::<Health>(&app, player);

    app.world_mut()
        .run_system_once(apply_terrain_gravity)
        .unwrap();

    let position = app.world().entity(player).get::<Position>().unwrap();
    assert!(position.y < 60.0);
    assert_ne!(
        component_last_changed::<Position>(&app, player),
        position_tick
    );
    assert_ne!(
        component_last_changed::<FallTracker>(&app, player),
        tracker_tick
    );
    assert_eq!(component_last_changed::<Health>(&app, player), health_tick);
}

#[test]
fn terrain_within_step_reach_above_the_feet_lifts_the_player() {
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, 50.0);
    let mut pos = Position {
        x: 90.0,
        y: 48.6,
        z: -90.0,
    };
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        0.05,
    );
    assert!((pos.y - 50.0).abs() < 0.01);
}

/// Shared ground rule: a surface higher than the feet plus the step reach is a
/// ceiling, so a player 20 yd under the terrain falls instead of popping up.
#[test]
fn terrain_beyond_step_reach_above_the_feet_does_not_lift_the_player() {
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, 50.0);
    let mut pos = Position {
        x: 90.0,
        y: 30.0,
        z: -90.0,
    };
    let dt = 0.05;
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        dt,
    );
    assert!((pos.y - (30.0 - GRAVITY * dt)).abs() < 0.01, "y {}", pos.y);
}

#[test]
fn terrain_applies_gravity() {
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, 50.0);
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
    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        dt,
    );
    let expected = 60.0 - GRAVITY * dt;
    assert!((pos.y - expected).abs() < 0.01);
}

#[test]
fn gravity_accumulates_until_landing() {
    let ground_y = 50.0;
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, ground_y);
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

    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        dt,
    );
    let after_one = 60.0 - GRAVITY * dt;
    assert!((pos.y - after_one).abs() < 0.01);

    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        dt,
    );
    let after_two = after_one - GRAVITY * dt;
    assert!((pos.y - after_two).abs() < 0.01);
    assert!(pos.y > ground_y, "still airborne");

    for _ in 0..200 {
        probe_and_clamp(
            &mut map,
            &mut pos,
            &mut vel,
            &mut fall_tracker,
            &mut health,
            dt,
        );
    }
    assert!(
        (pos.y - ground_y).abs() < 0.01,
        "landed at ground: y={}, ground={ground_y}",
        pos.y,
    );

    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        dt,
    );
    assert!((pos.y - ground_y).abs() < 0.01);
}

/// The server falls at `GRAVITY * dt` per tick and leaves a player resting up to
/// `GROUND_SNAP_THRESHOLD` above the ground, and WMO floor heights carry float
/// noise from one probe to the next. Coming to rest a hair above the floor is a
/// landing: the 30 yd fall deals its damage, and a later 2 yd drop is judged
/// from the floor.
#[test]
fn a_fall_that_comes_to_rest_a_hair_above_the_floor_lands() {
    let dt = 0.05;
    let mut pos = Position {
        x: 0.0,
        y: 50.0 + 31.0 * GRAVITY * dt + 0.05,
        z: 0.0,
    };
    let mut vel = VerticalVelocity::default();
    let mut fall_tracker = FallTracker::grounded(pos.y);
    let mut health = Health {
        current: 100.0,
        max: 100.0,
    };
    let mut settle = |ground: f32, pos: &mut Position, health: &mut Health| {
        for _ in 0..40 {
            clamp_to_ground(
                Entity::PLACEHOLDER,
                (GroundProbe::Supported(ground), None),
                pos,
                &mut vel,
                &mut fall_tracker,
                health,
                dt,
                FallAuras::NONE,
            );
        }
    };

    settle(50.0, &mut pos, &mut health);
    let after_fall = health.current;
    settle(49.99999, &mut pos, &mut health);
    settle(48.0, &mut pos, &mut health);

    assert!(after_fall < 100.0, "the 30 yd fall deals damage");
    assert!((pos.y - 48.0).abs() <= GROUND_SNAP_THRESHOLD, "y {}", pos.y);
    assert_eq!(health.current, after_fall, "a 2 yd drop deals no damage");
}

#[test]
fn terrain_on_ground_stays() {
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, 50.0);
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
    probe_and_clamp(
        &mut map,
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
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, ground_y);
    let dt = 0.05;

    let mut below = Position {
        x: 90.0,
        y: 48.6,
        z: -90.0,
    };
    let mut below_vel = VerticalVelocity::default();
    let mut below_fall = FallTracker::grounded(below.y);
    let mut below_health = Health {
        current: 100.0,
        max: 100.0,
    };
    probe_and_clamp(
        &mut map,
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
    probe_and_clamp(
        &mut map,
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
    probe_and_clamp(
        &mut map,
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
    probe_and_clamp(
        &mut map,
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
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, ground_y);
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

    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        dt,
    );
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
        probe_and_clamp(
            &mut map,
            &mut pos,
            &mut vel,
            &mut fall_tracker,
            &mut health,
            dt,
        );
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
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, ground_y);
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
        probe_and_clamp(
            &mut map,
            &mut pos,
            &mut vel,
            &mut fall_tracker,
            &mut health,
            dt,
        );
        if pos.y <= ground_y + 0.01 && vel.0 == 0.0 {
            break;
        }
    }

    assert_eq!(health.current, 100.0);
}

#[test]
fn long_fall_deals_damage_on_landing_once() {
    let ground_y = 50.0;
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, ground_y);
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
        probe_and_clamp(
            &mut map,
            &mut pos,
            &mut vel,
            &mut fall_tracker,
            &mut health,
            dt,
        );
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
    probe_and_clamp(
        &mut map,
        &mut pos,
        &mut vel,
        &mut fall_tracker,
        &mut health,
        dt,
    );
    assert_eq!(
        health.current, landed_health,
        "damage should not repeat after landing"
    );
}

#[test]
fn lethal_fall_reduces_health_to_zero() {
    let ground_y = 50.0;
    let mut map = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, ground_y);
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
        probe_and_clamp(
            &mut map,
            &mut pos,
            &mut vel,
            &mut fall_tracker,
            &mut health,
            dt,
        );
        if pos.y <= ground_y + 0.01 && vel.0 == 0.0 {
            break;
        }
    }

    assert_eq!(health.current, 0.0);
}

#[test]
fn a_player_the_server_flies_is_left_out_of_gravity() {
    let mut app = setup_app_with_time();
    app.insert_resource(crate::terrain_height::test_helpers::flat_ground(
        100.0, 100.0, 50.0,
    ));
    let player = spawn_player(app.world_mut(), 90.0, 60.0, -90.0);
    app.world_mut()
        .get_mut::<shared::components::MovementControl>(player)
        .unwrap()
        .controlled = true;
    app.update();

    app.world_mut()
        .run_system_once(apply_terrain_gravity)
        .unwrap();

    assert_eq!(app.world().get::<Position>(player).unwrap().y, 60.0);
}

#[test]
fn aura_world_safe_fall_1860_reduces_landing_height_seventeen_yards() {
    let mut app = setup_app_with_time();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(crate::terrain_height::test_helpers::flat_ground(
        100.0, 100.0, 50.0,
    ));
    let player = spawn_player(app.world_mut(), 90.0, 80.0, -90.0);
    let auras = crate::auras::world_xp_tests::native_auras(1860);
    assert_eq!(auras.modifier(144), 17.0);
    app.world_mut().entity_mut(player).insert(auras);
    for _ in 0..150 {
        app.update();
        app.world_mut()
            .run_system_once(apply_terrain_gravity)
            .unwrap();
    }
    assert!((app.world().get::<Position>(player).unwrap().y - 50.0).abs() < 0.001);
    let expected = 100.0 * (1.0 - shared::movement::fall_damage_percent(13.0));
    assert!((app.world().get::<Health>(player).unwrap().current - expected).abs() < 0.001);
}
