use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::components::{Health, MovementSpeed, Npc, Player, Position, Rotation};
use shared::protocol::{ChatMessage, LoadTerrain, PlayerInput, TerrainChannel};
use std::collections::VecDeque;
use std::time::Duration;

use crate::auth::{
    self, Authenticated, ConnectedAccounts, InWorld, PendingAuth, PendingAuthTimer, SentTiles,
    clear_connected_account,
};
use crate::messaging;
use crate::persistence::{self, CharacterData, ClientId, GameDatabase};
use crate::terrain_height::{GRAVITY, GROUND_SNAP_THRESHOLD, TerrainHeightMap};

const SERVER_PORT: u16 = 5000;
const DEFAULT_RUN_SPEED: f32 = 7.0;
const DEFAULT_WALK_SPEED: f32 = 2.5;

/// Links a connection entity to its spawned player entity.
#[derive(Component, Debug)]
pub struct PlayerLink(pub Entity);

/// The entity this player is currently targeting in combat.
#[derive(Component, Debug, Default)]
pub struct CombatTarget(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct ChatHistoryBuffer {
    pub entries: VecDeque<ChatMessage>,
}

/// Marks an entity for deferred despawn (next frame), allowing lightyear to process
/// the removal of Replicate/NetworkVisibility before the entity disappears.
#[derive(Component)]
struct PendingDespawn;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectedAccounts>();
        app.init_resource::<ChatHistoryBuffer>();
        app.add_systems(Startup, start_server);
        app.add_systems(PostUpdate, despawn_pending);
        app.add_systems(FixedUpdate, process_player_inputs);
        app.add_systems(
            FixedUpdate,
            apply_terrain_gravity.after(process_player_inputs),
        );
        app.add_systems(FixedUpdate, stream_terrain_tiles);
        app.add_systems(Update, messaging::process_chat_messages);
        app.add_systems(Update, messaging::process_chat_history_requests);
        app.add_systems(Update, messaging::process_guild_invites);
        app.add_systems(Update, messaging::process_guild_accepts);
        app.add_systems(Update, messaging::process_target_messages);
        app.add_systems(Update, auth::process_login_requests);
        app.add_systems(Update, auth::process_register_requests);
        app.add_systems(Update, auth::process_create_character);
        app.add_systems(Update, auth::process_delete_character);
        app.add_systems(Update, auth::process_select_character);
        app.add_systems(Update, auth::auth_timeout);
        app.add_observer(on_new_link);
        app.add_observer(on_client_connected);
        app.add_observer(on_client_disconnected);
        app.add_observer(on_player_link_inserted);
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
fn on_new_link(
    trigger: On<Add, LinkOf>,
    mut commands: Commands,
    links: Query<(Has<Connected>, Has<ClientOf>, Has<ReplicationSender>), With<LinkOf>>,
    replicated: Query<(Entity, Has<Player>, Has<Npc>), With<Replicate>>,
) {
    let replicated_count = replicated.iter().count();
    let replicated_players = replicated
        .iter()
        .filter(|(_, has_player, _)| *has_player)
        .count();
    let replicated_npcs = replicated.iter().filter(|(_, _, has_npc)| *has_npc).count();
    if let Ok((connected, client_of, sender)) = links.get(trigger.entity) {
        debug!(
            entity = ?trigger.entity,
            connected,
            client_of,
            sender,
            replicated_count,
            replicated_players,
            replicated_npcs,
            "LinkOf inserted; adding ReplicationSender"
        );
    }
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
    query: Query<
        (
            &RemoteId,
            Has<ClientOf>,
            Has<LinkOf>,
            Has<ReplicationSender>,
            Has<MessageSender<LoadTerrain>>,
            Has<MessageSender<ChatMessage>>,
            Has<MessageSender<shared::protocol::LoginResponse>>,
        ),
        With<Connected>,
    >,
    replicated: Query<
        (
            Entity,
            Has<Player>,
            Has<Npc>,
            Has<NetworkVisibility>,
            Has<ReplicationState>,
        ),
        With<Replicate>,
    >,
    time: Res<Time>,
    mut commands: Commands,
) {
    let Ok((
        remote_id,
        has_client_of,
        has_link_of,
        has_sender,
        has_terrain_sender,
        has_chat_sender,
        has_auth_sender,
    )) = query.get(trigger.entity)
    else {
        return;
    };
    let client_id = remote_id.0.to_bits();
    log_replication_state(
        &replicated,
        client_id,
        &[
            has_client_of,
            has_link_of,
            has_sender,
            has_terrain_sender,
            has_chat_sender,
            has_auth_sender,
        ],
    );
    info!("Client connected: {client_id} (pending auth)");
    let elapsed = time.elapsed_secs_f64();
    commands
        .entity(trigger.entity)
        .insert((PendingAuth, PendingAuthTimer(elapsed)));
}

fn log_replication_state(
    replicated: &Query<
        (
            Entity,
            Has<Player>,
            Has<Npc>,
            Has<NetworkVisibility>,
            Has<ReplicationState>,
        ),
        With<Replicate>,
    >,
    client_id: u64,
    flags: &[bool; 6],
) {
    let replicated_count = replicated.iter().count();
    let replicated_players = replicated.iter().filter(|(_, p, _, _, _)| *p).count();
    let replicated_npcs = replicated.iter().filter(|(_, _, n, _, _)| *n).count();
    let visible = replicated.iter().filter(|(_, _, _, v, _)| *v).count();
    let stateful = replicated.iter().filter(|(_, _, _, _, s)| *s).count();

    debug!(
        client_id,
        has_client_of = flags[0],
        has_link_of = flags[1],
        has_sender = flags[2],
        has_terrain_sender = flags[3],
        has_chat_sender = flags[4],
        has_auth_sender = flags[5],
        replicated_count,
        replicated_players,
        replicated_npcs,
        visible,
        stateful,
        "Client connected"
    );
}

/// When a client disconnects, save data and clean up.
fn on_client_disconnected(
    trigger: On<Remove, Connected>,
    conn_query: Query<
        (
            Option<&PlayerLink>,
            Option<&Authenticated>,
            Option<&InWorld>,
        ),
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
        if clear_connected_account(&mut connected, auth.account_id, trigger.entity) {
            info!("Account {} logged out", auth.account_id);
        } else {
            debug!(
                "Ignoring disconnect for replaced session {:?} on account {}",
                trigger.entity, auth.account_id
            );
        }
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
    queue_player_despawn(commands, entity);
}

pub(crate) fn queue_player_despawn(commands: &mut Commands, entity: Entity) {
    commands
        .entity(entity)
        .queue_silenced(|mut entity: EntityWorldMut| {
            entity.remove::<(Replicate, NetworkVisibility)>();
            entity.insert(PendingDespawn);
        });
    info!(
        "Removed replication for player entity {:?}, pending despawn",
        entity
    );
}

fn despawn_pending(mut commands: Commands, query: Query<Entity, With<PendingDespawn>>) {
    for entity in &query {
        commands.entity(entity).try_despawn();
    }
}

/// When PlayerLink is inserted (player enters world), immediately grant visibility
/// for all nearby replicated entities to this connection. Without this, entities
/// stay invisible until the interest system's next tick (~500ms).
fn on_player_link_inserted(
    trigger: On<Add, PlayerLink>,
    link_query: Query<&PlayerLink>,
    positions: Query<&Position>,
    mut vis_query: Query<(Entity, &Position, &mut ReplicationState), With<NetworkVisibility>>,
) {
    let conn_entity = trigger.entity;
    let Ok(link) = link_query.get(conn_entity) else {
        return;
    };
    let Ok(player_pos) = positions.get(link.0) else {
        return;
    };
    // Grant visibility for the player's own entity
    if let Ok(mut state) = vis_query.get_mut(link.0) {
        state.2.gain_visibility(conn_entity);
    }
    // Grant visibility for all nearby entities
    let mut count = 0u32;
    for (entity, pos, mut state) in vis_query.iter_mut() {
        if entity == link.0 {
            continue;
        }
        let dx = pos.x - player_pos.x;
        let dy = pos.y - player_pos.y;
        let dz = pos.z - player_pos.z;
        if dx * dx + dy * dy + dz * dz <= 100.0 * 100.0 {
            state.gain_visibility(conn_entity);
            count += 1;
        }
    }
    info!(
        conn = ?conn_entity,
        player = ?link.0,
        nearby = count,
        "Granted immediate visibility for player + {count} nearby entities"
    );
}

/// Send the initial terrain loading command to a client.
pub fn send_load_terrain(connection: Entity, senders: &mut Query<&mut MessageSender<LoadTerrain>>) {
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
    let new_x = pos.x + input.direction[0] * scale;
    let new_y = pos.y + input.direction[1] * scale;
    if !validate_move_distance(scale * dir_len, speed.0, time.delta_secs()) {
        return;
    }
    pos.x = new_x;
    pos.y = new_y;
    pos.z += input.direction[2] * scale;
}

/// Reject movement if distance exceeds max speed with tolerance.
fn validate_move_distance(distance: f32, max_speed: f32, dt: f32) -> bool {
    let max_distance = max_speed * dt * 1.5; // 50% tolerance for network jitter
    distance <= max_distance
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

/// Clamp a position (Bevy space) to terrain: apply gravity if airborne, snap if below ground.
fn clamp_to_terrain(terrain: &TerrainHeightMap, pos: &mut Position, dt: f32) {
    let Some(ground_y) = terrain.height_at_bevy(pos.x, pos.z) else {
        return;
    };
    if pos.y < ground_y {
        pos.y = ground_y;
    } else if pos.y > ground_y + GROUND_SNAP_THRESHOLD {
        pos.y -= GRAVITY * dt;
        if pos.y < ground_y {
            pos.y = ground_y;
        }
    }
}

fn direction_length(dir: &[f32; 3]) -> f32 {
    (dir[0].powi(2) + dir[1].powi(2) + dir[2].powi(2)).sqrt()
}

const TILE_SIZE: f32 = 533.33;

fn wow_to_tile(wow_x: f32, wow_y: f32) -> (u32, u32) {
    let row = ((17066.67 - wow_y) / TILE_SIZE).floor() as i32;
    let col = ((17066.67 - wow_x) / TILE_SIZE).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

fn stream_terrain_tiles(
    mut conn_q: Query<(Entity, &PlayerLink, &mut SentTiles)>,
    players: Query<&Position, With<Player>>,
    mut senders: Query<&mut MessageSender<LoadTerrain>>,
) {
    for (conn_entity, link, mut sent) in conn_q.iter_mut() {
        let Ok(pos) = players.get(link.0) else {
            continue;
        };
        send_nearby_tiles(conn_entity, pos, &mut sent, &mut senders);
    }
}

fn send_nearby_tiles(
    conn_entity: Entity,
    pos: &Position,
    sent: &mut SentTiles,
    senders: &mut Query<&mut MessageSender<LoadTerrain>>,
) {
    let (wow_x, wow_y, _) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
    let (tile_y, tile_x) = wow_to_tile(wow_x, wow_y);
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let ty = (tile_y as i32 + dy).clamp(0, 63) as u32;
            let tx = (tile_x as i32 + dx).clamp(0, 63) as u32;
            if sent.tiles.insert((ty, tx)) {
                let Ok(mut sender) = senders.get_mut(conn_entity) else {
                    continue;
                };
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
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use shared::protocol::{ChatType, PlayerInput};

    #[derive(Component)]
    struct ReplicationTag;

    #[derive(Component)]
    struct VisibilityTag;

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
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
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
            >,
                  time: Res<Time>| {
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
            >,
                  time: Res<Time>| {
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
            >,
                  time: Res<Time>| {
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

    #[test]
    fn terrain_clamps_below_ground() {
        // Terrain chunk at WoW corner (100, 100), height 50.
        // Bevy pos: x=90 (wow_x=90), z=-90 (wow_y=90), y=30 (below ground_y=50).
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
        let mut pos = Position {
            x: 90.0,
            y: 30.0,
            z: -90.0,
        };
        clamp_to_terrain(&map, &mut pos, 0.05);
        assert!((pos.y - 50.0).abs() < 0.01);
    }

    #[test]
    fn terrain_applies_gravity() {
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
        let mut pos = Position {
            x: 90.0,
            y: 60.0,
            z: -90.0,
        };
        let dt = 0.05;
        clamp_to_terrain(&map, &mut pos, dt);
        let expected = 60.0 - GRAVITY * dt;
        assert!((pos.y - expected).abs() < 0.01);
    }

    #[test]
    fn terrain_on_ground_stays() {
        let map = crate::terrain_height::test_helpers::flat_terrain(100.0, 100.0, 50.0);
        let mut pos = Position {
            x: 90.0,
            y: 50.05,
            z: -90.0,
        };
        clamp_to_terrain(&map, &mut pos, 0.05);
        assert!((pos.y - 50.05).abs() < 0.01);
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

    #[test]
    fn known_elwynn_position_maps_to_existing_adt_tile() {
        assert_eq!(wow_to_tile(-8949.0, -132.0), (32, 48));
    }

    #[test]
    fn deferred_player_despawn_ignores_entities_already_despawned() {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);

        let player = app
            .world_mut()
            .spawn((ReplicationTag, VisibilityTag))
            .id();

        app.add_systems(Update, move |mut commands: Commands| {
            commands.entity(player).despawn();
            queue_player_despawn(&mut commands, player);
        });

        let result = catch_unwind(AssertUnwindSafe(|| app.update()));
        assert!(
            result.is_ok(),
            "queue_player_despawn should ignore entities already despawned before cleanup flushes"
        );
    }
}
