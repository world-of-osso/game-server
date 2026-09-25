use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::components::{Health, Mounted, Npc, Player, Position, VerticalVelocity};
use shared::death::DeathState;
use shared::protocol::{ChatMessage, LoadTerrain, PlayerInput, TerrainChannel};
use std::collections::VecDeque;

use crate::auth::{
    self, Authenticated, ConnectedAccounts, InWorld, PendingAuth, PendingAuthTimer, SentTiles,
    clear_connected_account,
};
use crate::interest::{InterestDirtyQueue, NetworkVisibility, mark_interest_position_changed};
use crate::messaging;
use crate::persistence::{self, ClientId, GameDatabase};
use crate::terrain_height::{GRAVITY, GROUND_SNAP_THRESHOLD, TerrainHeightMap};

#[path = "networking_movement.rs"]
mod movement;
use movement::*;

const DEFAULT_SERVER_PORT: u16 = 5000;
/// Squared radius for immediate visibility grant on world entry (matches interest system).
const INITIAL_VISIBILITY_RADIUS_SQ: f32 = 100.0 * 100.0;
type LinkStatusData = (Has<Connected>, Has<ClientOf>, Has<ReplicationSender>);
type ConnInfoData = (
    &'static RemoteId,
    Has<ClientOf>,
    Has<LinkOf>,
    Has<ReplicationSender>,
    Has<MessageSender<LoadTerrain>>,
    Has<MessageSender<ChatMessage>>,
    Has<MessageSender<shared::protocol::LoginResponse>>,
);
type ReplicatedData = (
    Entity,
    Has<Player>,
    Has<Npc>,
    Has<NetworkVisibility>,
    Has<ReplicationState>,
);
type DisconnectData = (
    Option<&'static PlayerLink>,
    Option<&'static Authenticated>,
    Option<&'static InWorld>,
);

#[derive(SystemParam)]
struct LinkStatusQuery<'w, 's> {
    query: Query<'w, 's, LinkStatusData, With<LinkOf>>,
}

#[derive(SystemParam)]
struct ConnInfoQuery<'w, 's> {
    query: Query<'w, 's, ConnInfoData, With<Connected>>,
}

#[derive(SystemParam)]
struct ReplicatedQuery<'w, 's> {
    query: Query<'w, 's, ReplicatedData, With<Replicate>>,
}

#[derive(SystemParam)]
struct DisconnectQuery<'w, 's> {
    query: Query<'w, 's, DisconnectData, With<ClientOf>>,
}

/// Links a connection entity to its spawned player entity.
#[derive(Component, Debug)]
pub struct PlayerLink(pub Entity);

/// The entity this player is currently targeting in combat.
#[derive(Component, Debug, Default)]
pub struct CombatTarget(pub Option<Entity>);

/// Whether the player's newest input requests translation. Server-only; a
/// blocking cast freezes `Position`, so casts read intent instead.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Moving(pub bool);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct FallTracker {
    peak_y: f32,
    last_y: f32,
    airborne: bool,
}

impl FallTracker {
    pub(crate) fn grounded(y: f32) -> Self {
        Self {
            peak_y: y,
            last_y: y,
            airborne: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct ChatHistoryBuffer {
    pub entries: VecDeque<ChatHistoryEntry>,
}

/// A routed chat message plus the character names it was delivered to, so
/// history replays only what each requester was allowed to see.
#[derive(Clone, Debug)]
pub struct ChatHistoryEntry {
    pub message: ChatMessage,
    pub recipients: Vec<String>,
}

/// Marks an entity for deferred despawn (next frame), allowing lightyear to process
/// the removal of Replicate/NetworkVisibility before the entity disappears.
#[derive(Component)]
struct PendingDespawn;

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<crate::death::PlayerDeathTransition>();
        app.init_resource::<crate::friends::AutoAfkScheduler>();
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
        app.add_systems(
            Update,
            messaging::send_motd_on_enter_world.after(auth::process_select_character),
        );
        app.add_systems(Update, auth::auth_timeout);
        app.add_systems(Update, auth::deliver_forced_disconnects);
        app.add_observer(on_new_link);
        app.add_observer(on_client_connected);
        app.add_observer(on_client_disconnected);
        app.add_observer(on_player_link_inserted);
    }
}

fn start_server(mut commands: Commands) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), server_port());
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

