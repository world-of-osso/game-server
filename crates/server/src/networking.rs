use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use shared::components::{Health, Player, Position};
use shared::protocol::{InputChannel, PlayerInput};
use std::time::Duration;

const SERVER_PORT: u16 = 5000;

/// Elwynn Forest spawn point (Northshire Abbey area).
const SPAWN_X: f32 = -8949.0;
const SPAWN_Y: f32 = -132.0;
const SPAWN_Z: f32 = 83.0;

/// Movement speed in yards per second (matches WoW walk speed).
const MOVE_SPEED: f32 = 2.5;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_server);
        app.add_systems(FixedUpdate, process_player_inputs);
        app.add_observer(on_new_link);
        app.add_observer(on_client_connected);
        app.add_observer(on_client_disconnected);
    }
}

fn start_server(mut commands: Commands) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), SERVER_PORT);
    let entity = commands
        .spawn((
            LocalAddr(addr),
            ServerUdpIo::default(),
            NetcodeServer::new(NetcodeConfig::default()),
        ))
        .id();
    commands.trigger(Start { entity });
    info!("Server listening on {addr}");
}

/// When a new transport link is established (client raw link), insert a ReplicationSender.
fn on_new_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands.entity(trigger.entity).insert(ReplicationSender::new(
        Duration::from_millis(50),
        SendUpdatesMode::SinceLastAck,
        false,
    ));
}

/// When a client fully connects, spawn a player entity with replicated components.
fn on_client_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0;
    info!("Client connected: {client_id:?}");

    let name = format!("Player-{client_id:?}");
    commands.spawn((
        Position { x: SPAWN_X, y: SPAWN_Y, z: SPAWN_Z },
        Health { current: 100.0, max: 100.0 },
        Player { name: name.clone() },
        Replicate::default(),
    ));
    info!("Spawned player entity '{name}' at ({SPAWN_X}, {SPAWN_Y}, {SPAWN_Z})");
}

/// When a client disconnects, log it.
fn on_client_disconnected(
    trigger: On<Remove, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
) {
    if let Ok(remote_id) = query.get(trigger.entity) {
        info!("Client disconnected: {:?}", remote_id.0);
    }
}

/// Read PlayerInput messages from connected clients and apply movement to their Position.
fn process_player_inputs(
    mut receivers: Query<&mut MessageReceiver<PlayerInput>>,
    mut players: Query<&mut Position, With<Player>>,
    time: Res<Time>,
) {
    for mut receiver in receivers.iter_mut() {
        for input in receiver.receive() {
            let dir_len = (input.direction[0].powi(2)
                + input.direction[1].powi(2)
                + input.direction[2].powi(2))
            .sqrt();
            if dir_len < 0.001 {
                continue;
            }
            let dt = time.delta_secs();
            let scale = MOVE_SPEED * dt / dir_len;
            // Apply to the first player entity (single-player for Phase 1).
            // TODO: associate input with specific player via client ID.
            if let Some(mut pos) = players.iter_mut().next() {
                pos.x += input.direction[0] * scale;
                pos.y += input.direction[1] * scale;
                pos.z += input.direction[2] * scale;
            }
        }
    }
}
