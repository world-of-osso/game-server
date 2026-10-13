//! The server adopts the height the client reports, within what swimming, the ground, a
//! jump and a fall allow (docs/specs/player-height.md), through Lightyear's receive path
//! and the fixed-tick input and gravity systems.
use super::*;
use crate::ground::GroundMap;
use crate::terrain_height::wow_to_bevy;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, PeerId};
use shared::movement::{GRAVITY, SWIM_DEPTH, SWIM_SPEED};
use shared::protocol::InputChannel;
use std::time::Duration;

/// `world/maps/azeroth/azeroth.wdt`.
const AZEROTH_WDT_FDID: u32 = 775_971;
/// The Godot swimming fixtures' deep-water spot, Azeroth 32,48 (Northshire lake), WoW X/Y.
const SWIM_SPOT: (f32, f32) = (-8558.0, -490.0);
/// Terrain and MH2O surface there (terrain_height_tests
/// `swim_spot_liquid_is_the_surface_the_client_floats_under`).
const SEABED: f32 = 140.04;
const SURFACE: f32 = 143.99;
/// The Godot client's frame step (60 fps).
pub(super) const FRAME_SECS: f32 = 1.0 / 60.0;
const TICK: Duration = Duration::from_millis(50);
/// Flat test ground: one chunk with corner (100, 100) at height 50.
pub(super) const FLAT: f32 = 50.0;

pub(super) fn movement_app(ground: GroundMap, spawn: Position) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: TICK,
    });
    app.register_message::<PlayerInput>();
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.init_resource::<crate::friends::AutoAfkScheduler>();
    app.insert_resource(ground);
    app.add_systems(
        FixedUpdate,
        (process_player_inputs, apply_terrain_gravity).chain(),
    );
    app.finish();
    app.cleanup();
    let player = spawn_player(app.world_mut(), spawn.x, spawn.y, spawn.z);
    app.world_mut()
        .entity_mut(player)
        .insert(shared::components::MovementControl::default());
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

fn swim_app(feet_y: f32) -> (App, Entity, Entity) {
    let ground = GroundMap::new(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        std::collections::HashMap::from([(0, AZEROTH_WDT_FDID)]),
    );
    movement_app(ground, swim_spot(feet_y))
}

pub(super) fn flat_app(feet_y: f32) -> (App, Entity, Entity) {
    let ground = crate::terrain_height::test_helpers::flat_ground(100.0, 100.0, FLAT);
    movement_app(ground, flat_spot(feet_y))
}

fn swim_spot(feet_y: f32) -> Position {
    let (x, y, z) = wow_to_bevy(SWIM_SPOT.0, SWIM_SPOT.1, feet_y);
    Position { x, y, z }
}

/// Inside the flat chunk (WoW 90, 90).
pub(super) fn flat_spot(feet_y: f32) -> Position {
    let (x, y, z) = wow_to_bevy(90.0, 90.0, feet_y);
    Position { x, y, z }
}

pub(super) fn tick(app: &mut App, connection: Entity, inputs: Vec<PlayerInput>) {
    for input in inputs {
        app.world_mut()
            .get_mut::<MessageSender<PlayerInput>>(connection)
            .unwrap()
            .send::<InputChannel>(input);
    }
    app.world_mut().run_schedule(Last);
    let mut time = Time::<()>::default();
    time.advance_by(TICK);
    app.insert_resource(time);
    app.world_mut().run_schedule(FixedUpdate);
}

pub(super) fn server_y(app: &App, player: Entity) -> f32 {
    app.world().get::<Position>(player).unwrap().y
}

/// What the Godot client sends for one frame at `client`: moving vertically only while
/// swimming (`network_input` with zero direction), or standing/jumping on land.
fn frame_input(client: &Position, swimming: bool, jumping: bool) -> PlayerInput {
    PlayerInput {
        transport: None,
        direction: [0.0; 3],
        facing_yaw: 0.0,
        jumping,
        running: true,
        swimming,
        flying: false,
        position: [client.x, client.y, client.z],
        epoch: 0,
    }
}

/// One server tick of three 60 fps client frames, each moving the client's feet by `step`.
fn frames(client: &mut Position, swimming: bool, step: impl Fn(f32) -> f32) -> Vec<PlayerInput> {
    (0..3)
        .map(|_| {
            client.y = step(client.y);
            frame_input(client, swimming, false)
        })
        .collect()
}