fn server_port() -> u16 {
    match std::env::var("GAME_SERVER_PORT") {
        Ok(value) => match value.parse::<u16>() {
            Ok(0) | Err(_) => {
                warn!(
                    "Invalid GAME_SERVER_PORT={value:?}; falling back to default {DEFAULT_SERVER_PORT}"
                );
                DEFAULT_SERVER_PORT
            }
            Ok(port) => port,
        },
        Err(_) => DEFAULT_SERVER_PORT,
    }
}

/// When a new transport link is established, insert a ReplicationSender.
fn on_new_link(
    trigger: On<Add, LinkOf>,
    mut commands: Commands,
    links: LinkStatusQuery,
    replicated: Query<(Entity, Has<Player>, Has<Npc>), With<Replicate>>,
) {
    let replicated_count = replicated.iter().count();
    let replicated_players = replicated
        .iter()
        .filter(|(_, has_player, _)| *has_player)
        .count();
    let replicated_npcs = replicated.iter().filter(|(_, _, has_npc)| *has_npc).count();
    if let Ok((connected, client_of, sender)) = links.query.get(trigger.entity) {
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
    commands.entity(trigger.entity).insert(ReplicationSender);
}

/// When a client connects, mark as pending auth (no player entity yet).
fn on_client_connected(
    trigger: On<Add, Connected>,
    query: ConnInfoQuery,
    replicated: ReplicatedQuery,
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
    )) = query.query.get(trigger.entity)
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

