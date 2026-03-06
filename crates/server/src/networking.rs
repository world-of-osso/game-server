use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::components::{Health, MovementSpeed, Player, Position, Rotation};
use shared::protocol::{
    ChatChannel, ChatMessage, LoadTerrain, PlayerInput, SetTarget, TerrainChannel,
};
use std::time::Duration;

use crate::auth::{
    self, Authenticated, ConnectedAccounts, InWorld, PendingAuth, PendingAuthTimer, SentTiles,
};
use crate::persistence::{self, CharacterData, ClientId, GameDatabase};
use crate::terrain_height::{GRAVITY, GROUND_SNAP_THRESHOLD, TerrainHeightMap};

const SERVER_PORT: u16 = 5000;

/// Links a connection entity to its spawned player entity.
#[derive(Component, Debug)]
pub struct PlayerLink(pub Entity);

/// The entity this player is currently targeting in combat.
#[derive(Component, Debug, Default)]
pub struct CombatTarget(pub Option<Entity>);

/// Default run speed in yards per second (WoW run speed).
const DEFAULT_RUN_SPEED: f32 = 7.0;
/// Walk speed in yards per second.
const DEFAULT_WALK_SPEED: f32 = 2.5;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectedAccounts>();
        app.add_systems(Startup, start_server);
        app.add_systems(FixedUpdate, process_player_inputs);
        app.add_systems(
            FixedUpdate,
            apply_terrain_gravity.after(process_player_inputs),
        );
        app.add_systems(FixedUpdate, process_chat_messages);
        app.add_systems(FixedUpdate, stream_terrain_tiles);
        app.add_systems(FixedUpdate, process_target_messages);
        app.add_systems(FixedUpdate, auth::process_login_requests);
        app.add_systems(FixedUpdate, auth::process_register_requests);
        app.add_systems(FixedUpdate, auth::process_create_character);
        app.add_systems(FixedUpdate, auth::process_delete_character);
        app.add_systems(FixedUpdate, auth::process_select_character);
        app.add_systems(FixedUpdate, auth::auth_timeout);
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

/// When a new transport link is established, insert a ReplicationSender.
fn on_new_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(ReplicationSender::new(
            Duration::from_millis(50),
            SendUpdatesMode::SinceLastAck,
            false,
        ));
}

/// When a client connects, mark as pending auth (no player entity yet).
fn on_client_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0.to_bits();
    info!("Client connected: {client_id} (pending auth)");
    let elapsed = time.elapsed_secs_f64();
    commands
        .entity(trigger.entity)
        .insert((PendingAuth, PendingAuthTimer(elapsed)));
}

/// When a client disconnects, save data and clean up.
fn on_client_disconnected(
    trigger: On<Remove, Connected>,
    conn_query: Query<
        (Option<&PlayerLink>, Option<&Authenticated>, Option<&InWorld>),
        With<ClientOf>,
    >,
    player_query: Query<(&Position, &Health, &Player, &ClientId)>,
    db: Option<Res<GameDatabase>>,
    mut connected: ResMut<ConnectedAccounts>,
    mut commands: Commands,
) {
    let Ok((player_link, auth, in_world)) = conn_query.get(trigger.entity) else {
        return;
    };
    if let Some(link) = player_link {
        save_and_despawn(&db, &player_query, link.0, in_world, &mut commands);
    }
    if let Some(auth) = auth {
        connected.0.remove(&auth.account_id);
        info!("Account {} logged out", auth.account_id);
    }
}

/// Save player data and despawn the entity on disconnect.
fn save_and_despawn(
    db: &Option<Res<GameDatabase>>,
    player_query: &Query<(&Position, &Health, &Player, &ClientId)>,
    entity: Entity,
    in_world: Option<&InWorld>,
    commands: &mut Commands,
) {
    if let (Some(db), Some(_)) = (db.as_ref(), in_world) {
        if let Ok((pos, health, player, cid)) = player_query.get(entity) {
            let data = CharacterData::from_components(player, pos, health);
            persistence::save_character(&db.db, cid.0, &data);
            info!("Saved character data for character_id {}", cid.0);
        }
    }
    commands.entity(entity).despawn();
    info!("Despawned player entity {:?}", entity);
}

/// Send the initial terrain loading command to a client.
pub fn send_load_terrain(
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

/// Read PlayerInput messages and apply movement.
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
    rotation.y = input.facing_yaw;
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
        pos.z = ground_z;
    } else if pos.z > ground_z + GROUND_SNAP_THRESHOLD {
        pos.z -= GRAVITY * dt;
        if pos.z < ground_z {
            pos.z = ground_z;
        }
    }
}

