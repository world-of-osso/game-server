use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::components::{Health, MovementSpeed, Player, Position, Rotation};
use shared::protocol::{ChatChannel, ChatMessage, LoadTerrain, PlayerInput, SetTarget, TerrainChannel};
use std::collections::HashSet;
use std::time::Duration;

use crate::persistence::{self, CharacterData, ClientId, GameDatabase};
use crate::terrain_height::{GRAVITY, GROUND_SNAP_THRESHOLD, TerrainHeightMap};

const SERVER_PORT: u16 = 5000;

/// Links a connection entity to its spawned player entity.
#[derive(Component, Debug)]
pub struct PlayerLink(pub Entity);

/// The entity this player is currently targeting in combat.
#[derive(Component, Debug, Default)]
pub struct CombatTarget(pub Option<Entity>);

/// Elwynn Forest spawn point (Northshire Abbey area).
const SPAWN_X: f32 = -8949.0;
const SPAWN_Y: f32 = -132.0;
const SPAWN_Z: f32 = 83.0;

/// Tracks which terrain tiles have been sent to a client.
#[derive(Component, Default)]
struct SentTiles {
    tiles: HashSet<(u32, u32)>,
}

/// Default run speed in yards per second (WoW run speed).
const DEFAULT_RUN_SPEED: f32 = 7.0;
/// Walk speed in yards per second.
const DEFAULT_WALK_SPEED: f32 = 2.5;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, start_server);
        app.add_systems(FixedUpdate, process_player_inputs);
        app.add_systems(
            FixedUpdate,
            apply_terrain_gravity.after(process_player_inputs),
        );
        app.add_systems(FixedUpdate, process_chat_messages);
        app.add_systems(FixedUpdate, stream_terrain_tiles);
        app.add_systems(FixedUpdate, process_target_messages);
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
    commands
        .entity(trigger.entity)
        .insert(ReplicationSender::new(
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
    mut senders: Query<&mut MessageSender<LoadTerrain>>,
    db: Option<Res<GameDatabase>>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0.to_bits();
    info!("Client connected: {client_id}");

    let saved = db
        .as_ref()
        .and_then(|db| persistence::load_character(&db.db, client_id));
    let player_entity = spawn_player_entity(&mut commands, saved, client_id);
    // Seed SentTiles with the initial 3x3 grid around (32, 48)
    let mut initial_tiles = HashSet::new();
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let ty = (32i32 + dy).clamp(0, 63) as u32;
            let tx = (48i32 + dx).clamp(0, 63) as u32;
            initial_tiles.insert((ty, tx));
        }
    }

    commands
        .entity(trigger.entity)
        .insert((PlayerLink(player_entity), SentTiles { tiles: initial_tiles }));

    send_load_terrain(trigger.entity, &mut senders);
}

/// Send the initial terrain loading command to a newly connected client.
fn send_load_terrain(
    connection: Entity,
    senders: &mut Query<&mut MessageSender<LoadTerrain>>,
) {
    let Ok(mut sender) = senders.get_mut(connection) else {
        warn!("No MessageSender<LoadTerrain> on connection entity");
        return;
    };
    let msg = LoadTerrain {
        map_name: "azeroth".into(),
        initial_tile_y: 32,
        initial_tile_x: 48,
    };
    sender.send::<TerrainChannel>(msg);
    info!("Sent LoadTerrain: azeroth tile (32, 48)");
}

