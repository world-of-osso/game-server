use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::*;
use lightyear::prelude::server::*;
use shared::components::{Health, MovementSpeed, Player, Position, Rotation};
use shared::protocol::PlayerInput;
use std::time::Duration;

use crate::persistence::{self, CharacterData, ClientId, GameDatabase};

const SERVER_PORT: u16 = 5000;

/// Links a connection entity to its spawned player entity.
#[derive(Component, Debug)]
pub struct PlayerLink(pub Entity);

/// Elwynn Forest spawn point (Northshire Abbey area).
const SPAWN_X: f32 = -8949.0;
const SPAWN_Y: f32 = -132.0;
const SPAWN_Z: f32 = 83.0;

/// Default run speed in yards per second (WoW run speed).
const DEFAULT_RUN_SPEED: f32 = 7.0;
/// Walk speed in yards per second.
const DEFAULT_WALK_SPEED: f32 = 2.5;

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
/// Loads saved data from the database if available, otherwise uses defaults.
fn on_client_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    db: Option<Res<GameDatabase>>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0.to_bits();
    info!("Client connected: {client_id}");

    let saved = db.as_ref().and_then(|db| persistence::load_character(&db.db, client_id));
    let (pos, health, name) = character_from_saved(saved, client_id);

    let rotation = Rotation { x: 0.0, y: 0.0, z: 0.0 };
    let speed = MovementSpeed(DEFAULT_RUN_SPEED);
    let player_entity = commands
        .spawn((pos, health, rotation, speed, Player { name: name.clone() }, ClientId(client_id), Replicate::default()))
        .id();
    commands.entity(trigger.entity).insert(PlayerLink(player_entity));
    info!("Spawned player '{name}' at ({}, {}, {})", pos.x, pos.y, pos.z);
}

/// Extract position/health/name from saved data, or use defaults.
fn character_from_saved(saved: Option<CharacterData>, client_id: u64) -> (Position, Health, String) {
    match saved {
        Some(data) => (
            Position { x: data.x, y: data.y, z: data.z },
            Health { current: data.health, max: data.max_health },
            data.name,
        ),
        None => (
            Position { x: SPAWN_X, y: SPAWN_Y, z: SPAWN_Z },
            Health { current: 100.0, max: 100.0 },
            format!("Player-{client_id}"),
        ),
    }
}

/// When a client disconnects, save their data and despawn the player entity.
fn on_client_disconnected(
    trigger: On<Remove, Connected>,
    conn_query: Query<(&RemoteId, Option<&PlayerLink>), With<ClientOf>>,
    player_query: Query<(&Position, &Health, &Player, &ClientId)>,
    db: Option<Res<GameDatabase>>,
    mut commands: Commands,
) {
    let Ok((remote_id, player_link)) = conn_query.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0.to_bits();
    info!("Client disconnected: {client_id}");

    if let Some(link) = player_link {
        save_player_on_disconnect(&db, &player_query, link.0);
        commands.entity(link.0).despawn();
        info!("Despawned player entity for client {client_id}");
    }
}

/// Save player data to the database before disconnect.
fn save_player_on_disconnect(
    db: &Option<Res<GameDatabase>>,
    player_query: &Query<(&Position, &Health, &Player, &ClientId)>,
    entity: Entity,
) {
    let Some(db) = db.as_ref() else { return };
    let Ok((pos, health, player, cid)) = player_query.get(entity) else { return };
    let data = CharacterData::from_components(player, pos, health);
    persistence::save_character(&db.db, cid.0, &data);
    info!("Saved character data for client {}", cid.0);
}

/// Read PlayerInput messages from connected clients and apply movement to their Position.
fn process_player_inputs(
    mut receivers: Query<(&mut MessageReceiver<PlayerInput>, &PlayerLink)>,
    mut players: Query<(&mut Position, &mut Rotation, &mut MovementSpeed), With<Player>>,
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
    players: &mut Query<(&mut Position, &mut Rotation, &mut MovementSpeed), With<Player>>,
    time: &Time,
) {
    let Ok((mut pos, mut rotation, mut speed)) = players.get_mut(player_entity) else {
        return;
    };
    // Update rotation from client facing
    rotation.y = input.facing_yaw;
    // Update speed based on run/walk toggle
    speed.0 = if input.running { DEFAULT_RUN_SPEED } else { DEFAULT_WALK_SPEED };

    let dir_len = direction_length(&input.direction);
    if dir_len < 0.001 {
        return;
    }
    let scale = speed.0 * time.delta_secs() / dir_len;
    pos.x += input.direction[0] * scale;
    pos.y += input.direction[1] * scale;
    pos.z += input.direction[2] * scale;
}

/// Compute the length of a 3D direction vector.
fn direction_length(dir: &[f32; 3]) -> f32 {
    (dir[0].powi(2) + dir[1].powi(2) + dir[2].powi(2)).sqrt()
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
                Rotation { x: 0.0, y: 0.0, z: 0.0 },
                MovementSpeed(DEFAULT_RUN_SPEED),
                Player { name: "test".into() },
            ))
            .id()
    }

    fn make_input(dx: f32, dy: f32, dz: f32) -> PlayerInput {
        PlayerInput {
            direction: [dx, dy, dz],
            facing_yaw: 0.0,
            jumping: false,
            running: true,
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
            move |mut q: Query<(&mut Position, &mut Rotation, &mut MovementSpeed), With<Player>>, time: Res<Time>| {
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

    /// Verify that facing_yaw updates the player's Rotation.y.
    #[test]
    fn input_updates_rotation() {
        let mut app = setup_app_with_time();
        let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
        let input = PlayerInput {
            direction: [1.0, 0.0, 0.0],
            facing_yaw: 1.5,
            jumping: false,
            running: true,
        };
        let _ = app.world_mut().run_system_once(
            move |mut q: Query<(&mut Position, &mut Rotation, &mut MovementSpeed), With<Player>>, time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );
        let rot = *app.world().entity(player).get::<Rotation>().unwrap();
        assert!((rot.y - 1.5).abs() < 1e-6, "rotation.y should be 1.5, got {}", rot.y);
    }

    /// Verify that running=false sets walk speed and moves slower.
    #[test]
    fn walk_speed_when_not_running() {
        let mut app = setup_app_with_time();
        let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
        let input = PlayerInput {
            direction: [1.0, 0.0, 0.0],
            facing_yaw: 0.0,
            jumping: false,
            running: false,
        };
        let _ = app.world_mut().run_system_once(
            move |mut q: Query<(&mut Position, &mut Rotation, &mut MovementSpeed), With<Player>>, time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );
        let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
        assert!((speed.0 - DEFAULT_WALK_SPEED).abs() < 1e-6, "speed should be walk speed");
    }

    /// Verify that zero-length direction is ignored.
    #[test]
    fn zero_direction_ignored() {
        let mut app = setup_app_with_time();

        let player = spawn_player(app.world_mut(), 5.0, 5.0, 5.0);
        let input = make_input(0.0, 0.0, 0.0);

        let _ = app.world_mut().run_system_once(
            move |mut q: Query<(&mut Position, &mut Rotation, &mut MovementSpeed), With<Player>>, time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );

        let pos = *app.world().entity(player).get::<Position>().unwrap();
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 5.0);
        assert_eq!(pos.z, 5.0);
    }
}
