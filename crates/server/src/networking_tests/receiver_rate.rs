use super::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, PeerId};
use shared::protocol::InputChannel;
use std::time::Duration;

fn receiver_app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<PlayerInput>();
    app.init_resource::<crate::friends::AutoAfkScheduler>();
    app.add_systems(FixedUpdate, process_player_inputs);
    app.finish();
    app.cleanup();
    let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            PlayerLink(player),
            MessageSender::<PlayerInput>::default(),
            MessageReceiver::<PlayerInput>::default(),
        ))
        .id();
    (app, player, connection)
}

fn deliver_and_tick(app: &mut App, connection: Entity, inputs: Vec<PlayerInput>) {
    let count = inputs.len();
    for input in inputs {
        app.world_mut()
            .get_mut::<MessageSender<PlayerInput>>(connection)
            .unwrap()
            .send::<InputChannel>(input);
    }
    // Exercise Lightyear's real send/receive path before the production fixed system.
    // Host delivery runs in Last, after clearing the previous receive buffer.
    app.world_mut().run_schedule(Last);
    assert_eq!(
        app.world()
            .get::<MessageReceiver<PlayerInput>>(connection)
            .unwrap()
            .num_messages(),
        count
    );
    let mut time = Time::<()>::default();
    time.advance_by(Duration::from_millis(50));
    app.insert_resource(time);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(
        !app.world()
            .get::<MessageReceiver<PlayerInput>>(connection)
            .unwrap()
            .has_messages()
    );
}

fn assert_position(app: &App, player: Entity, expected: [f32; 3]) {
    let position = app.world().get::<Position>(player).unwrap();
    for (actual, expected) in [position.x, position.y, position.z]
        .into_iter()
        .zip(expected)
    {
        assert!(
            (actual - expected).abs() < 1e-5,
            "position {position:?}, expected coordinate {expected}"
        );
    }
}

/// A 7 yd/s run for one 20 fps frame.
const RUN_STEP: f32 = DEFAULT_RUN_SPEED * TEST_STEP_SECS;

#[test]
fn each_input_moves_the_player_for_the_time_it_covers() {
    // A 60 fps client sends three inputs per 50 ms server tick; each moves its own frame.
    for count in [1, 3] {
        let (mut app, player, connection) = receiver_app();
        deliver_and_tick(
            &mut app,
            connection,
            (0..count).map(|_| make_input(0.0, 0.0, 1.0)).collect(),
        );
        assert_position(&app, player, [0.0, 0.0, RUN_STEP * count as f32]);
        // A drained receiver must not replay movement on the next fixed tick.
        app.world_mut().run_schedule(FixedUpdate);
        assert_position(&app, player, [0.0, 0.0, RUN_STEP * count as f32]);
    }
}

#[test]
fn a_slow_client_frame_moves_the_player_as_far_as_the_client_moved() {
    // A 5 fps client sends one input per 200 ms frame: the server moves 200 ms, not one
    // 50 ms tick.
    let (mut app, player, connection) = receiver_app();
    let mut input = make_input(0.0, 0.0, 1.0);
    input.elapsed_secs = 0.2;
    deliver_and_tick(&mut app, connection, vec![input]);
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.2]);
}

#[test]
fn one_input_moves_at_most_a_quarter_second() {
    let (mut app, player, connection) = receiver_app();
    let mut input = make_input(0.0, 0.0, 1.0);
    input.elapsed_secs = 3.0;
    deliver_and_tick(&mut app, connection, vec![input]);
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.25]);
}

#[test]
fn inputs_never_move_a_player_further_than_server_time_allows() {
    let (mut app, player, connection) = receiver_app();
    // Twenty 50 ms inputs in one tick: only the half-second bank moves.
    deliver_and_tick(
        &mut app,
        connection,
        (0..20).map(|_| make_input(0.0, 0.0, 1.0)).collect(),
    );
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.5]);
    // The next tick refills one tick (50 ms), no matter how much the client claims.
    deliver_and_tick(
        &mut app,
        connection,
        (0..20).map(|_| make_input(0.0, 0.0, 1.0)).collect(),
    );
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.55]);
}