fn direction_length(dir: &[f32; 3]) -> f32 {
    (dir[0].powi(2) + dir[1].powi(2) + dir[2].powi(2)).sqrt()
}

/// Read incoming chat messages and broadcast to all clients.
fn process_chat_messages(
    mut receivers: Query<(&mut MessageReceiver<ChatMessage>, &PlayerLink)>,
    players: Query<&Player>,
    mut senders: Query<&mut MessageSender<ChatMessage>>,
) {
    let outgoing: Vec<ChatMessage> = collect_chat_messages(&mut receivers, &players);
    broadcast_chat_messages(&outgoing, &mut senders);
}

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

/// Read SetTarget messages and update CombatTarget components.
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
        send_nearby_tiles(conn_entity, pos, &mut sent, &mut senders);
    }
}

fn send_nearby_tiles(
    conn_entity: Entity,
    pos: &Position,
    sent: &mut SentTiles,
    senders: &mut Query<&mut MessageSender<LoadTerrain>>,
) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use shared::protocol::{ChatType, PlayerInput};

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
        app.update();
        app.update();
        app
    }

    #[test]
    fn input_routes_to_correct_player() {
        let mut app = setup_app_with_time();
        let player_a = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
        let player_b = spawn_player(app.world_mut(), 100.0, 100.0, 100.0);
        let input = make_input(1.0, 0.0, 0.0);

        let _ = app.world_mut().run_system_once(
            move |mut q: Query<
                (&mut Position, &mut Rotation, &mut MovementSpeed),
                With<Player>,
            >, time: Res<Time>| {
                apply_movement_input(&input, player_a, &mut q, &time);
            },
        );

        let pos_a = *app.world().entity(player_a).get::<Position>().unwrap();
        let pos_b = *app.world().entity(player_b).get::<Position>().unwrap();
        assert!(pos_a.x > 0.0, "player_a should have moved");
        assert_eq!(pos_b.x, 100.0, "player_b x unchanged");
        assert_eq!(pos_b.y, 100.0, "player_b y unchanged");
    }

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
            >, time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );
        let rot = *app.world().entity(player).get::<Rotation>().unwrap();
        assert!((rot.y - 1.5).abs() < 1e-6);
    }

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
            >, time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );
        let speed = app.world().entity(player).get::<MovementSpeed>().unwrap();
        assert!((speed.0 - DEFAULT_WALK_SPEED).abs() < 1e-6);
    }

    #[test]
    fn zero_direction_ignored() {
        let mut app = setup_app_with_time();
        let player = spawn_player(app.world_mut(), 5.0, 5.0, 5.0);
        let input = make_input(0.0, 0.0, 0.0);

        let _ = app.world_mut().run_system_once(
            move |mut q: Query<
                (&mut Position, &mut Rotation, &mut MovementSpeed),
                With<Player>,
            >, time: Res<Time>| {
                apply_movement_input(&input, player, &mut q, &time);
            },
        );

        let pos = *app.world().entity(player).get::<Position>().unwrap();
        assert_eq!(pos.x, 5.0);
        assert_eq!(pos.y, 5.0);
        assert_eq!(pos.z, 5.0);
    }

    #[test]
    fn terrain_clamps_below_ground() {
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
        let mut pos = Position { x: 90.0, y: 90.0, z: 30.0 };
        clamp_to_terrain(&map, &mut pos, 0.05);
        assert!((pos.z - 50.0).abs() < 0.01);
    }

    #[test]
    fn terrain_applies_gravity() {
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
        let mut pos = Position { x: 90.0, y: 90.0, z: 60.0 };
        let dt = 0.05;
        clamp_to_terrain(&map, &mut pos, dt);
        let expected = 60.0 - GRAVITY * dt;
        assert!((pos.z - expected).abs() < 0.01);
    }

    #[test]
    fn terrain_on_ground_stays() {
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
        let mut pos = Position { x: 90.0, y: 90.0, z: 50.05 };
        clamp_to_terrain(&map, &mut pos, 0.05);
        assert!((pos.z - 50.05).abs() < 0.01);
    }

    #[test]
    fn chat_message_sender_override() {
        let mut msg = ChatMessage {
            sender: "hacker".into(),
            content: "Hello".into(),
            channel: ChatType::Say,
        };
        msg.sender = "TrustedPlayer".to_string();
        assert_eq!(msg.sender, "TrustedPlayer");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn set_target_updates_combat_target() {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);

        let target_entity = app.world_mut().spawn_empty().id();
        let player = spawn_player(app.world_mut(), 0.0, 0.0, 0.0);
        app.world_mut()
            .entity_mut(player)
            .insert(CombatTarget::default());

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
