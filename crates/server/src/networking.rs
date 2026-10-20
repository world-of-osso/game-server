use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use core::net::SocketAddr;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use shared::components::{Health, Mounted, Npc, Player, PlayerMotion, Position, VerticalVelocity};
use shared::death::DeathState;
use shared::protocol::{
    ChatMessage, CombatLogEvent, EnvironmentalKind, LoadTerrain, PlayerInput, TerrainChannel,
};
use std::collections::VecDeque;

use crate::auth::{
    self, Authenticated, ConnectedAccounts, InWorld, PendingAuth, PendingAuthTimer, SentTiles,
    clear_connected_account,
};
use crate::ground::{GroundMap, GroundProbe};
use crate::interest::{InterestDirtyQueue, NetworkVisibility, mark_interest_position_changed};
use crate::maps::InMap;
use crate::messaging;
use crate::persistence::{self, ClientId, GameDatabase};
use crate::terrain_height::{GRAVITY, GROUND_SNAP_THRESHOLD};

#[path = "networking_movement.rs"]
mod movement;
use movement::*;

#[path = "networking_binding.rs"]
mod binding;
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

/// Turn a predicted player without moving it or restarting an ongoing fall.
/// The matching epoch rejects pre-turn input until the client adopts ForcedFacing.
pub(crate) fn face_player(world: &mut World, player: Entity, yaw: f32) {
    let epoch = {
        let mut control = world
            .get_mut::<shared::components::MovementControl>(player)
            .expect("validated player movement control");
        control.epoch = control.epoch.wrapping_add(1);
        control.epoch
    };
    if let Some(mut fall) = world.get_mut::<FallTracker>(player) {
        fall.epoch = epoch;
    }
    world
        .get_mut::<shared::components::Rotation>(player)
        .expect("validated player rotation")
        .y = yaw;
    world
        .entity_mut(player)
        .insert(shared::components::ForcedFacing { epoch, yaw });
}

/// Whether the player's newest input requests translation. Server-only; a
/// blocking cast freezes `Position`, so casts read intent instead.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Moving(pub bool);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub(crate) struct FallTracker {
    peak_y: f32,
    last_y: f32,
    airborne: bool,
    /// Height the player left the ground from, and seconds since (while `airborne`).
    takeoff_y: f32,
    air_secs: f32,
    /// Upward speed the player left the ground with: `JUMP_IMPULSE` when a jumping input
    /// took off, 0 for walking off a ledge.
    jump_impulse: f32,
    /// An input set the height this tick: gravity leaves it to the client's physics.
    height_reported: bool,
    /// `MovementControl::epoch` of the position being tracked.
    epoch: u32,
}

impl FallTracker {
    pub(crate) fn grounded(y: f32) -> Self {
        Self {
            peak_y: y,
            last_y: y,
            airborne: false,
            takeoff_y: y,
            air_secs: 0.0,
            jump_impulse: 0.0,
            height_reported: false,
            epoch: 0,
        }
    }

    pub(crate) fn restart_at(&mut self, y: f32) {
        *self = Self {
            epoch: self.epoch,
            ..Self::grounded(y)
        };
    }

    /// A server reposition (new `MovementControl::epoch`) is not a fall: tracking
    /// restarts at the new height.
    fn for_epoch(self, epoch: u32, y: f32) -> Self {
        if self.epoch == epoch {
            self
        } else {
            Self {
                epoch,
                ..Self::grounded(y)
            }
        }
    }

    /// Off the ground: where from, how fast up and for how long.
    fn airborne_state(&self) -> Option<movement::Airborne> {
        self.airborne.then_some(movement::Airborne {
            takeoff_y: self.takeoff_y,
            impulse: self.jump_impulse,
            secs: self.air_secs,
        })
    }

