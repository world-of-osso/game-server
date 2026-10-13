//! A player strafe-running from the Stockade entrance (`tdb_world_safe_locs` 3599, map 34)
//! with a Petty Criminal 46382 (`content_creature` 20375759) chasing, on the real Stockade
//! WMO ground (`stormwindjail.wdt`), through Lightyear's receive path.
use super::*;
use crate::ground::GroundMap;
use crate::terrain_height::wow_to_bevy;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, PeerId};
use shared::components::CreatureMotion;
use shared::protocol::InputChannel;
use std::time::Duration;

const STOCKADE: u32 = 34;
/// `world/maps/stormwindjail/stormwindjail.wdt`.
const STOCKADE_WDT_FDID: u32 = 791_060;
/// `tdb_world_safe_locs` 3599 "Stormwind Jail Entrance Target", WoW coordinates.
const ENTRANCE: (f32, f32, f32) = (56.6821, 0.62376, -19.2691);
/// Petty Criminal 20375759, WoW spawn position.
const CRIMINAL_SPAWN: (f32, f32, f32) = (93.2359, -0.0572555, -25.6062);
/// The Godot client's frame step (60 fps).
const FRAME_SECS: f32 = 1.0 / 60.0;
const TICK: Duration = Duration::from_millis(50);

fn bevy((x, y, z): (f32, f32, f32)) -> Position {
    let (x, y, z) = wow_to_bevy(x, y, z);
    Position { x, y, z }
}

fn stockade_app() -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: TICK,
    });
    app.register_message::<PlayerInput>();
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.init_resource::<crate::friends::AutoAfkScheduler>();
    app.insert_resource(GroundMap::new(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        std::collections::HashMap::from([(STOCKADE, STOCKADE_WDT_FDID)]),
    ));
    app.add_systems(
        FixedUpdate,
        (
            process_player_inputs,
            apply_terrain_gravity,
            crate::creature_ai::chase_system,
        )
            .chain(),
    );
    app.finish();
    app.cleanup();
    let entrance = bevy(ENTRANCE);
    let player = spawn_player(app.world_mut(), entrance.x, entrance.y, entrance.z);
    app.world_mut()
        .entity_mut(player)
        .insert(crate::maps::InMap::continent(STOCKADE));
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
    let spawn = bevy(CRIMINAL_SPAWN);
    let criminal = app
        .world_mut()
        .spawn((
            spawn,
            crate::creature_ai::SpawnPoint {
                x: spawn.x,
                y: spawn.y,
                z: spawn.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.142_86,
            },
            crate::creature_ai::CreatureState::Pursuing {
                target: player.to_bits(),
            },
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            CreatureMotion::default(),
            crate::maps::InMap::continent(STOCKADE),
        ))
        .id();
    (app, player, connection, criminal)
}

fn tick(app: &mut App, connection: Entity, inputs: Vec<PlayerInput>) {
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

/// W+D facing east: the Godot client (`PlayerMovement::predict`) runs 7 yd/s along
/// forward + right, the diagonal `compute_movement_input` builds, and reports where it got.
fn strafe_run_frame(client: &mut Position) -> PlayerInput {
    let yaw = std::f32::consts::FRAC_PI_2;
    let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let right = Vec3::new(-forward.z, 0.0, forward.x);
    let step = (forward + right).normalize() * DEFAULT_RUN_SPEED * FRAME_SECS;
    client.x += step.x;
    client.z += step.z;
    PlayerInput {
        transport: None,
        // `movement_to_direction(MoveDirection::Forward, yaw)`: what the client sends.
        direction: forward.to_array(),
        facing_yaw: yaw,
        jumping: false,
        running: true,
        swimming: false,
        flying: false,
        position: [client.x, client.y, client.z],
        epoch: 0,
    }
}

fn horizontal_gap(a: &Position, b: &Position) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}

/// Two seconds of W+D from the entrance, then standing: the server's player is where the
/// client put it, and the Petty Criminal that chased it stands in melee reach of the player
/// the client shows.
#[test]
fn strafe_running_player_is_where_the_client_shows_it_and_the_chaser_reaches_it() {
    let (mut app, player, connection, criminal) = stockade_app();
    let mut client = bevy(ENTRANCE);
    for _ in 0..40 {
        let inputs = (0..3).map(|_| strafe_run_frame(&mut client)).collect();
        tick(&mut app, connection, inputs);
    }
    for _ in 0..60 {
        tick(&mut app, connection, vec![]);
    }
    let server = *app.world().get::<Position>(player).unwrap();
    let mob = *app.world().get::<Position>(criminal).unwrap();
    let server_gap = horizontal_gap(&server, &client);
    let mob_gap = horizontal_gap(&mob, &client);
    assert!(
        server_gap < 0.1 && mob_gap <= 5.0,
        "server player {server_gap:.2} yd from the client's; Petty Criminal {mob_gap:.2} yd \
         from the client's player, {:.2} yd from the server's (client {client:?}, server \
         {server:?}, mob {mob:?})",
        horizontal_gap(&mob, &server)
    );
}
