//! `PlayerMotion`: the Retail `MovementFlags` each applied `PlayerInput` sets, which other
//! clients animate the player from.
use super::receiver_rate::{deliver_and_tick, receiver_app};
use super::*;
use shared::components::PlayerMotion;
use shared::protocol::PlayerInput;

/// A facing off the world axes, so a wrong axis cannot pass.
const YAW: f32 = 0.6;

/// The wire direction the client sends for its animation direction
/// (game-engine `movement_to_direction`): forward, or its right `(-forward.z, forward.x)`.
fn wire_direction(forward: f32, right: f32) -> [f32; 3] {
    let (sin, cos) = YAW.sin_cos();
    [
        sin * forward - cos * right,
        0.0,
        cos * forward + sin * right,
    ]
}

fn input(direction: [f32; 3]) -> PlayerInput {
    PlayerInput {
        transport: None,
        direction,
        facing_yaw: YAW,
        jumping: false,
        running: true,
        swimming: false,
        flying: false,
        position: [0.0; 3],
        epoch: 0,
    }
}

fn motion(app: &App, player: Entity) -> PlayerMotion {
    *app.world().get::<PlayerMotion>(player).unwrap()
}

/// Input receive → height judgement → gravity → replicated motion, including ticks
/// with no input. Walking across the single fixture chunk's edge removes support.
#[test]
fn player_motion_airborne_walkoff_and_touchdown_replicate_without_jump() {
    use super::client_height::{movement_app, tick};
    let ground = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, 50.0);
    let (x, y, z) = crate::terrain_height::wow_to_bevy(100.0 - 100.0 / 3.0 + 0.2, 90.0, 50.0);
    let (mut app, player, connection) = movement_app(ground, Position { x, y, z });
    tick(&mut app, connection, vec![]);
    assert_eq!(motion(&app, player), PlayerMotion(0));
    let sent = PlayerInput {
        transport: None,
        direction: [-1.0, 0.0, 0.0],
        facing_yaw: -std::f32::consts::FRAC_PI_2,
        position: [x - 0.35, y, z],
        ..input([0.0; 3])
    };
    tick(&mut app, connection, vec![sent]);
    assert!(motion(&app, player).contains(PlayerMotion::FORWARD | PlayerMotion::FALLING));
    assert!(!motion(&app, player).contains(PlayerMotion::JUMP_STARTED));
    tick(&mut app, connection, vec![]);
    assert!(motion(&app, player).contains(PlayerMotion::FALLING));
    // Terrain corners identify authored 100/3-yard grid cells: the lower surface
    // must occupy the adjacent cell the player walked into, not the original one.
    app.insert_resource(crate::terrain_height::test_helpers::flat_ground(
        100.0 - 100.0 / 3.0,
        100.0,
        48.0,
    ));
    let pos = *app.world().get::<Position>(player).unwrap();
    assert_eq!(
        app.world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .ground_at(0, Vec3::new(pos.x, pos.y, pos.z)),
        crate::ground::GroundProbe::Supported(48.0)
    );
    tick(&mut app, connection, vec![]);
    let landed = app.world().get::<Position>(player).unwrap();
    assert!((landed.y - 48.0).abs() <= GROUND_SNAP_THRESHOLD);
    assert!(!app.world().get::<FallTracker>(player).unwrap().airborne);
    assert_eq!(motion(&app, player), PlayerMotion(PlayerMotion::FORWARD));
    for _ in 0..9 {
        tick(&mut app, connection, vec![]);
    }
    assert_eq!(motion(&app, player), PlayerMotion(PlayerMotion::FORWARD));
}

#[test]
fn player_motion_airborne_jump_keeps_origin_until_touchdown() {
    use super::client_height::{flat_app, flat_spot, tick};
    let (mut app, player, connection) = flat_app(50.0);
    let start = flat_spot(51.5);
    let sent = PlayerInput {
        jumping: true,
        position: [start.x, start.y, start.z],
        ..input([0.0; 3])
    };
    tick(&mut app, connection, vec![sent]);
    assert!(motion(&app, player).contains(PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED));
    // No jump opcode/report on subsequent ticks: airborne origin must persist.
    tick(&mut app, connection, vec![]);
    assert!(motion(&app, player).contains(PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED));
    for _ in 0..10 {
        tick(&mut app, connection, vec![]);
    }
    assert_eq!(motion(&app, player), PlayerMotion(0));
}

/// The server owns aura409's bit39; client input must not erase that ability.
#[test]
fn toy_coverage_turn_falling_owner_bit_survives_real_input_receive_and_stop() {
    const CAN_TURN_WHILE_FALLING: u64 = 0x0000_0080_0000_0000;
    let (mut app, player, connection) = receiver_app();
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerMotion(CAN_TURN_WHILE_FALLING));
    deliver_and_tick(&mut app, connection, vec![input(wire_direction(1.0, 0.0))]);
    assert!(motion(&app, player).contains(CAN_TURN_WHILE_FALLING | PlayerMotion::FORWARD));
    deliver_and_tick(&mut app, connection, vec![input([0.0; 3])]);
    assert_eq!(motion(&app, player), PlayerMotion(CAN_TURN_WHILE_FALLING));
}

#[test]
fn spawned_player_stands_still() {
    let (app, player, _) = receiver_app();
    assert_eq!(motion(&app, player), PlayerMotion(0));
}

