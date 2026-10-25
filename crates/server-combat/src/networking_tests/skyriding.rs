//! Skyriding: a player who can skyride (`PlayerMotion::CAN_ADV_FLY`, from the Skyriding
//! capability aura 406095's ADV_FLYING 446) moves to the position it reports as fast as
//! its energy allows (`shared::skyriding::speed_after`): diving gains speed, climbing
//! loses it, and no report outruns the bound. Steady flight keeps its fixed speed.
use super::client_height::{FLAT, FRAME_SECS, flat_app, flat_spot, server_y, tick};
use super::*;
use shared::components::PlayerMotion;
use shared::movement::FLIGHT_SPEED;
use shared::skyriding::{Glider, LAUNCH_SPEED, SKYRIDING, SURGE_FORWARD, SURGE_SPEED, max_climb};

/// Skyriding 406095: MOD_INCREASE_MOUNTED_SPEED 100%, MOD_INCREASE_MOUNTED_FLIGHT_SPEED 310%.
const SKYRIDING_GROUND: f32 = 2.0;
const SKYRIDING_FLYING: f32 = 4.1;
/// Mount Speed Mod: Standard Flying Mount 86459: MOD_INCREASE_MOUNTED_FLIGHT_SPEED 220%.
const STANDARD_FLYING: f32 = 3.2;

/// A skyrider whose takeoff the server granted: its Skyward Ascent cast left its
/// `SkyridingImpulse` (`crate::skyriding::flap`) on the ground.
fn skyrider(app: &mut App, player: Entity) {
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerMotion(
            PlayerMotion::CAN_FLY | PlayerMotion::CAN_ADV_FLY,
        ))
        .insert(AuraStatMods {
            movement_speed: SKYRIDING_GROUND,
            flight_speed: SKYRIDING_FLYING,
            ..Default::default()
        })
        .insert(crate::skyriding::SkyridingImpulse(LAUNCH_SPEED));
}

/// The server's bound at takeoff: the mounted run speed with the launch on top.
fn launch_bound() -> f32 {
    (7.0 * SKYRIDING_GROUND).hypot(LAUNCH_SPEED)
}

fn flight_input(client: Vec3, flying: bool) -> PlayerInput {
    PlayerInput {
        transport: None,
        direction: [0.0, 0.0, 1.0],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
        flying,
        position: client.to_array(),
        epoch: 0,
    }
}

fn server_position(app: &App, player: Entity) -> Vec3 {
    let position = app.world().get::<Position>(player).unwrap();
    Vec3::new(position.x, position.y, position.z)
}

fn spot() -> Vec3 {
    let spot = flat_spot(FLAT);
    Vec3::new(spot.x, spot.y, spot.z)
}

/// One server tick of three 60 fps frames of `glider` steering toward `pitch`; the
/// client's position after it.
fn glide(app: &mut App, connection: Entity, glider: &mut Glider, client: &mut Vec3, pitch: f32) {
    let inputs = (0..3)
        .map(|_| {
            *client += glider.step(&SKYRIDING, 0.0, Some(pitch), client.y - FLAT, FRAME_SECS);
            flight_input(*client, true)
        })
        .collect();
    tick(app, connection, inputs);
}

/// Launched 300 yards up (no ground in the way), so the dive has room to reach its speed.
#[test]
fn the_server_follows_a_skyriding_launch_dive_and_pull_up() {
    let (mut app, player, connection) = flat_app(FLAT + 300.0);
    skyrider(&mut app, player);
    let mut client = server_position(&app, player);
    let mut glider = Glider::launch(Vec3::ZERO);
    let mut top_speed = 0.0_f32;
    let mut worst_gap = 0.0_f32;
    let dive = -50.0_f32.to_radians();
    let pull_up = 20.0_f32.to_radians();
    for (pitch, ticks) in [(0.0, 40), (dive, 70), (pull_up, 30), (0.0, 30)] {
        for _ in 0..ticks {
            glide(&mut app, connection, &mut glider, &mut client, pitch);
            top_speed = top_speed.max(glider.speed());
            worst_gap = worst_gap.max(server_position(&app, player).distance(client));
        }
    }
    // The dive passes both the launch and the steady flight speed (4.1 × 7 = 28.7).
    assert!(top_speed > 50.0, "top speed {top_speed}");
    assert!(worst_gap < 0.05, "server fell {worst_gap} yards behind");
    let motion = *app.world().get::<PlayerMotion>(player).unwrap();
    assert!(motion.contains(PlayerMotion::FLYING | PlayerMotion::ADV_FLYING));
}

#[test]
fn a_climb_past_the_launch_energy_is_cut_short() {
    let (mut app, player, connection) = flat_app(FLAT);
    skyrider(&mut app, player);
    let mut client = spot();
    // Straight up at 40 yd/s for 2 s: 80 yards on a launch that climbs at most
    // 34.47² / (2 × 19.29) = 30.8.
    for _ in 0..40 {
        let inputs = (0..3)
            .map(|_| {
                client.y += 40.0 * FRAME_SECS;
                flight_input(client, true)
            })
            .collect();
        tick(&mut app, connection, inputs);
    }
    let climbed = server_y(&app, player) - FLAT;
    assert!(
        climbed <= max_climb(launch_bound()) + 0.01,
        "climbed {climbed}"
    );
    assert!(climbed > 25.0, "climbed {climbed}");
}