fn log_replication_state(replicated: &ReplicatedQuery, client_id: u64, flags: &[bool; 6]) {
    let replicated_count = replicated.query.iter().count();
    let replicated_players = replicated.query.iter().filter(|(_, p, _, _, _)| *p).count();
    let replicated_npcs = replicated.query.iter().filter(|(_, _, n, _, _)| *n).count();
    let visible = replicated.query.iter().filter(|(_, _, _, v, _)| *v).count();
    let stateful = replicated.query.iter().filter(|(_, _, _, _, s)| *s).count();

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

type DisconnectPlayer = (
    &'static Position,
    &'static Health,
    &'static Player,
    &'static ClientId,
    Option<&'static crate::death::PlayerDeathState>,
);

/// When a client disconnects, save data and clean up.
fn on_client_disconnected(
    trigger: On<Remove, Connected>,
    conn_query: DisconnectQuery,
    player_query: Query<DisconnectPlayer>,
    db: Option<Res<GameDatabase>>,
    mut connected: ResMut<ConnectedAccounts>,
    mut commands: Commands,
) {
    let Ok((player_link, auth, in_world)) = conn_query.query.get(trigger.entity) else {
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
    player_query: &Query<DisconnectPlayer>,
    entity: Entity,
    in_world: Option<&InWorld>,
    commands: &mut Commands,
) {
    if let (Some(db), Some(_)) = (db.as_ref(), in_world)
        && let Ok((pos, health, player, cid, death)) = player_query.get(entity)
    {
        persistence::save_live_character(&db.db, cid.0, player, pos, health, death);
        info!("Saved character data for character_id {}", cid.0);
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
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut vis_query: Query<(Entity, &Position, &mut NetworkVisibility)>,
    mut commands: Commands,
) {
    let connection = trigger.entity;
    let Ok(link) = link_query.get(connection) else {
        return;
    };
    let Ok(player_position) = positions.get(link.0) else {
        return;
    };

    let mut nearby_count = 0u32;
    for (entity, position, mut visibility) in &mut vis_query {
        let dx = position.x - player_position.x;
        let dy = position.y - player_position.y;
        let dz = position.z - player_position.z;
        let is_self = entity == link.0;
        let is_nearby = dx * dx + dy * dy + dz * dz <= INITIAL_VISIBILITY_RADIUS_SQ;

        if is_self || is_nearby {
            visibility.gain_visibility(connection);
            commands.gain_visibility(entity, connection);
            nearby_count += u32::from(!is_self);
        } else {
            visibility.lose_visibility(connection);
            commands.lose_visibility(entity, connection);
        }
        if let Some(replication_dirty) = replication_dirty.as_deref() {
            replication_dirty.mark_for_sender(entity, connection);
        }
    }

    info!(
        conn = ?connection,
        player = ?link.0,
        nearby = nearby_count,
        "Granted immediate visibility for player + {nearby_count} nearby entities"
    );
}

/// Send the initial terrain loading command for the tile under the entering character;
/// the client's loading screen waits for exactly this tile.
pub fn send_load_terrain(
    connection: Entity,
    position: &Position,
    senders: &mut Query<&mut MessageSender<LoadTerrain>>,
) {
    let Ok(mut sender) = senders.get_mut(connection) else {
        warn!("No MessageSender<LoadTerrain> on connection entity");
        return;
    };
    let (tile_y, tile_x) = position_tile(position);
    sender.send::<TerrainChannel>(LoadTerrain {
        map_name: "azeroth".into(),
        initial_tile_y: tile_y,
        initial_tile_x: tile_x,
    });
    info!("Sent LoadTerrain: azeroth tile ({tile_y}, {tile_x})");
}

#[derive(SystemParam)]
pub(crate) struct PlayerInputParams<'w, 's> {
    commands: Commands<'w, 's>,
    receivers: Query<
        'w,
        's,
        (
            &'static mut MessageReceiver<PlayerInput>,
            &'static PlayerLink,
        ),
    >,
    players: Query<'w, 's, PlayerMovement, With<Player>>,
    presence: Query<
        'w,
        's,
        (
            &'static mut shared::components::PresenceStatus,
            &'static mut crate::friends::PresenceTracker,
        ),
        With<Player>,
    >,
    mounted: Query<'w, 's, Entity, (With<Player>, With<Mounted>)>,
    auto_afk: ResMut<'w, crate::friends::AutoAfkScheduler>,
    dirty_queue: Option<Res<'w, InterestDirtyQueue>>,
    replication_dirty: Option<Res<'w, ReplicationDirtyQueue>>,
}

/// Read PlayerInput messages and apply movement.
pub(crate) fn process_player_inputs(time: Res<Time>, mut params: PlayerInputParams) {
    for (mut receiver, link) in &mut params.receivers {
        let mut latest = None;
        let mut jumping = false;
        let mut swimming_seen = false;
        for input in receiver.receive() {
            jumping |= input.jumping;
            swimming_seen |= input.swimming;
            latest = Some(input);
        }
        if let Some(mut input) = latest {
            input.jumping = jumping;
            crate::friends::mark_player_activity(
                link.0,
                &mut params.presence,
                &mut params.auto_afk,
                time.elapsed(),
                params.replication_dirty.as_deref(),
            );
            if swimming_seen && params.mounted.contains(link.0) {
                params.commands.entity(link.0).remove::<Mounted>();
            }
            let changes = apply_movement_input(&input, link.0, &mut params.players, &time);
            params
                .commands
                .entity(link.0)
                .insert(Moving(direction_length(&input.direction) >= 0.001));
            if changes.position_changed {
                mark_interest_position_changed(params.dirty_queue.as_deref(), link.0);
            }
            if changes.replicated_state_changed
                && let Some(replication_dirty) = params.replication_dirty.as_deref()
            {
                replication_dirty.mark(link.0);
            }
        }
    }
}

type GravityPlayerData<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Position,
        &'static mut VerticalVelocity,
        &'static mut FallTracker,
        &'static mut Health,
        Option<&'static crate::death::PlayerDeathState>,
    ),
    With<Player>,
>;

/// Apply gravity and terrain clamping to all players each tick.
pub(crate) fn apply_terrain_gravity(
    terrain: Option<Res<TerrainHeightMap>>,
    mut players: GravityPlayerData,
    time: Res<Time>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut player_deaths: MessageWriter<crate::death::PlayerDeathTransition>,
) {
    let Some(terrain) = terrain.as_ref().filter(|t| t.is_loaded()) else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut pos, mut vert_vel, mut fall_tracker, mut health, death_state) in
        players.iter_mut()
    {
        let previous_health = *health;
        let (next_pos, next_vert_vel, next_fall_tracker, next_health) = calculate_gravity_state(
            terrain,
            *pos,
            *vert_vel,
            *fall_tracker,
            *health,
            death_state,
            dt,
        );
        if *pos != next_pos {
            *pos = next_pos;
            mark_interest_position_changed(dirty_queue.as_deref(), entity);
        }
        if *vert_vel != next_vert_vel {
            *vert_vel = next_vert_vel;
        }
        if *fall_tracker != next_fall_tracker {
            *fall_tracker = next_fall_tracker;
        }
        apply_gravity_health_transition(
            entity,
            previous_health,
            next_health,
            &mut health,
            replication_dirty.as_deref(),
            &mut player_deaths,
        );
    }
}

fn apply_gravity_health_transition(
    player_entity: Entity,
    previous_health: Health,
    next_health: Health,
    health: &mut Mut<Health>,
    replication_dirty: Option<&ReplicationDirtyQueue>,
    player_deaths: &mut MessageWriter<crate::death::PlayerDeathTransition>,
) {
    if **health != next_health {
        **health = next_health;
        if let Some(replication_dirty) = replication_dirty {
            replication_dirty.mark(player_entity);
        }
    }
    if previous_health.current > 0.0 && next_health.current <= 0.0 {
        player_deaths.write(crate::death::PlayerDeathTransition {
            player: player_entity,
        });
    }
}

fn calculate_gravity_state(
    terrain: &TerrainHeightMap,
    mut pos: Position,
    mut vert_vel: VerticalVelocity,
    mut fall_tracker: FallTracker,
    mut health: Health,
    death_state: Option<&crate::death::PlayerDeathState>,
    dt: f32,
) -> (Position, VerticalVelocity, FallTracker, Health) {
    if death_state.is_some_and(|state| state.state != DeathState::Alive) {
        fall_tracker.last_y = pos.y;
        fall_tracker.peak_y = pos.y;
        fall_tracker.airborne = false;
    } else {
        clamp_to_terrain(
            terrain,
            &mut pos,
            &mut vert_vel,
            &mut fall_tracker,
            &mut health,
            dt,
        );
    }
    (pos, vert_vel, fall_tracker, health)
}

/// Clamp a position (Bevy space) to terrain: apply vertical velocity and gravity.
fn clamp_to_terrain(
    terrain: &TerrainHeightMap,
    pos: &mut Position,
    vert_vel: &mut VerticalVelocity,
    fall_tracker: &mut FallTracker,
    health: &mut Health,
    dt: f32,
) {
    let Some(ground_y) = terrain.height_at_bevy(pos.x, pos.z) else {
        return;
    };

    // Apply vertical velocity (jump impulse)
    if vert_vel.0 != 0.0 {
        pos.y += vert_vel.0 * dt;
        vert_vel.0 -= GRAVITY * dt;
    }

    if pos.y < ground_y {
        pos.y = ground_y;
        vert_vel.0 = 0.0;
    } else if vert_vel.0 == 0.0 && pos.y > ground_y + GROUND_SNAP_THRESHOLD {
        pos.y -= GRAVITY * dt;
        if pos.y < ground_y {
            pos.y = ground_y;
        }
    }

    apply_fall_damage_on_landing(fall_tracker, pos.y, ground_y, vert_vel.0, health);
}

fn apply_fall_damage_on_landing(
    fall_tracker: &mut FallTracker,
    current_y: f32,
    ground_y: f32,
    vertical_velocity: f32,
    health: &mut Health,
) {
    let now_airborne = current_y > ground_y + GROUND_SNAP_THRESHOLD || vertical_velocity > 0.0;
    if !fall_tracker.airborne && now_airborne {
        fall_tracker.airborne = true;
        fall_tracker.peak_y = fall_tracker.last_y.max(current_y).max(ground_y);
    } else if fall_tracker.airborne {
        fall_tracker.peak_y = fall_tracker.peak_y.max(fall_tracker.last_y).max(current_y);
    }

    let landed = fall_tracker.airborne && vertical_velocity == 0.0 && current_y <= ground_y;
    if landed {
        let fall_distance = (fall_tracker.peak_y - ground_y).max(0.0);
        let damage = health.max * shared::movement::fall_damage_percent(fall_distance);
        if damage > 0.0 {
            crate::death::apply_health_damage(health, damage);
        }
        fall_tracker.airborne = false;
        fall_tracker.peak_y = ground_y;
    }

    fall_tracker.last_y = current_y;
}

const TILE_SIZE: f32 = 533.33;
/// WoW map origin offset (32 tiles * TILE_SIZE).
const MAP_ORIGIN: f32 = 17066.67;

fn wow_to_tile(wow_x: f32, wow_y: f32) -> (u32, u32) {
    let row = ((MAP_ORIGIN - wow_y) / TILE_SIZE).floor() as i32;
    let col = ((MAP_ORIGIN - wow_x) / TILE_SIZE).floor() as i32;
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
        let current_tile = position_tile(pos);
        if sent.last_tile == Some(current_tile) {
            continue;
        }
        sent.last_tile = Some(current_tile);
        send_nearby_tiles(conn_entity, current_tile, &mut sent, &mut senders);
    }
}

fn position_tile(pos: &Position) -> (u32, u32) {
    let (wow_x, wow_y, _) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
    wow_to_tile(wow_x, wow_y)
}

fn send_nearby_tiles(
    conn_entity: Entity,
    (tile_y, tile_x): (u32, u32),
    sent: &mut SentTiles,
    senders: &mut Query<&mut MessageSender<LoadTerrain>>,
) {
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
#[path = "networking_tests.rs"]
mod tests;
