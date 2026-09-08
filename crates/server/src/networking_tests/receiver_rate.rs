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

#[test]
fn receiver_rate_one_and_three_packets_move_one_fixed_step() {
    for count in [1, 3] {
        let (mut app, player, connection) = receiver_app();
        deliver_and_tick(
            &mut app,
            connection,
            (0..count).map(|_| make_input(0.0, 0.0, 1.0)).collect(),
        );
        assert_position(&app, player, [0.0, 0.0, 0.35]);
        // A drained receiver must not replay movement on the next fixed tick.
        app.world_mut().run_schedule(FixedUpdate);
        assert_position(&app, player, [0.0, 0.0, 0.35]);
    }
}

#[test]
fn receiver_rate_latest_direction_facing_and_walk_state_win() {
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
    assert_position(&app, player, [0.125, 0.0, 0.0]);
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
fn receiver_rate_early_jump_survives_latest_stopped_input() {
    let (mut app, player, connection) = receiver_app();
    let mut jump = make_input(0.0, 0.0, 1.0);
    jump.jumping = true;
    deliver_and_tick(&mut app, connection, vec![jump, make_input(0.0, 0.0, 0.0)]);
    assert_position(&app, player, [0.0; 3]);
    assert_eq!(
        app.world().get::<VerticalVelocity>(player).unwrap().0,
        shared::movement::JUMP_IMPULSE
    );
}

#[test]
fn receiver_rate_early_swimming_dismounts_but_latest_running_controls_speed() {
    let (mut app, player, connection) = receiver_app();
    app.world_mut().entity_mut(player).insert(Mounted {
        mount_display_id: 123,
    });
    let mut swim = make_input(0.0, 0.0, 1.0);
    swim.swimming = true;
    deliver_and_tick(&mut app, connection, vec![swim, make_input(0.0, 0.0, 1.0)]);
    assert_position(&app, player, [0.0, 0.0, 0.35]);
    assert_eq!(
        app.world().get::<MovementSpeed>(player).unwrap().0,
        DEFAULT_RUN_SPEED
    );
    assert!(app.world().get::<Mounted>(player).is_none());
}

#[test]
fn receiver_rate_latest_swimming_controls_translation() {
    let (mut app, player, connection) = receiver_app();
    let mut swim = make_input(0.0, 1.0, 0.0);
    swim.swimming = true;
    deliver_and_tick(&mut app, connection, vec![make_input(0.0, 0.0, 1.0), swim]);
    assert_position(
        &app,
        player,
        [
            0.0,
            DEFAULT_SWIM_SPEED * shared::movement::STRAFE_MULTIPLIER * 0.05,
            0.0,
        ],
    );
}