/// Without the Skyward Ascent cast the takeoff has only the mounted run speed (14 yd/s),
/// which climbs at most v²/2g ≈ 5.1 yards: the client's 31.5 yd/s launch is cut short.
#[test]
fn a_takeoff_the_server_never_granted_is_cut_short() {
    let (mut app, player, connection) = flat_app(FLAT);
    skyrider(&mut app, player);
    app.world_mut()
        .entity_mut(player)
        .remove::<crate::skyriding::SkyridingImpulse>();
    let mut client = spot();
    let mut glider = Glider::launch(Vec3::new(0.0, 0.0, 7.0 * SKYRIDING_GROUND));
    for _ in 0..20 {
        glide(&mut app, connection, &mut glider, &mut client, 0.0);
    }
    let climbed = server_position(&app, player).y - FLAT;
    assert!(client.y - FLAT > 15.0, "client climbed {}", client.y - FLAT);
    assert!(
        climbed <= max_climb(7.0 * SKYRIDING_GROUND) + 0.01,
        "server climbed {climbed}"
    );
}

#[test]
fn level_skyriding_faster_than_the_energy_allows_is_rejected() {
    let (mut app, player, connection) = flat_app(FLAT);
    skyrider(&mut app, player);
    let start = spot();
    let mut client = start;
    // 100 yd/s level for 1 s; the bank grants 0.5 s ahead of 1 s of server time at the
    // launch bound 34.47 yd/s.
    for _ in 0..20 {
        let inputs = (0..3)
            .map(|_| {
                client.z += 100.0 * FRAME_SECS;
                flight_input(client, true)
            })
            .collect();
        tick(&mut app, connection, inputs);
    }
    let travelled = server_position(&app, player).distance(start);
    assert!(
        travelled <= launch_bound() * 1.5 + 0.01,
        "travelled {travelled}"
    );
    assert!(travelled > launch_bound(), "travelled {travelled}");
}

#[test]
fn a_steady_flyer_gains_no_speed_diving() {
    let (mut app, player, connection) = flat_app(FLAT + 200.0);
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerMotion(PlayerMotion::CAN_FLY))
        .insert(AuraStatMods {
            flight_speed: STANDARD_FLYING,
            ..Default::default()
        });
    let start = flat_spot(FLAT + 200.0);
    let mut client = Vec3::new(start.x, start.y, start.z);
    // Diving 50 yd/s straight down for 1 s.
    for _ in 0..20 {
        let inputs = (0..3)
            .map(|_| {
                client.y -= 50.0 * FRAME_SECS;
                flight_input(client, true)
            })
            .collect();
        tick(&mut app, connection, inputs);
    }
    let dropped = start.y - server_y(&app, player);
    assert!(
        dropped <= FLIGHT_SPEED * STANDARD_FLYING * 1.5 + 0.01,
        "dropped {dropped}"
    );
    let motion = *app.world().get::<PlayerMotion>(player).unwrap();
    assert!(!motion.contains(PlayerMotion::ADV_FLYING));
}

#[test]
fn a_stalled_skyrider_landing_ahead_of_the_server_lands_without_falling() {
    let (mut app, player, connection) = flat_app(FLAT + 40.0);
    skyrider(&mut app, player);
    let hover = server_position(&app, player);

    let landed = super::flight::stall_then_land(&mut app, connection, hover, flight_input);

    assert_eq!(server_position(&app, player), landed);
    let health = app.world().get::<Health>(player).unwrap();
    assert_eq!(health.current, health.max);
    let motion = *app.world().get::<PlayerMotion>(player).unwrap();
    assert!(!motion.contains(PlayerMotion::FLYING));
}

/// Launch 300 yards up, dive and level out, flap Surge Forward (`granted`: the cast left its
/// `SkyridingImpulse`), glide on; the farthest the server fell behind the client after the
/// flap.
fn surge_gap(granted: bool) -> f32 {
    let (mut app, player, connection) = flat_app(FLAT + 300.0);
    skyrider(&mut app, player);
    let mut client = server_position(&app, player);
    let mut glider = Glider::launch(Vec3::ZERO);
    for (pitch, ticks) in [(0.0, 40), (-50.0_f32.to_radians(), 40), (0.0, 20)] {
        for _ in 0..ticks {
            glide(&mut app, connection, &mut glider, &mut client, pitch);
        }
    }
    let before = glider.speed();
    assert!(glider.cast(&SKYRIDING, SURGE_FORWARD, 0.0));
    assert!(
        glider.speed() > before + 25.0,
        "{before} -> {}",
        glider.speed()
    );
    if granted {
        app.world_mut()
            .entity_mut(player)
            .insert(crate::skyriding::SkyridingImpulse(SURGE_SPEED));
    }
    let mut worst_gap = 0.0_f32;
    for _ in 0..40 {
        glide(&mut app, connection, &mut glider, &mut client, 0.0);
        worst_gap = worst_gap.max(server_position(&app, player).distance(client));
    }
    worst_gap
}

#[test]
fn the_server_keeps_up_with_a_granted_surge_forward() {
    let gap = surge_gap(true);
    assert!(gap < 0.05, "server fell {gap} yards behind");
}

/// The same flap without the cast: faster than the energy bound, cut short.
#[test]
fn a_surge_the_server_never_granted_falls_behind() {
    let gap = surge_gap(false);
    assert!(gap > 5.0, "server only {gap} yards behind");
}
