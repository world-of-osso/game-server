use super::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, PeerId};
use shared::protocol::InputChannel;
use std::time::Duration;

pub(super) fn receiver_app() -> (App, Entity, Entity) {
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

pub(super) fn deliver_and_tick(app: &mut App, connection: Entity, inputs: Vec<PlayerInput>) {
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

/// Run frames north from the origin, the `n`th reporting `n` run steps.
fn run_north(frames: std::ops::RangeInclusive<usize>) -> Vec<PlayerInput> {
    frames
        .map(|n| reporting([0.0, 0.0, RUN_STEP * n as f32], make_input(0.0, 0.0, 1.0)))
        .collect()
}

#[test]
fn compass_facing_rejects_old_input_then_allows_a_new_player_turn() {
    let (mut app, player, connection) = receiver_app();
    let mut falling = FallTracker::grounded(20.0);
    falling.airborne = true;
    falling.takeoff_y = 20.0;
    falling.air_secs = 0.7;
    falling.last_y = 18.0;
    app.world_mut().entity_mut(player).insert(falling);
    crate::networking::face_player(app.world_mut(), player, 1.25);
    let after = *app.world().get::<FallTracker>(player).unwrap();
    assert_eq!(after.airborne_state().unwrap().secs, 0.7);
    assert_eq!(after.takeoff_y, 20.0);
    assert_eq!(after.last_y, 18.0);
    assert_eq!(
        after.for_epoch(1, 18.0),
        after,
        "facing must not reset the fall next tick"
    );
    let old = reporting([0.0; 3], make_input(0.0, 0.0, 0.0));
    deliver_and_tick(&mut app, connection, vec![old.clone()]);
    assert_eq!(app.world().get::<Rotation>(player).unwrap().y, 1.25);
    assert_position(&app, player, [0.0; 3]);
    let adopted = PlayerInput {
        epoch: 1,
        facing_yaw: 2.25,
        ..old
    };
    deliver_and_tick(&mut app, connection, vec![adopted]);
    assert_eq!(app.world().get::<Rotation>(player).unwrap().y, 2.25);
    assert_position(&app, player, [0.0; 3]);
}

#[test]
fn each_input_moves_the_player_to_the_position_it_reports() {
    // A 60 fps client sends three inputs per 50 ms server tick; the last position stands.
    for count in [1, 3] {
        let (mut app, player, connection) = receiver_app();
        deliver_and_tick(&mut app, connection, run_north(1..=count));
        assert_position(&app, player, [0.0, 0.0, RUN_STEP * count as f32]);
        // A drained receiver must not replay movement on the next fixed tick.
        app.world_mut().run_schedule(FixedUpdate);
        assert_position(&app, player, [0.0, 0.0, RUN_STEP * count as f32]);
    }
}

#[test]
fn a_slow_client_frame_moves_the_player_as_far_as_the_client_moved() {
    // A 5 fps client sends one input per 200 ms frame: the server moves 200 ms of running,
    // not one 50 ms tick.
    let (mut app, player, connection) = receiver_app();
    let far = DEFAULT_RUN_SPEED * 0.2;
    let input = reporting([0.0, 0.0, far], make_input(0.0, 0.0, 1.0));
    deliver_and_tick(&mut app, connection, vec![input]);
    assert_position(&app, player, [0.0, 0.0, far]);
}

#[test]
fn one_input_moves_at_most_a_quarter_second() {
    let (mut app, player, connection) = receiver_app();
    let input = reporting([0.0, 0.0, 21.0], make_input(0.0, 0.0, 1.0));
    deliver_and_tick(&mut app, connection, vec![input]);
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.25]);
}

#[test]
fn inputs_never_move_a_player_further_than_server_time_allows() {
    let (mut app, player, connection) = receiver_app();
    // Twenty 50 ms frames in one tick: only the half-second bank moves.
    deliver_and_tick(&mut app, connection, run_north(1..=20));
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.5]);
    // The next tick refills one tick (50 ms), no matter how far the client reports.
    deliver_and_tick(&mut app, connection, run_north(21..=40));
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.55]);
}

