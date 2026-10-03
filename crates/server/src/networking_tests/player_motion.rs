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
            PlayerMotion::FORWARD | PlayerMotion::FALLING,
        ),
        (
            PlayerInput {
                jumping: true,
                ..input([0.0; 3])
            },
            PlayerMotion::FALLING,
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
        PlayerMotion(PlayerMotion::STRAFE_LEFT | PlayerMotion::WALKING | PlayerMotion::FALLING)
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
