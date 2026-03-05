use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use shared::components::{Health, Player, Position};
use shared::protocol::{InputChannel, PlayerInput};
use std::time::Duration;

const SERVER_PORT: u16 = 5000;

/// Links a connection entity to its spawned player entity.
#[derive(Component, Debug)]
pub struct PlayerLink(pub Entity);

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
    let player_entity = commands
        .spawn((
            Position { x: SPAWN_X, y: SPAWN_Y, z: SPAWN_Z },
            Health { current: 100.0, max: 100.0 },
            Player { name: name.clone() },
            Replicate::default(),
        ))
        .id();
    commands.entity(trigger.entity).insert(PlayerLink(player_entity));
    info!("Spawned player entity '{name}' at ({SPAWN_X}, {SPAWN_Y}, {SPAWN_Z})");
}

/// When a client disconnects, despawn their player entity.
fn on_client_disconnected(
    trigger: On<Remove, Connected>,
    query: Query<(&RemoteId, Option<&PlayerLink>), With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok((remote_id, player_link)) = query.get(trigger.entity) else {
        return;
    };
    info!("Client disconnected: {:?}", remote_id.0);
    if let Some(link) = player_link {
        commands.entity(link.0).despawn();
        info!("Despawned player entity for client {:?}", remote_id.0);
    }
}

/// Read PlayerInput messages from connected clients and apply movement to their Position.
fn process_player_inputs(
    mut receivers: Query<(&mut MessageReceiver<PlayerInput>, &PlayerLink)>,
    mut players: Query<&mut Position, With<Player>>,
    time: Res<Time>,
) {
    for (mut receiver, link) in receivers.iter_mut() {
        for input in receiver.receive() {
            apply_movement_input(&input, link.0, &mut players, &time);
        }
    }
}

/// Apply a single movement input to the specified player entity.
fn apply_movement_input(
    input: &PlayerInput,
    player_entity: Entity,
    players: &mut Query<&mut Position, With<Player>>,
    time: &Time,
) {
    let dir_len = (input.direction[0].powi(2)
        + input.direction[1].powi(2)
        + input.direction[2].powi(2))
    .sqrt();
    if dir_len < 0.001 {
        return;
    }
    let dt = time.delta_secs();
    let scale = MOVE_SPEED * dt / dir_len;
    if let Ok(mut pos) = players.get_mut(player_entity) {
        pos.x += input.direction[0] * scale;
        pos.y += input.direction[1] * scale;
        pos.z += input.direction[2] * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use shared::protocol::PlayerInput;

    fn spawn_player(world: &mut World, x: f32, y: f32, z: f32) -> Entity {
        world
            .spawn((
                Position { x, y, z },
                Player { name: "test".into() },
            ))
            .id()
    }

    fn make_input(dx: f32, dy: f32, dz: f32) -> PlayerInput {
        PlayerInput {
            direction: [dx, dy, dz],
            facing_yaw: 0.0,
            jumping: false,
        }
    }

    fn setup_app_with_time() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        // Two updates so Time has a non-zero delta
        app.update();
        app.update();
        app
    }

    /// Verify that apply_movement_input only moves the targeted entity.
    #[test]
    fn input_routes_to_correct_player() {
        let mut app = setup_app_with_time();

        let player_a = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
        let player_b = spawn_player(app.world_mut(), 100.0, 100.0, 100.0);

        let input = make_input(1.0, 0.0, 0.0);

        // Apply input targeting player_a only
        let _ = app.world_mut().run_system_once(
            move |mut q: Query<&mut Position, With<Player>>, time: Res<Time>| {
                apply_movement_input(&input, player_a, &mut q, &time);
            },
        );

        let pos_a = *app.world().entity(player_a).get::<Position>().unwrap();
        let pos_b = *app.world().entity(player_b).get::<Position>().unwrap();

        // player_a should have moved (x increased)
        assert!(pos_a.x > 0.0, "player_a should have moved, got x={}", pos_a.x);
        // player_b should be untouched
        assert_eq!(pos_b.x, 100.0, "player_b x should be unchanged");
        assert_eq!(pos_b.y, 100.0, "player_b y should be unchanged");
    }

    /// Verify that zero-length direction is ignored.
    #[test]
    fn zero_direction_ignored() {
        let mut app = setup_app_with_time();

        let player = spawn_player(app.world_mut(), 5.0, 5.0, 5.0);
        let input = make_input(0.0, 0.0, 0.0);

        let _ = app.world_mut().run_system_once(
            move |mut q: Query<&mut Position, With<Player>>, time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );

        let pos = *app.world().entity(player).get::<Position>().unwrap();
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 5.0);
        assert_eq!(pos.z, 5.0);
    }
}