#[test]
fn negative_or_nan_elapsed_time_does_not_move_the_player() {
    let (mut app, player, connection) = receiver_app();
    let mut backwards = make_input(0.0, 0.0, 1.0);
    backwards.elapsed_secs = -1.0;
    let mut nan = make_input(0.0, 0.0, 1.0);
    nan.elapsed_secs = f32::NAN;
    deliver_and_tick(&mut app, connection, vec![backwards, nan]);
    assert_position(&app, player, [0.0; 3]);
}

#[test]
fn inputs_apply_in_order_and_the_latest_facing_and_walk_state_remain() {
    let (mut app, player, connection) = receiver_app();
    let mut latest = make_input(1.0, 0.0, 0.0);
    latest.running = false;
    deliver_and_tick(
        &mut app,
        connection,
        vec![
            make_input(0.0, 0.0, 1.0),
            make_input(-1.0, 0.0, 0.0),
            latest,
        ],
    );
    let walk_step = DEFAULT_WALK_SPEED * TEST_STEP_SECS;
    assert_position(&app, player, [walk_step - RUN_STEP, 0.0, RUN_STEP]);
    assert_eq!(
        app.world().get::<MovementSpeed>(player).unwrap().0,
        DEFAULT_WALK_SPEED
    );
    assert_eq!(
        app.world().get::<Rotation>(player).unwrap().y,
        std::f32::consts::FRAC_PI_2
    );
}

#[test]
fn an_early_jump_survives_a_later_stopped_input() {
    let (mut app, player, connection) = receiver_app();
    let mut jump = make_input(0.0, 0.0, 1.0);
    jump.jumping = true;
    deliver_and_tick(&mut app, connection, vec![jump, make_input(0.0, 0.0, 0.0)]);
    assert_position(&app, player, [0.0, 0.0, RUN_STEP]);
    assert_eq!(
        app.world().get::<VerticalVelocity>(player).unwrap().0,
        shared::movement::JUMP_IMPULSE
    );
}

#[test]
fn an_early_swimming_input_dismounts_and_the_latest_running_controls_speed() {
    let (mut app, player, connection) = receiver_app();
    app.world_mut().entity_mut(player).insert(Mounted {
        mount_display_id: 123,
    });
    let mut swim = make_input(0.0, 0.0, 1.0);
    swim.swimming = true;
    deliver_and_tick(&mut app, connection, vec![swim, make_input(0.0, 0.0, 1.0)]);
    assert_position(
        &app,
        player,
        [0.0, 0.0, DEFAULT_SWIM_SPEED * TEST_STEP_SECS + RUN_STEP],
    );
    assert_eq!(
        app.world().get::<MovementSpeed>(player).unwrap().0,
        DEFAULT_RUN_SPEED
    );
    assert!(app.world().get::<Mounted>(player).is_none());
}

#[test]
fn a_swimming_input_moves_vertically() {
    let (mut app, player, connection) = receiver_app();
    let mut swim = make_input(0.0, 1.0, 0.0);
    swim.swimming = true;
    deliver_and_tick(&mut app, connection, vec![make_input(0.0, 0.0, 1.0), swim]);
    assert_position(
        &app,
        player,
        [
            0.0,
            DEFAULT_SWIM_SPEED * shared::movement::STRAFE_MULTIPLIER * TEST_STEP_SECS,
            RUN_STEP,
        ],
    );
}

#[test]
fn input_of_a_player_the_server_flies_is_dropped() {
    let (mut app, player, connection) = receiver_app();
    app.world_mut()
        .get_mut::<shared::components::MovementControl>(player)
        .unwrap()
        .controlled = true;
    deliver_and_tick(&mut app, connection, vec![make_input(0.0, 0.0, 1.0)]);
    assert_position(&app, player, [0.0; 3]);
}