    /// An input set the height this tick; a grounded one that jumps takes off with
    /// `JUMP_IMPULSE`.
    fn report_height(&mut self, jumping: bool) {
        self.height_reported = true;
        if !self.airborne {
            self.jump_impulse = if jumping {
                shared::movement::JUMP_IMPULSE
            } else {
                0.0
            };
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
pub(crate) struct PendingDespawn;

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
    let addr = server_address();
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

fn server_address() -> SocketAddr {
    let read = |name: &str| {
        std::env::var_os(name).map(|value| {
            value
                .into_string()
                .unwrap_or_else(|_| panic!("{name} must be valid UTF-8"))
        })
    };
    let ip = read("GAME_SERVER_BIND_IP");
    let port = read("GAME_SERVER_PORT");
    binding::parse_server_address(ip.as_deref(), port.as_deref())
        .unwrap_or_else(|error| panic!("server binding configuration: {error}"))
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
    Option<&'static InMap>,
    Option<&'static crate::maps::MapTransfer>,
    Option<&'static crate::rest::PlayerRestState>,
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
        && let Ok((pos, health, player, cid, death, map, transfer, rest)) = player_query.get(entity)
    {
        let observed_logout = crate::rest::unix_secs();
        let map_id = crate::maps::saved_map_id(map, transfer);
        persistence::save_live_character(&db.db, cid.0, player, pos, health, death, map_id);
        persistence::save_character_last_logout(&db.db, cid.0, observed_logout);
        if let Some(rest) = rest {
            let data = rest.logout_data(crate::rest::unix_secs());
            crate::persistence_progression::save_rest(&db.db, cid.0, &data);
        }
        info!("Saved character data for character_id {}", cid.0);
    }
    queue_player_despawn(commands, entity);
}

pub(crate) fn queue_player_despawn(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| {
        crate::pvp::reconnect::logged_out(world, entity);
        crate::hunter_pet::persist_current_pet_health(world, entity);
        // The transport can outlive its retired player while ForcedDisconnect is
        // delivered. Revoke its world link before PostUpdate removes that player.
        let mut links = world.query::<(Entity, &PlayerLink)>();
        let connections: Vec<_> = links
            .iter(world)
            .filter(|(_, link)| link.0 == entity)
            .map(|(connection, _)| connection)
            .collect();
        for connection in connections {
            world
                .entity_mut(connection)
                .remove::<(PlayerLink, InWorld)>();
        }
    });
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

#[cfg(test)]
pub(crate) fn install_disconnect_fixture(app: &mut App) {
    app.init_resource::<ConnectedAccounts>();
    app.add_observer(on_client_disconnected);
    app.add_systems(PreUpdate, despawn_pending);
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
    positions: Query<(&Position, Option<&InMap>)>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut vis_query: Query<(Entity, &Position, Option<&InMap>, &mut NetworkVisibility)>,
    mut commands: Commands,
) {
    let connection = trigger.entity;
    let Ok(link) = link_query.get(connection) else {
        return;
    };
    let Ok((player_position, player_map)) = positions.get(link.0) else {
        return;
    };

    let mut nearby_count = 0u32;
    for (entity, position, map, mut visibility) in &mut vis_query {
        let dx = position.x - player_position.x;
        let dy = position.y - player_position.y;
        let dz = position.z - player_position.z;
        let is_self = entity == link.0;
        let is_nearby = player_map.is_some()
            && map == player_map
            && dx * dx + dy * dy + dz * dz <= INITIAL_VISIBILITY_RADIUS_SQ;

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
/// `map_directory` is the map's lowercase `Map.db2` Directory.
pub fn send_load_terrain(
    connection: Entity,
    map_directory: &str,
    position: &Position,
    senders: &mut Query<&mut MessageSender<LoadTerrain>>,
) {
    let Ok(mut sender) = senders.get_mut(connection) else {
        warn!("No MessageSender<LoadTerrain> on connection entity");
        return;
    };
    let (tile_y, tile_x) = position_tile(position);
    sender.send::<TerrainChannel>(LoadTerrain {
        map_name: map_directory.into(),
        initial_tile_y: tile_y,
        initial_tile_x: tile_x,
    });
    info!("Sent LoadTerrain: {map_directory} tile ({tile_y}, {tile_x})");
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
    /// A player in a map transfer (no `InMap`) does not move.
    on_map: Query<'w, 's, (), (With<Player>, With<InMap>)>,
    death_states: Query<'w, 's, &'static crate::death::PlayerDeathState>,
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
    controls: Query<'w, 's, &'static shared::components::MovementControl>,
    passengers: Query<'w, 's, &'static shared::components::TransportPassenger>,
    ships: crate::transport::ShipQuery<'w, 's>,
    auto_afk: ResMut<'w, crate::friends::AutoAfkScheduler>,
    dirty_queue: Option<Res<'w, InterestDirtyQueue>>,
    replication_dirty: Option<Res<'w, ReplicationDirtyQueue>>,
    clocks: Local<'s, MovementClocks>,
    impulses: Query<'w, 's, (Entity, &'static crate::skyriding::SkyridingImpulse)>,
    ground: Option<ResMut<'w, GroundMap>>,
    maps: Query<'w, 's, &'static InMap>,
}

/// Read PlayerInput messages and move each player to the positions its client reports,
/// within the player's movement bank. A player without new input moves on to the report
/// its bank has not reached yet.
pub(crate) fn process_player_inputs(time: Res<Time>, mut params: PlayerInputParams) {
    params.clocks.advance(time.delta_secs());
    let players = &params.players;
    params
        .clocks
        .retain_players(|player| players.contains(player));
    for (player, impulse) in &params.impulses {
        params.clocks.add_impulse(player, impulse.0);
        params
            .commands
            .entity(player)
            .remove::<crate::skyriding::SkyridingImpulse>();
    }
    for (mut receiver, link) in &mut params.receivers {
        let control = params
            .controls
            .get(link.0)
            .ok()
            .copied()
            .unwrap_or_default();
        // Input sent before the client adopted a server teleport is stale
        // (`HandleMovementOpcode` ignores movement while `IsBeingTeleported`).
        let received: Vec<PlayerInput> = receiver
            .receive()
            .filter(|input| input.epoch == control.epoch)
            .collect();
        let resumed = received.is_empty();
        let inputs = if resumed {
            match params.clocks.unreached(link.0, control.epoch) {
                Some(unreached) => vec![unreached],
                None => continue,
            }
        } else {
            received
        };
        let Some(latest) = inputs.last().cloned() else {
            continue;
        };
        // The server drives a controlled player (taxi flight); its input is dropped.
        let controlled = control.controlled;
        // A corpse (dead, not yet released) does not move.
        let corpse = params
            .death_states
            .get(link.0)
            .is_ok_and(|death| death.state == DeathState::Dead);
        if controlled || corpse || !params.on_map.contains(link.0) {
            params.clocks.track(link.0, latest, false);
            stop_held_player(
                link.0,
                &mut params.players,
                params.replication_dirty.as_deref(),
            );
            continue;
        }
        let moving = direction_length(&latest.direction) >= 0.001;
        if !moving {
            debug!(player = ?link.0, epoch = latest.epoch, position = ?latest.position, resumed, transport = ?latest.transport, "BG_TRAVEL_STATIONARY_INPUT");
        }
        if !resumed {
            crate::friends::mark_player_activity(
                link.0,
                &mut params.presence,
                &mut params.auto_afk,
                time.elapsed(),
                params.replication_dirty.as_deref(),
            );
        }
        if inputs.iter().any(|input| input.swimming) && params.mounted.contains(link.0) {
            let player = link.0;
            params.commands.queue(move |world: &mut World| {
                if !crate::auras::mount::cancel_mount_aura(world, player) {
                    world.entity_mut(player).remove::<Mounted>();
                }
            });
        }
        let map = params.maps.get(link.0).copied().ok();
        let mut changes = MovementChanges::default();
        let passenger = params.passengers.get(link.0).ok().copied();
        let ship = passenger
            .and_then(|passenger| crate::transport::movement_frame(&params.ships, passenger));
        for input in inputs {
            let input = match crate::transport_movement::resolve_input(
                &input,
                passenger,
                ship,
                map.expect("validated player movement map"),
            ) {
                Ok(input) => input,
                Err(error) => {
                    warn!(player=?link.0, "Transport movement refused: {error}");
                    continue;
                }
            };
            if passenger.is_some() && input.transport.is_none() {
                let player = link.0;
                params
                    .commands
                    .queue(move |world: &mut World| crate::transport::disembark(world, player));
            }
            let terrain = params.ground.as_deref_mut().zip(map);
            changes = changes.merge(apply_movement_input(
                &input,
                link.0,
                &mut params.players,
                &mut params.clocks,
                terrain,
            ));
        }
        params.clocks.track(link.0, latest, changes.behind);
        params.commands.entity(link.0).insert(Moving(moving));
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

/// A player whose input the server drops (taxi, corpse, map transfer) stops moving for
/// other clients.
fn stop_held_player(
    player: Entity,
    players: &mut Query<PlayerMovement, With<Player>>,
    replication_dirty: Option<&ReplicationDirtyQueue>,
) {
    let Ok((.., mut motion, _)) = players.get_mut(player) else {
        return;
    };
    if stop_motion(&mut motion).replicated_state_changed
        && let Some(replication_dirty) = replication_dirty
    {
        replication_dirty.mark(player);
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
        &'static shared::components::MovementControl,
        &'static InMap,
        Has<OutOfBounds>,
        Option<&'static mut PlayerMotion>,
        Option<&'static shared::components::FeatherFall>,
        Option<&'static crate::auras::ActiveAuras>,
    ),
    With<Player>,
>;

/// `PLAYER_FLAGS_IS_OUT_OF_BOUNDS`: the player fell below `Map::GetMinHeight`. Cleared once
/// they are above it again (a release or teleport moves them back).
#[derive(Component, Debug)]
pub struct OutOfBounds;

/// Apply gravity and ground clamping (terrain and WMO floors) to all players each tick.
pub(crate) fn apply_terrain_gravity(
    ground: Option<ResMut<GroundMap>>,
    mut players: GravityPlayerData,
    time: Res<Time>,
    dirty_queue: Option<Res<InterestDirtyQueue>>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut player_deaths: MessageWriter<crate::death::PlayerDeathTransition>,
    mut commands: Commands,
) {
    let Some(mut ground) = ground else {
        return;
    };
    let dt = time.delta_secs();
    for (
        entity,
        mut pos,
        mut vert_vel,
        mut fall_tracker,
        mut health,
        death_state,
        control,
        map,
        out_of_bounds,
        motion,
        feather_fall,
        auras,
    ) in players.iter_mut()
    {
        if control.controlled {
            continue;
        }
        // A flyer has no gravity; a fall starts where its flight ends
        // (`SetCanFly(false)` → `SetFallInformation(0, GetPositionZ())`).
        if motion
            .as_ref()
            .is_some_and(|motion| motion.contains(PlayerMotion::FLYING))
        {
            let grounded = FallTracker {
                epoch: control.epoch,
                ..FallTracker::grounded(pos.y)
            };
            fall_tracker.set_if_neq(grounded);
            vert_vel.set_if_neq(VerticalVelocity(0.0));
            replicate_airborne_motion(motion, &grounded, entity, replication_dirty.as_deref());
            continue;
        }
        let previous_health = *health;
        let feet = Vec3::new(pos.x, pos.y, pos.z);
        let surface = ground
            .liquid_at(map.map_id, feet)
            .map(|liquid| liquid.level);
        let (next_pos, next_vert_vel, next_fall_tracker, mut next_health, fall) =
            calculate_gravity_state(
                entity,
                (ground.ground_in(*map, feet), surface),
                *pos,
                *vert_vel,
                fall_tracker.for_epoch(control.epoch, pos.y),
                *health,
                death_state,
                dt,
                FallAuras {
                    feather_fall: feather_fall.is_some_and(|state| state.0),
                    damage_multiplier: fall_damage_multiplier(auras),
                },
            );
        let min_height =
            ground.min_height(map.map_id, Vec3::new(next_pos.x, next_pos.y, next_pos.z));
        let under_map = min_height.is_some_and(|min_height| next_pos.y < min_height);
        let alive = death_state.is_none_or(|state| state.state == DeathState::Alive);
        let fell_to_void = under_map && alive && next_health.current > 0.0;
        // TC Player.h940-945: normal falling2 differs from the custom void6 type,
        // even though both are logged as Falling by EnvironmentalDamageLog.
        let death_kind = if fell_to_void {
            Some(6)
        } else if fall.is_some() && previous_health.current > 0.0 && next_health.current <= 0.0 {
            Some(2)
        } else {
            None
        };
        let survived_landing = fall.is_some() && next_health.current > 0.0 && !fell_to_void;
        let fall_distance =
            (fall_tracker.peak_y.max(fall_tracker.last_y).max(pos.y) - next_pos.y).max(0.0);
        let mut logs: Vec<_> = fall.into_iter().collect();
        if fell_to_void {
            logs.extend(fall_to_void(entity, &mut next_health));
            commands.entity(entity).insert(OutOfBounds);
        } else if !under_map && out_of_bounds {
            commands.entity(entity).remove::<OutOfBounds>();
        }
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
        replicate_airborne_motion(
            motion,
            &next_fall_tracker,
            entity,
            replication_dirty.as_deref(),
        );
        apply_gravity_health_transition(
            entity,
            previous_health,
            next_health,
            &mut health,
            replication_dirty.as_deref(),
            &mut player_deaths,
        );
        if survived_landing {
            commands.queue(move |world: &mut World| {
                crate::achievement::record_survived_fall(world, entity, fall_distance);
            });
        }
        if let Some(kind) = death_kind {
            commands.queue(move |world: &mut World| {
                crate::achievement::record_environmental_death(world, entity, kind);
            });
        }
        for log in logs {
            commands.queue(move |world: &mut World| crate::spell_cast::log_event(world, log));
        }
    }
}

/// Airborne motion changes replicate even without a new input or a position change.
fn replicate_airborne_motion(
    motion: Option<Mut<'_, PlayerMotion>>,
    tracker: &FallTracker,
    player: Entity,
    replication_dirty: Option<&ReplicationDirtyQueue>,
) {
    let Some(mut motion) = motion else {
        return;
    };
    let next = airborne_motion(*motion, tracker);
    if motion.set_if_neq(next)
        && let Some(dirty) = replication_dirty
    {
        dirty.mark(player);
    }
}

/// Below the map (MovementHandler.cpp:594-612): `EnvironmentalDamage(DAMAGE_FALL_TO_VOID,
/// GetMaxHealth())`, logged as Falling, then `KillPlayer` if that left the player alive.
fn fall_to_void(player: Entity, health: &mut Health) -> Option<CombatLogEvent> {
    let log = crate::death::deal_environmental_damage(
        player,
        EnvironmentalKind::Falling,
        health,
        health.max,
    );
    health.current = 0.0;
    log
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
    player: Entity,
    ground: (GroundProbe, Option<f32>),
    mut pos: Position,
    mut vert_vel: VerticalVelocity,
    mut fall_tracker: FallTracker,
    mut health: Health,
    death_state: Option<&crate::death::PlayerDeathState>,
    dt: f32,
    fall_auras: FallAuras,
) -> (
    Position,
    VerticalVelocity,
    FallTracker,
    Health,
    Option<CombatLogEvent>,
) {
    let mut fall = None;
    if death_state.is_some_and(|state| state.state != DeathState::Alive) {
        fall_tracker = FallTracker {
            epoch: fall_tracker.epoch,
            ..FallTracker::grounded(pos.y)
        };
    } else {
        fall = clamp_to_ground(
            player,
            ground,
            &mut pos,
            &mut vert_vel,
            &mut fall_tracker,
            &mut health,
            dt,
            fall_auras,
        );
    }
    (pos, vert_vel, fall_tracker, health, fall)
}

/// Clamp a position (Bevy space) to its ground: apply vertical velocity and gravity unless
/// an input reported the height this tick; a swimmer (water `SWIM_DEPTH` over the feet at
/// `surface`) has no gravity, and water ends a fall without damage. Returns the combat log
/// line of the landing's fall damage.
fn clamp_to_ground(
    player: Entity,
    (ground, surface): (GroundProbe, Option<f32>),
    pos: &mut Position,
    vert_vel: &mut VerticalVelocity,
    fall_tracker: &mut FallTracker,
    health: &mut Health,
    dt: f32,
    fall_auras: FallAuras,
) -> Option<CombatLogEvent> {
    let feather_fall = fall_auras.feather_fall;
    let ground_y = match ground {
        GroundProbe::Supported(ground_y) => ground_y,
        // Nothing within step reach: keep falling until a surface is under the feet.
        GroundProbe::Unsupported => f32::NEG_INFINITY,
        GroundProbe::Unloaded => return None,
    };
    let reported = std::mem::take(&mut fall_tracker.height_reported);
    if shared::movement::is_swimming(pos.y, ground_y, surface) {
        vert_vel.0 = 0.0;
        pos.y = pos.y.max(ground_y);
        *fall_tracker = FallTracker {
            epoch: fall_tracker.epoch,
            ..FallTracker::grounded(pos.y)
        };
        return None;
    }

    if feather_fall {
        vert_vel.0 = vert_vel.0.max(-shared::movement::FEATHER_FALL_SPEED);
        if reported {
            pos.y = pos
                .y
                .max(fall_tracker.last_y - shared::movement::FEATHER_FALL_SPEED * dt);
        } else if pos.y > ground_y + GROUND_SNAP_THRESHOLD || vert_vel.0 > 0.0 {
            vert_vel.0 = (vert_vel.0 - GRAVITY * dt).max(-shared::movement::FEATHER_FALL_SPEED);
            pos.y += vert_vel.0 * dt;
        }
    } else if !reported && vert_vel.0 != 0.0 {
        pos.y += vert_vel.0 * dt;
        vert_vel.0 -= GRAVITY * dt;
    }

    if pos.y < ground_y {
        pos.y = ground_y;
        vert_vel.0 = 0.0;
    } else if !reported && vert_vel.0 == 0.0 && pos.y > ground_y + GROUND_SNAP_THRESHOLD {
        pos.y -= GRAVITY * dt;
        if pos.y < ground_y {
            pos.y = ground_y;
        }
    }

    let fall_damage = apply_fall_damage_on_landing(fall_tracker, pos.y, ground_y, vert_vel.0, dt);
    crate::death::deal_environmental_damage(
        player,
        EnvironmentalKind::Falling,
        health,
        health.max * if feather_fall { 0.0 } else { fall_damage } * fall_auras.damage_multiplier,
    )
}

/// Auras shaping a fall: FEATHER_FALL caps the descent and negates the landing,
/// MODIFY_FALL_DAMAGE_PCT 374 multiplies its damage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FallAuras {
    pub(crate) feather_fall: bool,
    pub(crate) damage_multiplier: f32,
}

impl FallAuras {
    #[cfg(test)]
    pub(crate) const NONE: Self = Self {
        feather_fall: false,
        damage_multiplier: 1.0,
    };
}

/// `GetTotalAuraMultiplier(SPELL_AURA_MODIFY_FALL_DAMAGE_PCT)` of `Player::HandleFall`.
pub(crate) fn fall_damage_multiplier(auras: Option<&crate::auras::ActiveAuras>) -> f32 {
    auras
        .into_iter()
        .flat_map(|auras| auras.iter())
        .flat_map(|aura| {
            aura.effects
                .iter()
                .filter(|effect| effect.aura_type == crate::auras::handlers::MODIFY_FALL_DAMAGE_PCT)
                .map(|effect| 1.0 + effect.amount * aura.stack_multiplier() / 100.0)
        })
        .product()
}

/// Track the fall; on landing, the fraction of maximum health it deals.
fn apply_fall_damage_on_landing(
    fall_tracker: &mut FallTracker,
    current_y: f32,
    ground_y: f32,
    vertical_velocity: f32,
    dt: f32,
) -> f32 {
    let now_airborne = current_y > ground_y + GROUND_SNAP_THRESHOLD || vertical_velocity > 0.0;
    if !fall_tracker.airborne && now_airborne {
        fall_tracker.airborne = true;
        fall_tracker.peak_y = fall_tracker.last_y.max(current_y).max(ground_y);
        fall_tracker.takeoff_y = fall_tracker.last_y;
        fall_tracker.air_secs = dt;
    } else if fall_tracker.airborne {
        fall_tracker.peak_y = fall_tracker.peak_y.max(fall_tracker.last_y).max(current_y);
        fall_tracker.air_secs += dt;
    }

    // Landing is coming within snap reach: WMO floor heights carry float noise
    // from one probe to the next, so the feet can rest a hair above the floor.
    let landed = fall_tracker.airborne && !now_airborne;
    let mut damage = 0.0;
    if landed {
        let fall_distance = (fall_tracker.peak_y - ground_y).max(0.0);
        damage = shared::movement::fall_damage_percent(fall_distance);
        fall_tracker.airborne = false;
        fall_tracker.peak_y = ground_y;
        fall_tracker.air_secs = 0.0;
        fall_tracker.jump_impulse = 0.0;
    }

    fall_tracker.last_y = current_y;
    damage
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
    players: Query<(&Position, &InMap), With<Player>>,
    maps: Option<Res<crate::maps::MapStore>>,
    mut senders: Query<&mut MessageSender<LoadTerrain>>,
) {
    let Some(maps) = maps else { return };
    for (conn_entity, link, mut sent) in conn_q.iter_mut() {
        let Ok((pos, map)) = players.get(link.0) else {
            continue;
        };
        let Some(entry) = maps.map(map.map_id) else {
            continue;
        };
        let current_tile = position_tile(pos);
        if sent.last_tile == Some(current_tile) {
            continue;
        }
        sent.last_tile = Some(current_tile);
        send_nearby_tiles(
            conn_entity,
            &entry.directory,
            current_tile,
            &mut sent,
            &mut senders,
        );
    }
}

fn position_tile(pos: &Position) -> (u32, u32) {
    let (wow_x, wow_y, _) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
    wow_to_tile(wow_x, wow_y)
}

fn send_nearby_tiles(
    conn_entity: Entity,
    map_directory: &str,
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
                    map_name: map_directory.into(),
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