#[test]
fn a_player_behind_its_client_catches_up_from_the_bank() {
    // A 0.4 s client hitch: its one input moves only a quarter second...
    let (mut app, player, connection) = receiver_app();
    let hitch = DEFAULT_RUN_SPEED * 0.4;
    let input = reporting([0.0, 0.0, hitch], make_input(0.0, 0.0, 1.0));
    deliver_and_tick(&mut app, connection, vec![input]);
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.25]);
    // ...and the next frame, reporting one more step, brings the player level with it.
    let next = reporting([0.0, 0.0, hitch + RUN_STEP], make_input(0.0, 0.0, 1.0));
    deliver_and_tick(&mut app, connection, vec![next]);
    assert_position(&app, player, [0.0, 0.0, hitch + RUN_STEP]);
}

#[test]
fn a_low_fps_client_run_ends_where_the_client_stopped() {
    // The 1.5 fps headless hunter (tamebeast live run): each 2/3 s frame reports 4.67 yd
    // more, the server ticks without input between frames, and the release sends one stop
    // at the final position. The server moved 7 yd of the client's 14 and stayed there.
    let (mut app, player, connection) = receiver_app();
    let frame_secs = 2.0 / 3.0;
    let mut reported = 0.0;
    for _frame in 0..3 {
        reported += DEFAULT_RUN_SPEED * frame_secs;
        let run = reporting([0.0, 0.0, reported], make_input(0.0, 0.0, 1.0));
        deliver_and_tick(&mut app, connection, vec![run]);
        for _tick in 1..13 {
            deliver_and_tick(&mut app, connection, Vec::new());
        }
    }
    let stop = reporting([0.0, 0.0, reported], make_input(0.0, 0.0, 0.0));
    deliver_and_tick(&mut app, connection, vec![stop]);
    for _tick in 0..20 {
        deliver_and_tick(&mut app, connection, Vec::new());
    }
    assert_position(&app, player, [0.0, 0.0, reported]);
}

#[test]
fn a_player_behind_its_client_moves_no_faster_than_its_speed() {
    // One report 21 yd away: the player walks toward it from the bank, at most a quarter
    // second per tick, then one tick's refill per tick.
    let (mut app, player, connection) = receiver_app();
    let input = reporting([0.0, 0.0, 21.0], make_input(0.0, 0.0, 1.0));
    deliver_and_tick(&mut app, connection, vec![input]);
    for _tick in 0..4 {
        deliver_and_tick(&mut app, connection, Vec::new());
    }
    // The half-second bank and four 50 ms refills.
    assert_position(&app, player, [0.0, 0.0, DEFAULT_RUN_SPEED * 0.7]);
}

#[test]
fn nan_or_infinite_reported_positions_do_not_move_the_player() {
    let (mut app, player, connection) = receiver_app();
    let nan = reporting([f32::NAN, 0.0, 1.0], make_input(0.0, 0.0, 1.0));
    let infinite = reporting([0.0, 0.0, f32::INFINITY], make_input(0.0, 0.0, 1.0));
    deliver_and_tick(&mut app, connection, vec![nan, infinite]);
    assert_position(&app, player, [0.0; 3]);
}

#[test]
fn passenger_seat_input_cannot_translate_or_turn_the_player() {
    let (mut app, player, connection) = receiver_app();
    app.world_mut()
        .entity_mut(player)
        .insert(shared::components::VehiclePassenger {
            driver: 99,
            seat_index: 1,
            seat_id: 2764,
        });
    app.world_mut()
        .get_mut::<shared::components::MovementControl>(player)
        .unwrap()
        .controlled = true;
    let before = *app
        .world()
        .get::<shared::components::Rotation>(player)
        .unwrap();
    deliver_and_tick(&mut app, connection, run_north(1..=5));
    assert_position(&app, player, [0.0; 3]);
    assert_eq!(
        app.world().get::<shared::components::Rotation>(player),
        Some(&before)
    );
}