#[test]
fn a_swimmer_holding_ascend_rises_with_the_client_and_floats_at_the_surface() {
    let (mut app, player, connection) = swim_app(SEABED);
    let top = SURFACE - SWIM_DEPTH;
    let mut client = swim_spot(SEABED);

    for _ in 0..20 {
        let inputs = frames(&mut client, true, |y| {
            (y + SWIM_SPEED * FRAME_SECS).min(top)
        });
        tick(&mut app, connection, inputs);
        let gap = (server_y(&app, player) - client.y).abs();
        assert!(
            gap < 0.01,
            "server {} client {}",
            server_y(&app, player),
            client.y
        );
    }
    assert!((client.y - top).abs() < 1e-4);
    // Floating idle, the client sends nothing: no gravity pulls the swimmer to the seabed.
    for _ in 0..40 {
        tick(&mut app, connection, vec![]);
    }
    assert!((server_y(&app, player) - top).abs() < 0.01);
}

#[test]
fn a_swimmer_reported_above_the_float_height_is_held_at_it() {
    let (mut app, player, connection) = swim_app(SURFACE - SWIM_DEPTH - 0.2);
    let mut client = swim_spot(SURFACE + 5.0);

    tick(
        &mut app,
        connection,
        vec![frame_input(&client, true, false)],
    );

    let y = server_y(&app, player);
    assert!((y - (SURFACE - SWIM_DEPTH)).abs() < 0.01, "server {y}");
    client.y = SURFACE - SWIM_DEPTH - 0.5;
    tick(
        &mut app,
        connection,
        vec![frame_input(&client, true, false)],
    );
    let y = server_y(&app, player);
    assert!(
        (y - client.y).abs() < 0.01,
        "server {y} client {}",
        client.y
    );
}

#[test]
fn a_swimmer_rises_no_faster_than_swim_speed_over_the_movement_bank() {
    let (mut app, player, connection) = swim_app(SEABED);
    let client = swim_spot(SURFACE - SWIM_DEPTH);

    // Ten reports of the surface in one tick: the 0.5 s bank covers 2.36 of the 2.67 yd.
    tick(
        &mut app,
        connection,
        vec![frame_input(&client, true, false); 10],
    );

    let expected = SEABED + SWIM_SPEED * 0.5;
    let y = server_y(&app, player);
    assert!(
        (y - expected).abs() < 0.01,
        "server {y}, expected {expected}"
    );
}

#[test]
fn a_swimmer_reported_under_the_seabed_stays_on_it() {
    let (mut app, player, connection) = swim_app(SEABED + 0.5);
    let client = swim_spot(SEABED - 3.0);

    tick(
        &mut app,
        connection,
        vec![frame_input(&client, true, false)],
    );

    assert!(
        (server_y(&app, player) - SEABED).abs() < 0.01,
        "{}",
        server_y(&app, player)
    );
}

/// The Godot client's jump (`update_jump`: 7 yd/s up, shared `GRAVITY` down) from flat
/// ground, sent at 60 fps; the server follows it and it lands where it took off.
#[test]
fn a_jump_the_client_reports_is_the_server_height_through_the_arc() {
    let (mut app, player, connection) = flat_app(FLAT);
    let mut client = flat_spot(FLAT);
    let mut velocity = 7.0_f32;
    let mut peak = FLAT;

    // 7 yd/s up lands after 0.71 s: 16 ticks cover the arc and the landing.
    for _ in 0..16 {
        let inputs = (0..3)
            .map(|_| {
                velocity -= GRAVITY * FRAME_SECS;
                client.y = (client.y + velocity * FRAME_SECS).max(FLAT);
                frame_input(&client, false, client.y > FLAT)
            })
            .collect();
        tick(&mut app, connection, inputs);
        peak = peak.max(server_y(&app, player));
        let gap = (server_y(&app, player) - client.y).abs();
        assert!(
            gap < 0.01,
            "server {} client {}",
            server_y(&app, player),
            client.y
        );
    }
    assert!(peak > FLAT + 1.0, "peak {peak}");
    assert_eq!(server_y(&app, player), FLAT);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
}

#[test]
fn a_grounded_player_reported_high_above_the_ground_is_held_to_a_jump() {
    let (mut app, player, connection) = flat_app(FLAT);
    let client = flat_spot(FLAT + 20.0);

    tick(
        &mut app,
        connection,
        vec![frame_input(&client, false, true)],
    );

    let apex = shared::movement::JUMP_IMPULSE.powi(2) / (2.0 * GRAVITY);
    let y = server_y(&app, player);
    assert!(y <= FLAT + apex + 0.3, "server {y}");
}

