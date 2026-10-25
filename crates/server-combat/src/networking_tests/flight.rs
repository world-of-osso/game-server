//! Steady flight: a player who can fly (`PlayerMotion::CAN_FLY`, from a flying mount's
//! MOD_INCREASE_MOUNTED_FLIGHT_SPEED aura) moves to the height it reports at its flight
//! speed and hangs there without gravity; flight it cannot do is held to the land rules
//! (MovementHandler.cpp:128), and losing the ability mid-air drops it.
use super::client_height::{FLAT, FRAME_SECS, flat_app, flat_spot, server_y, tick};
use super::*;
use shared::components::PlayerMotion;
use shared::movement::FLIGHT_SPEED;

/// Mount Speed Mod: Standard Flying Mount 86459: MOD_INCREASE_MOUNTED_FLIGHT_SPEED 220%.
const STANDARD_FLYING: f32 = 3.2;

fn can_fly(app: &mut App, player: Entity) {
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerMotion(PlayerMotion::CAN_FLY))
        .insert(AuraStatMods {
            flight_speed: STANDARD_FLYING,
            ..Default::default()
        });
}

/// One client frame flying (or not) with the feet at `client`.
fn flight_input(client: &Position, flying: bool) -> PlayerInput {
    PlayerInput {
        transport: None,
        direction: [0.0; 3],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
        flying,
        position: [client.x, client.y, client.z],
        epoch: 0,
    }
}

/// One server tick: three 60 fps client frames ascending at `rise` yards a second.
fn ascend(app: &mut App, connection: Entity, client: &mut Position, rise: f32, flying: bool) {
    let inputs = (0..3)
        .map(|_| {
            client.y += rise * FRAME_SECS;
            flight_input(client, flying)
        })
        .collect();
    tick(app, connection, inputs);
}

fn motion(app: &App, player: Entity) -> PlayerMotion {
    *app.world().get::<PlayerMotion>(player).unwrap()
}

#[test]
fn a_flyer_ascends_with_the_client_and_hangs_in_the_air() {
    let (mut app, player, connection) = flat_app(FLAT);
    can_fly(&mut app, player);
    let mut client = flat_spot(FLAT);
    let flight_speed = FLIGHT_SPEED * STANDARD_FLYING;

    for _ in 0..40 {
        ascend(&mut app, connection, &mut client, flight_speed, true);
    }
    // 40 ticks of 0.05 s at 22.4 yd/s: 44.8 yards up.
    assert!((client.y - (FLAT + 44.8)).abs() < 0.01, "{}", client.y);
    assert!((server_y(&app, player) - client.y).abs() < 0.01);
    assert!(motion(&app, player).contains(PlayerMotion::CAN_FLY | PlayerMotion::FLYING));
    assert_eq!(
        app.world().get::<MovementSpeed>(player).unwrap().0,
        flight_speed
    );

    let hovering = client.y;
    for _ in 0..20 {
        tick(&mut app, connection, Vec::new());
    }
    assert_eq!(server_y(&app, player), hovering);
}

#[test]
fn a_flyer_cannot_outpace_its_flight_speed() {
    let (mut app, player, connection) = flat_app(FLAT);
    can_fly(&mut app, player);
    let mut client = flat_spot(FLAT);

    for _ in 0..20 {
        ascend(&mut app, connection, &mut client, 100.0, true);
    }
    // 1 s of reports at 100 yd/s; the bank grants 0.5 s ahead of 1 s of server time,
    // at most 22.4 yd/s.
    let max = FLAT + FLIGHT_SPEED * STANDARD_FLYING * 1.5;
    assert!(
        server_y(&app, player) <= max + 0.01,
        "{}",
        server_y(&app, player)
    );
    assert!(server_y(&app, player) > FLAT + 20.0);
}

#[test]
fn flying_without_can_fly_is_held_to_the_land_rules() {
    let (mut app, player, connection) = flat_app(FLAT);
    let mut client = flat_spot(FLAT);

    for _ in 0..20 {
        ascend(&mut app, connection, &mut client, 22.4, true);
    }
    assert!(
        server_y(&app, player) < FLAT + 0.5,
        "{}",
        server_y(&app, player)
    );
    assert!(!motion(&app, player).contains(PlayerMotion::FLYING));
}

#[test]
fn losing_can_fly_in_the_air_falls_to_the_ground() {
    let (mut app, player, connection) = flat_app(FLAT);
    can_fly(&mut app, player);
    let mut client = flat_spot(FLAT);
    for _ in 0..10 {
        ascend(&mut app, connection, &mut client, 22.4, true);
    }
    assert!(server_y(&app, player) > FLAT + 10.0);

    // `Unit::SetCanFly(false)` (the speed aura left with the mount).
    app.world_mut().entity_mut(player).insert(PlayerMotion(0));
    for _ in 0..40 {
        tick(&mut app, connection, Vec::new());
    }
    assert_eq!(server_y(&app, player), FLAT);
}

/// A flyer hovering 40 yards up whose client stalls: one frame dives to 5 yards over the
/// ground 12 yards ahead, the next lands 18 yards ahead (`flying: false`), both before
/// the server has flown the dive. `input` builds the client's frame at a point, flying
/// or not. Returns the landing point.
pub(super) fn stall_then_land(
    app: &mut App,
    connection: Entity,
    hover: Vec3,
    input: impl Fn(Vec3, bool) -> PlayerInput,
) -> Vec3 {
    tick(app, connection, vec![input(hover, true)]);
    let dived = Vec3::new(hover.x, FLAT + 5.0, hover.z + 12.0);
    tick(app, connection, vec![input(dived, true)]);
    let landed = Vec3::new(hover.x, FLAT, hover.z + 18.0);
    tick(app, connection, vec![input(landed, false)]);
    for _ in 0..80 {
        tick(app, connection, Vec::new());
    }
    landed
}

/// The landing frame's `flying: false` is the state where it landed; the client flew
/// there. TrinityCore keeps the fall height at every flying report
/// (`Player::UpdateFallInformationIfNeed`), so landing from flight falls nothing
/// (`Player::HandleFall`: z_diff = m_lastFallZ - landing z).
#[test]
fn a_stalled_flyer_landing_ahead_of_the_server_lands_without_falling() {
    let (mut app, player, connection) = flat_app(FLAT + 40.0);
    can_fly(&mut app, player);
    let spawn = flat_spot(FLAT + 40.0);
    let hover = Vec3::new(spawn.x, spawn.y, spawn.z);

    let landed = stall_then_land(&mut app, connection, hover, |at, flying| {
        flight_input(
            &Position {
                x: at.x,
                y: at.y,
                z: at.z,
            },
            flying,
        )
    });

    let position = app.world().get::<Position>(player).unwrap();
    assert_eq!(Vec3::new(position.x, position.y, position.z), landed);
    let health = app.world().get::<Health>(player).unwrap();
    assert_eq!(health.current, health.max);
    assert!(!motion(&app, player).contains(PlayerMotion::FLYING));
    assert_eq!(app.world().get::<MovementSpeed>(player).unwrap().0, 7.0);
}