#[test]
fn input_from_before_a_teleport_is_dropped() {
    let (mut app, player, connection) = receiver_app();
    app.world_mut()
        .get_mut::<shared::components::MovementControl>(player)
        .unwrap()
        .epoch = 1;
    // Sent before the client adopted the teleport: its position is the old place's.
    deliver_and_tick(&mut app, connection, run_north(1..=1));
    assert_position(&app, player, [0.0; 3]);
    let adopted = PlayerInput {
        epoch: 1,
        ..run_north(1..=1).remove(0)
    };
    deliver_and_tick(&mut app, connection, vec![adopted]);
    assert_position(&app, player, [0.0, 0.0, RUN_STEP]);
}

#[test]
fn inputs_apply_in_order_and_the_latest_facing_and_walk_state_remain() {
    let (mut app, player, connection) = receiver_app();
    let walk_step = DEFAULT_WALK_SPEED * TEST_STEP_SECS;
    let latest = PlayerInput {
        running: false,
        ..reporting(
            [walk_step - RUN_STEP, 0.0, RUN_STEP],
            make_input(1.0, 0.0, 0.0),
        )
    };
    deliver_and_tick(
        &mut app,
        connection,
        vec![
            make_input(0.0, 0.0, 1.0),
            reporting([-RUN_STEP, 0.0, RUN_STEP], make_input(-1.0, 0.0, 0.0)),
            latest,
        ],
    );
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

/// The client's physics owns a jump: the reported height is adopted
/// (networking_tests/client_height.rs) and the server starts no jump arc of its own.
#[test]
fn a_jumping_input_starts_no_server_jump_arc() {
    let (mut app, player, connection) = receiver_app();
    let mut jump = make_input(0.0, 0.0, 1.0);
    jump.jumping = true;
    let stopped = reporting(jump.position, make_input(0.0, 0.0, 0.0));
    deliver_and_tick(&mut app, connection, vec![jump, stopped]);
    assert_position(&app, player, [0.0, 0.0, RUN_STEP]);
    assert_eq!(app.world().get::<VerticalVelocity>(player).unwrap().0, 0.0);
}

#[test]
fn an_early_swimming_input_dismounts_and_the_latest_running_controls_speed() {
    let (mut app, player, connection) = receiver_app();
    app.world_mut().entity_mut(player).insert(Mounted {
        mount_display_id: 123,
        ..Default::default()
    });
    let swum = DEFAULT_SWIM_SPEED * TEST_STEP_SECS;
    let swim = PlayerInput {
        swimming: true,
        flying: false,
        ..reporting([0.0, 0.0, swum], make_input(0.0, 0.0, 1.0))
    };
    let run = reporting([0.0, 0.0, swum + RUN_STEP], make_input(0.0, 0.0, 1.0));
    deliver_and_tick(&mut app, connection, vec![swim, run]);
    assert_position(&app, player, [0.0, 0.0, swum + RUN_STEP]);
    assert_eq!(
        app.world().get::<MovementSpeed>(player).unwrap().0,
        DEFAULT_RUN_SPEED
    );
    assert!(app.world().get::<Mounted>(player).is_none());
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

#[test]
fn a_corpse_ignores_movement_input_and_a_ghost_moves() {
    let (mut app, player, connection) = receiver_app();
    app.world_mut()
        .entity_mut(player)
        .insert(crate::death::PlayerDeathState {
            state: DeathState::Dead,
            ..Default::default()
        });
    deliver_and_tick(&mut app, connection, vec![make_input(0.0, 0.0, 1.0)]);
    assert_position(&app, player, [0.0; 3]);

    app.world_mut()
        .get_mut::<crate::death::PlayerDeathState>(player)
        .unwrap()
        .state = DeathState::Ghost;
    deliver_and_tick(&mut app, connection, vec![make_input(0.0, 0.0, 1.0)]);
    assert_position(&app, player, [0.0, 0.0, RUN_STEP]);
}