#[test]
fn a_player_reported_under_the_ground_stands_on_it() {
    let (mut app, player, connection) = flat_app(FLAT);
    let client = flat_spot(FLAT - 5.0);

    tick(
        &mut app,
        connection,
        vec![frame_input(&client, false, false)],
    );

    assert_eq!(server_y(&app, player), FLAT);
}

/// A client that keeps reporting itself 1 yd over the ground (hovering) comes down within
/// a jump's time plus the movement bank.
#[test]
fn a_player_reported_hovering_comes_down_to_the_ground() {
    let (mut app, player, connection) = flat_app(FLAT);
    let client = flat_spot(FLAT + 1.0);

    for _ in 0..30 {
        tick(
            &mut app,
            connection,
            vec![frame_input(&client, false, false); 3],
        );
    }

    assert!(
        server_y(&app, player) < FLAT + 0.3,
        "server {}",
        server_y(&app, player)
    );
}

/// Walking off a 10 yd ledge and falling as the client reports it lands with the server's
/// fall damage.
#[test]
fn a_fall_the_client_reports_lands_with_fall_damage() {
    let (mut app, player, connection) = flat_app(FLAT + 10.0);
    let mut client = flat_spot(FLAT + 10.0);
    let mut velocity = 0.0_f32;
    // Standing on nothing at 60: the server has no ground there but the flat chunk under it.
    for _ in 0..20 {
        let inputs = (0..3)
            .map(|_| {
                velocity -= GRAVITY * FRAME_SECS;
                client.y = (client.y + velocity * FRAME_SECS).max(FLAT);
                frame_input(&client, false, false)
            })
            .collect();
        tick(&mut app, connection, inputs);
        let gap = (server_y(&app, player) - client.y).abs();
        assert!(
            gap < 0.05,
            "server {} client {}",
            server_y(&app, player),
            client.y
        );
    }
    let health = app.world().get::<Health>(player).unwrap().current;
    let expected = 100.0 * (1.0 - shared::movement::fall_damage_percent(10.0));
    assert!(
        (health - expected).abs() < 0.5,
        "health {health}, expected {expected}"
    );
}

#[test]
fn toyfx2_feather_fall_caps_server_and_reported_descent_and_negates_damage() {
    for report in [false, true] {
        let (mut app, player, connection) = flat_app(FLAT + 20.0);
        app.world_mut()
            .entity_mut(player)
            .insert(shared::components::FeatherFall(true));
        let mut previous = FLAT + 20.0;
        for _ in 0..90 {
            let inputs = if report {
                vec![frame_input(&flat_spot(FLAT), false, false); 3]
            } else {
                Vec::new()
            };
            tick(&mut app, connection, inputs);
            let next = server_y(&app, player);
            assert!(
                previous - next <= 7.0 * 0.05 + 0.001,
                "reported={report}, {previous}->{next}"
            );
            assert!(app.world().get::<VerticalVelocity>(player).unwrap().0 >= -7.0);
            previous = next;
        }
        assert!((server_y(&app, player) - FLAT).abs() < 0.3);
        assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
        // Removing the last feather aura restores normal fall damage and speed.
        app.world_mut()
            .entity_mut(player)
            .insert(shared::components::FeatherFall(false));
        app.world_mut().get_mut::<Position>(player).unwrap().y = FLAT + 20.0;
        app.world_mut()
            .get_mut::<FallTracker>(player)
            .unwrap()
            .restart_at(FLAT + 20.0);
        for _ in 0..60 {
            tick(&mut app, connection, vec![]);
        }
        assert!(app.world().get::<Health>(player).unwrap().current < 100.0);
    }
}

/// Falling 10 yd into the lake: gravity stops where the player swims, without fall damage.
#[test]
fn a_fall_into_deep_water_stops_swimming_without_fall_damage() {
    let (mut app, player, connection) = swim_app(SURFACE + 10.0);

    for _ in 0..40 {
        tick(&mut app, connection, vec![]);
    }

    let y = server_y(&app, player);
    assert!(
        y <= SURFACE - SWIM_DEPTH + 0.01 && y > SEABED + 1.0,
        "server {y}"
    );
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
}