#[test]
fn inputs_set_forward_backward_strafe_walk_swim_and_jump_flags() {
    let cases = [
        (input(wire_direction(1.0, 0.0)), PlayerMotion::FORWARD),
        (
            PlayerInput {
                running: false,
                ..input(wire_direction(1.0, 0.0))
            },
            PlayerMotion::FORWARD | PlayerMotion::WALKING,
        ),
        (input(wire_direction(-1.0, 0.0)), PlayerMotion::BACKWARD),
        (input(wire_direction(0.0, -1.0)), PlayerMotion::STRAFE_LEFT),
        (input(wire_direction(0.0, 1.0)), PlayerMotion::STRAFE_RIGHT),
        (
            PlayerInput {
                swimming: true,
                flying: false,
                ..input(wire_direction(0.0, 1.0))
            },
            PlayerMotion::STRAFE_RIGHT | PlayerMotion::SWIMMING,
        ),
        (
            PlayerInput {
                jumping: true,
                ..input(wire_direction(1.0, 0.0))
            },
            PlayerMotion::FORWARD | PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED,
        ),
        (
            PlayerInput {
                jumping: true,
                ..input([0.0; 3])
            },
            PlayerMotion::FALLING | PlayerMotion::JUMP_STARTED,
        ),
    ];
    for (sent, expected) in cases {
        let (mut app, player, connection) = receiver_app();
        deliver_and_tick(&mut app, connection, vec![sent.clone()]);
        assert_eq!(motion(&app, player), PlayerMotion(expected), "{sent:?}");
    }
}

/// Releasing the keys sends one stop input: no direction, no jump. It clears the moving
/// flags and keeps the mode flags (walk, swim) as Retail's stop does.
#[test]
fn the_stop_input_clears_direction_and_falling() {
    let (mut app, player, connection) = receiver_app();
    let walking_jump = PlayerInput {
        running: false,
        jumping: true,
        ..input(wire_direction(0.0, -1.0))
    };
    deliver_and_tick(&mut app, connection, vec![walking_jump]);
    assert_eq!(
        motion(&app, player),
        PlayerMotion(
            PlayerMotion::STRAFE_LEFT
                | PlayerMotion::WALKING
                | PlayerMotion::FALLING
                | PlayerMotion::JUMP_STARTED
        )
    );
    let stop = PlayerInput {
        running: false,
        ..input([0.0; 3])
    };
    deliver_and_tick(&mut app, connection, vec![stop]);
    assert_eq!(motion(&app, player), PlayerMotion(PlayerMotion::WALKING));

    let (mut app, player, connection) = receiver_app();
    let swim = PlayerInput {
        swimming: true,
        flying: false,
        ..input(wire_direction(1.0, 0.0))
    };
    deliver_and_tick(&mut app, connection, vec![swim.clone()]);
    let stop = PlayerInput {
        direction: [0.0; 3],
        ..swim
    };
    deliver_and_tick(&mut app, connection, vec![stop]);
    assert_eq!(motion(&app, player), PlayerMotion(PlayerMotion::SWIMMING));
}

/// The flags replicate on change only: repeated identical inputs leave the component
/// unchanged, so no update is sent.
#[test]
fn repeated_identical_inputs_do_not_change_player_motion() {
    let (mut app, player, connection) = receiver_app();
    let forward = input(wire_direction(1.0, 0.0));
    deliver_and_tick(&mut app, connection, vec![forward.clone()]);
    let changed = |app: &App| {
        app.world()
            .entity(player)
            .get_ref::<PlayerMotion>()
            .unwrap()
            .last_changed()
    };
    let first = changed(&app);
    app.update();
    deliver_and_tick(&mut app, connection, vec![forward.clone(), forward]);
    assert_eq!(changed(&app), first);
    assert_eq!(motion(&app, player), PlayerMotion(PlayerMotion::FORWARD));
}

/// A cast that roots the caster stops it (speed 0): it no longer moves forward.
#[test]
fn a_blocking_cast_clears_the_moving_flags() {
    let (mut app, player, connection) = receiver_app();
    deliver_and_tick(&mut app, connection, vec![input(wire_direction(1.0, 0.0))]);
    app.world_mut()
        .entity_mut(player)
        .insert(shared::casting::CastState {
            spell_name: "Fireball".into(),
            spell_id: 133,
            target: 0,
            duration: 3.5,
            elapsed: 0.0,
            cast_type: shared::casting::CastType::Normal,
            interruptible: true,
            pushback_count: 0,
            channel_tick_interval: 0.0,
            channel_tick_timer: 0.0,
            cast_while_moving: false,
        });
    deliver_and_tick(&mut app, connection, vec![input(wire_direction(1.0, 0.0))]);
    assert_eq!(motion(&app, player), PlayerMotion(0));
}

/// A dead player's input is dropped; its body stops moving for other clients.
#[test]
fn a_corpse_input_stops_the_player() {
    let (mut app, player, connection) = receiver_app();
    deliver_and_tick(&mut app, connection, vec![input(wire_direction(1.0, 0.0))]);
    app.world_mut()
        .entity_mut(player)
        .insert(crate::death::PlayerDeathState {
            state: DeathState::Dead,
            ..Default::default()
        });
    deliver_and_tick(&mut app, connection, vec![input(wire_direction(1.0, 0.0))]);
    assert_eq!(motion(&app, player), PlayerMotion(0));
}
