//! A player running the Windrunner Spire (map 2805) corridor over the ~1 yd WMO step at
//! (5118.3, 136.7, 3060.5), through Lightyear's receive path on the real WMO ground. Live
//! run 2026-11-01 00:10Z: the server stopped at the step's steep edge while the client
//! climbed it and walked on, leaving the two 30 yd apart.
use super::*;
use crate::ground::GroundMap;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, PeerId};
use shared::protocol::InputChannel;
use std::time::Duration;

const WINDRUNNER: u32 = 2805;
/// `Map.db2` 2805 WdtFileDataID.
const WINDRUNNER_WDT_FDID: u32 = 6_099_383;
/// Client route waypoints 12 and 13 either side of the step (Bevy space).
const BEFORE_STEP: Vec3 = Vec3::new(5118.695, 136.7143, 3045.86);
const PAST_STEP: Vec3 = Vec3::new(5118.144, 136.707, 3066.151);
/// The Godot client's frame step (60 fps): one report per frame while it moves.
const FRAME_SECS: f32 = 1.0 / 60.0;
/// The run's driver walked in `movement forward --seconds 0.1` pulses (six frames), then
/// waited for the account state before the next (about half a second).
const PULSE_FRAMES: usize = 6;
const PULSE_IDLE_TICKS: usize = 10;
const TICK: Duration = Duration::from_millis(50);

fn windrunner_app() -> (App, Entity, Entity) {
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
        std::collections::HashMap::from([(WINDRUNNER, WINDRUNNER_WDT_FDID)]),
    ));
    app.add_systems(
        FixedUpdate,
        (process_player_inputs, apply_terrain_gravity).chain(),
    );
    app.finish();
    app.cleanup();
    let player = spawn_player(app.world_mut(), BEFORE_STEP.x, BEFORE_STEP.y, BEFORE_STEP.z);
    app.world_mut()
        .entity_mut(player)
        .insert(crate::maps::InMap::continent(WINDRUNNER));
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

/// One client frame running 7 yd/s toward `PAST_STEP`, reported at the walkable floor it
/// stands on (the client climbs the step: 136.71 -> 137.68 -> 136.71).
fn run_frame(client: &mut Vec3, ground: &mut GroundMap) -> PlayerInput {
    let direction = (PAST_STEP - *client).with_y(0.0).normalize_or_zero();
    let step = (DEFAULT_RUN_SPEED * FRAME_SECS).min((PAST_STEP - *client).with_y(0.0).length());
    *client += direction * step;
    let GroundProbe::Supported(floor) = ground.ground_at(WINDRUNNER, client.with_y(139.0)) else {
        panic!("no Windrunner floor under the client at {client:?}");
    };
    client.y = floor;
    PlayerInput {
        transport: None,
        direction: direction.to_array(),
        facing_yaw: direction.x.atan2(direction.z),
        jumping: false,
        running: true,
        swimming: false,
        flying: false,
        position: client.to_array(),
        epoch: 0,
    }
}

/// The client climbs the corridor's WMO step and runs on to waypoint 13; the server's
/// player follows it over the step instead of stopping at its steep edge.
#[test]
fn windrunner_player_climbs_the_corridor_wmo_step_with_the_client() {
    let (mut app, player, connection) = windrunner_app();
    let mut probe = GroundMap::new(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        std::collections::HashMap::from([(WINDRUNNER, WINDRUNNER_WDT_FDID)]),
    );
    let mut client = BEFORE_STEP;
    let mut highest = f32::NEG_INFINITY;
    while (PAST_STEP - client).with_y(0.0).length() > 0.0 {
        let frames: Vec<_> = (0..PULSE_FRAMES)
            .map(|_| run_frame(&mut client, &mut probe))
            .collect();
        let stop = PlayerInput {
            direction: [0.0; 3],
            ..frames.last().unwrap().clone()
        };
        let mut frames = frames.into_iter();
        tick(&mut app, connection, frames.by_ref().take(3).collect());
        tick(&mut app, connection, frames.collect());
        tick(&mut app, connection, vec![stop]);
        for _ in 0..PULSE_IDLE_TICKS {
            tick(&mut app, connection, vec![]);
        }
        highest = highest.max(app.world().get::<Position>(player).unwrap().y);
    }
    for _ in 0..20 {
        tick(&mut app, connection, vec![]);
    }
    let server = *app.world().get::<Position>(player).unwrap();
    let gap = (server.x - client.x).hypot(server.z - client.z);
    assert!(
        gap < 0.1 && (server.y - client.y).abs() < 0.1 && highest > 137.5,
        "server player {gap:.2} yd from the client's at {server:?} (client {client:?}); \
         highest server height {highest:.2}"
    );
}