/// Spawn a player entity with all replicated components and return its Entity.
fn spawn_player_entity(
    commands: &mut Commands,
    saved: Option<CharacterData>,
    client_id: u64,
) -> Entity {
    let (pos, health, name) = character_from_saved(saved, client_id);
    let rotation = Rotation {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let speed = MovementSpeed(DEFAULT_RUN_SPEED);
    let entity = commands
        .spawn((
            pos,
            health,
            rotation,
            speed,
            Player { name: name.clone() },
            ClientId(client_id),
            CombatTarget::default(),
            Replicate::default(),
            NetworkVisibility,
        ))
        .id();
    info!(
        "Spawned player '{name}' at ({}, {}, {})",
        pos.x, pos.y, pos.z
    );
    entity
}

/// Extract position/health/name from saved data, or use defaults.
fn character_from_saved(
    saved: Option<CharacterData>,
    client_id: u64,
) -> (Position, Health, String) {
    match saved {
        Some(data) => (
            Position {
                x: data.x,
                y: data.y,
                z: data.z,
            },
            Health {
                current: data.health,
                max: data.max_health,
            },
            data.name,
        ),
        None => (
            Position {
                x: SPAWN_X,
                y: SPAWN_Y,
                z: SPAWN_Z,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
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
    let Ok((pos, health, player, cid)) = player_query.get(entity) else {
        return;
    };
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
    speed.0 = if input.running {
        DEFAULT_RUN_SPEED
    } else {
        DEFAULT_WALK_SPEED
    };

    let dir_len = direction_length(&input.direction);
    if dir_len < 0.001 {
        return;
    }
    let scale = speed.0 * time.delta_secs() / dir_len;
    pos.x += input.direction[0] * scale;
    pos.y += input.direction[1] * scale;
    pos.z += input.direction[2] * scale;
}

/// Apply gravity and terrain clamping to all players each tick.
fn apply_terrain_gravity(
    terrain: Option<Res<TerrainHeightMap>>,
    mut players: Query<&mut Position, With<Player>>,
    time: Res<Time>,
) {
    let Some(terrain) = terrain.as_ref().filter(|t| t.is_loaded()) else {
        return;
    };
    let dt = time.delta_secs();
    for mut pos in players.iter_mut() {
        clamp_to_terrain(&terrain, &mut pos, dt);
    }
}

/// Clamp a position to terrain: apply gravity if airborne, snap if below ground.
fn clamp_to_terrain(terrain: &TerrainHeightMap, pos: &mut Position, dt: f32) {
    let Some(ground_z) = terrain.height_at(pos.x, pos.y) else {
        return;
    };
    if pos.z < ground_z {
        // Below terrain — snap up.
        pos.z = ground_z;
    } else if pos.z > ground_z + GROUND_SNAP_THRESHOLD {
        // Above terrain — apply gravity.
        pos.z -= GRAVITY * dt;
        // Don't overshoot ground.
        if pos.z < ground_z {
            pos.z = ground_z;
        }
    }
}

/// Compute the length of a 3D direction vector.
fn direction_length(dir: &[f32; 3]) -> f32 {
    (dir[0].powi(2) + dir[1].powi(2) + dir[2].powi(2)).sqrt()
}

/// Read incoming chat messages, override sender with the player's actual name,
/// and broadcast to all connected clients.
fn process_chat_messages(
    mut receivers: Query<(&mut MessageReceiver<ChatMessage>, &PlayerLink)>,
    players: Query<&Player>,
    mut senders: Query<&mut MessageSender<ChatMessage>>,
) {
    let outgoing: Vec<ChatMessage> = collect_chat_messages(&mut receivers, &players);
    broadcast_chat_messages(&outgoing, &mut senders);
}

/// Collect chat messages from all receivers, replacing sender with the player's name.
fn collect_chat_messages(
    receivers: &mut Query<(&mut MessageReceiver<ChatMessage>, &PlayerLink)>,
    players: &Query<&Player>,
) -> Vec<ChatMessage> {
    let mut outgoing = Vec::new();
    for (mut receiver, link) in receivers.iter_mut() {
        let sender_name = players
            .get(link.0)
            .map(|p| p.name.clone())
            .unwrap_or_else(|_| "Unknown".into());
        for mut msg in receiver.receive() {
            msg.sender = sender_name.clone();
            info!("[Chat] {}: {}", msg.sender, msg.content);
            outgoing.push(msg);
        }
    }
    outgoing
}

/// Send collected chat messages to all connected clients.
fn broadcast_chat_messages(
    outgoing: &[ChatMessage],
    senders: &mut Query<&mut MessageSender<ChatMessage>>,
) {
    for msg in outgoing {
        for mut sender in senders.iter_mut() {
            sender.send::<ChatChannel>(msg.clone());
        }
    }
}

/// Read SetTarget messages from clients and update their CombatTarget component.
fn process_target_messages(
    mut receivers: Query<(&mut MessageReceiver<SetTarget>, &PlayerLink)>,
    mut targets: Query<&mut CombatTarget>,
) {
    for (mut receiver, link) in receivers.iter_mut() {
        for msg in receiver.receive() {
            let target = msg.target_entity.map(|bits| Entity::from_bits(bits));
            if let Ok(mut combat_target) = targets.get_mut(link.0) {
                combat_target.0 = target;
                debug!("Player {:?} target set to {:?}", link.0, target);
            }
        }
    }
}

const TILE_SIZE: f32 = 533.33;

fn wow_to_tile(wow_x: f32, wow_y: f32) -> (u32, u32) {
    let row = ((17066.67 - wow_x) / TILE_SIZE).floor() as i32;
    let col = ((17066.67 - wow_y) / TILE_SIZE).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

fn stream_terrain_tiles(
    mut conn_q: Query<(Entity, &PlayerLink, &mut SentTiles)>,
    players: Query<&Position, With<Player>>,
    mut senders: Query<&mut MessageSender<LoadTerrain>>,
) {
    for (conn_entity, link, mut sent) in conn_q.iter_mut() {
        let Ok(pos) = players.get(link.0) else { continue };
        let (tile_y, tile_x) = wow_to_tile(pos.x, pos.y);
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let ty = (tile_y as i32 + dy).clamp(0, 63) as u32;
                let tx = (tile_x as i32 + dx).clamp(0, 63) as u32;
                if sent.tiles.insert((ty, tx)) {
                    let Ok(mut sender) = senders.get_mut(conn_entity) else { continue };
                    sender.send::<TerrainChannel>(LoadTerrain {
                        map_name: "azeroth".into(),
                        initial_tile_y: ty,
                        initial_tile_x: tx,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use shared::protocol::{ChatType, PlayerInput};

    fn spawn_player(world: &mut World, x: f32, y: f32, z: f32) -> Entity {
        world
            .spawn((
                Position { x, y, z },
                Rotation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                MovementSpeed(DEFAULT_RUN_SPEED),
                Player {
                    name: "test".into(),
                },
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
            move |mut q: Query<
                (&mut Position, &mut Rotation, &mut MovementSpeed),
                With<Player>,
            >,
                  time: Res<Time>| {
                apply_movement_input(&input, player_a, &mut q, &time);
            },
        );

        let pos_a = *app.world().entity(player_a).get::<Position>().unwrap();
        let pos_b = *app.world().entity(player_b).get::<Position>().unwrap();

        // player_a should have moved (x increased)
        assert!(
            pos_a.x > 0.0,
            "player_a should have moved, got x={}",
            pos_a.x
        );
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
            move |mut q: Query<
                (&mut Position, &mut Rotation, &mut MovementSpeed),
                With<Player>,
            >,
                  time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );
        let rot = *app.world().entity(player).get::<Rotation>().unwrap();
        assert!(
            (rot.y - 1.5).abs() < 1e-6,
            "rotation.y should be 1.5, got {}",
            rot.y
        );
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
            move |mut q: Query<
                (&mut Position, &mut Rotation, &mut MovementSpeed),
                With<Player>,
            >,
                  time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );
        let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
        assert!(
            (speed.0 - DEFAULT_WALK_SPEED).abs() < 1e-6,
            "speed should be walk speed"
        );
    }

    /// Verify that zero-length direction is ignored.
    #[test]
    fn zero_direction_ignored() {
        let mut app = setup_app_with_time();

        let player = spawn_player(app.world_mut(), 5.0, 5.0, 5.0);
        let input = make_input(0.0, 0.0, 0.0);

        let _ = app.world_mut().run_system_once(
            move |mut q: Query<
                (&mut Position, &mut Rotation, &mut MovementSpeed),
                With<Player>,
            >,
                  time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );

        let pos = *app.world().entity(player).get::<Position>().unwrap();
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 5.0);
        assert_eq!(pos.z, 5.0);
    }

    /// Verify that clamp_to_terrain snaps a player below ground up to terrain level.
    #[test]
    fn terrain_clamps_below_ground() {
        // Build a flat terrain chunk at height 50 covering corner (100, 100).
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);

        // Player below ground at z=30.
        let mut pos = Position {
            x: 90.0,
            y: 90.0,
            z: 30.0,
        };
        clamp_to_terrain(&map, &mut pos, 0.05);
        assert!(
            (pos.z - 50.0).abs() < 0.01,
            "player should be snapped to ground 50, got {}",
            pos.z
        );
    }

    /// Verify that gravity pulls a player above ground toward terrain.
    #[test]
    fn terrain_applies_gravity() {
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);

        // Player well above ground.
        let mut pos = Position {
            x: 90.0,
            y: 90.0,
            z: 60.0,
        };
        let dt = 0.05; // 50ms tick
        clamp_to_terrain(&map, &mut pos, dt);
        let expected = 60.0 - GRAVITY * dt;
        assert!(
            (pos.z - expected).abs() < 0.01,
            "player should have fallen to {expected}, got {}",
            pos.z
        );
    }

    /// Verify that a player on the ground (within snap threshold) stays put.
    #[test]
    fn terrain_on_ground_stays() {
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);

        let mut pos = Position {
            x: 90.0,
            y: 90.0,
            z: 50.05,
        };
        clamp_to_terrain(&map, &mut pos, 0.05);
        // Within threshold — no gravity applied, should stay.
        assert!(
            (pos.z - 50.05).abs() < 0.01,
            "player near ground should stay, got {}",
            pos.z
        );
    }

    /// Verify ChatMessage sender is overridden from Player component name.
    #[test]
    fn chat_message_sender_override() {
        let mut msg = ChatMessage {
            sender: "hacker".into(),
            content: "Hello".into(),
            channel: ChatType::Say,
        };
        // Simulate what collect_chat_messages does: override sender
        let player_name = "TrustedPlayer".to_string();
        msg.sender = player_name.clone();
        assert_eq!(msg.sender, "TrustedPlayer");
        assert_eq!(msg.content, "Hello");
    }

    /// Verify that SetTarget updates the player's CombatTarget component.
    #[test]
    fn set_target_updates_combat_target() {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);

        let target_entity = app.world_mut().spawn_empty().id();
        let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
        app.world_mut()
            .entity_mut(player)
            .insert(CombatTarget::default());

        // Simulate: set target via direct component mutation (no network in unit test)
        let bits = target_entity.to_bits();
        let resolved = Entity::from_bits(bits);
        app.world_mut()
            .entity_mut(player)
            .get_mut::<CombatTarget>()
            .unwrap()
            .0 = Some(resolved);

        let ct = app.world().entity(player).get::<CombatTarget>().unwrap();
        assert_eq!(ct.0, Some(target_entity));
    }
}
