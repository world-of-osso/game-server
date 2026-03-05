use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, MovementSpeed, Player, Position, Rotation};
use shared::protocol::{
    AuthChannel, CreateCharacter, CreateCharacterResponse, DeleteCharacter,
    DeleteCharacterResponse, EnterWorldResponse, LoadTerrain, LoginRequest, LoginResponse,
    SelectCharacter,
};
use std::collections::HashSet;

use crate::networking::{CombatTarget, PlayerLink, send_load_terrain};
use crate::persistence::{self, CharacterData, ClientId, GameDatabase};

/// Default run speed in yards per second (WoW run speed).
const DEFAULT_RUN_SPEED: f32 = 7.0;

/// Auth timeout in seconds.
const AUTH_TIMEOUT_SECS: f64 = 30.0;

/// Marker for unauthenticated connections.
#[derive(Component, Debug)]
pub struct PendingAuth;

/// Timer for auth timeout (elapsed seconds when connection was established).
#[derive(Component, Debug)]
pub struct PendingAuthTimer(pub f64);

/// Marker for authenticated connections.
#[derive(Component, Debug)]
pub struct Authenticated {
    pub account_id: u64,
}

/// Marker for connections that have entered the world.
#[derive(Component, Debug)]
pub struct InWorld {
    pub character_id: u64,
}

/// Prevents duplicate logins for the same account.
#[derive(Resource, Default)]
pub struct ConnectedAccounts(pub HashSet<u64>);

/// Tracks which terrain tiles have been sent to a client.
#[derive(Component, Default)]
pub struct SentTiles {
    pub tiles: HashSet<(u32, u32)>,
}

/// Disconnect clients that haven't authenticated within the timeout.
pub fn auth_timeout(
    pending: Query<(Entity, &PendingAuthTimer), With<PendingAuth>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let now = time.elapsed_secs_f64();
    for (entity, timer) in pending.iter() {
        if now - timer.0 > AUTH_TIMEOUT_SECS {
            warn!("Auth timeout for {:?}, disconnecting", entity);
            commands.entity(entity).despawn();
        }
    }
}

/// Process login requests from pending-auth connections.
pub fn process_login_requests(
    mut receivers: Query<(Entity, &mut MessageReceiver<LoginRequest>), With<PendingAuth>>,
    db: Option<Res<GameDatabase>>,
    mut connected: ResMut<ConnectedAccounts>,
    mut senders: Query<&mut MessageSender<LoginResponse>>,
    mut commands: Commands,
) {
    let Some(db) = db.as_ref() else { return };
    let logins: Vec<_> = collect_login_requests(&mut receivers);
    for (entity, req) in logins {
        handle_login(entity, &req, db, &mut connected, &mut senders, &mut commands);
    }
}

fn collect_login_requests(
    receivers: &mut Query<(Entity, &mut MessageReceiver<LoginRequest>), With<PendingAuth>>,
) -> Vec<(Entity, LoginRequest)> {
    let mut out = Vec::new();
    for (entity, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            out.push((entity, req));
        }
    }
    out
}

fn handle_login(
    entity: Entity,
    req: &LoginRequest,
    db: &GameDatabase,
    connected: &mut ConnectedAccounts,
    senders: &mut Query<&mut MessageSender<LoginResponse>>,
    commands: &mut Commands,
) {
    let token_ref = req.token.as_deref();
    let (account_id, token) =
        persistence::lookup_or_create_account(&db.db, token_ref, &req.username);

    if !connected.0.insert(account_id) {
        send_login_error(entity, senders, "Account already logged in");
        return;
    }

    let chars = persistence::list_characters_for_account(&db.db, account_id);
    let entries = chars_to_entries(&chars);
    send_login_success(entity, senders, &token, entries);
    commands
        .entity(entity)
        .remove::<(PendingAuth, PendingAuthTimer)>()
        .insert(Authenticated { account_id });
    info!("Account {account_id} authenticated");
}

fn chars_to_entries(
    chars: &[(u64, CharacterData)],
) -> Vec<shared::protocol::CharacterListEntry> {
    chars
        .iter()
        .map(|(id, data)| shared::protocol::CharacterListEntry {
            character_id: *id,
            name: data.name.clone(),
            level: data.level,
            race: data.race,
            class: data.class,
        })
        .collect()
}

fn send_login_success(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<LoginResponse>>,
    token: &str,
    characters: Vec<shared::protocol::CharacterListEntry>,
) {
    let Ok(mut sender) = senders.get_mut(entity) else { return };
    sender.send::<AuthChannel>(LoginResponse {
        success: true,
        token: token.to_string(),
        characters,
        error: None,
    });
}

fn send_login_error(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<LoginResponse>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(entity) else { return };
    sender.send::<AuthChannel>(LoginResponse {
        success: false,
        token: String::new(),
        characters: Vec::new(),
        error: Some(error.to_string()),
    });
}

/// Process character creation requests from authenticated connections.
pub fn process_create_character(
    mut receivers: Query<(Entity, &Authenticated, &mut MessageReceiver<CreateCharacter>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<CreateCharacterResponse>>,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_create_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        handle_create(entity, account_id, &req, db, &mut senders);
    }
}

fn collect_create_requests(
    receivers: &mut Query<(Entity, &Authenticated, &mut MessageReceiver<CreateCharacter>)>,
) -> Vec<(Entity, u64, CreateCharacter)> {
    let mut out = Vec::new();
    for (entity, auth, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            out.push((entity, auth.account_id, req));
        }
    }
    out
}

fn handle_create(
    entity: Entity,
    account_id: u64,
    req: &CreateCharacter,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<CreateCharacterResponse>>,
) {
    let Ok(mut sender) = senders.get_mut(entity) else { return };
    match persistence::create_character(&db.db, account_id, &req.name, req.race, req.class) {
        Ok((cid, data)) => {
            let entry = shared::protocol::CharacterListEntry {
                character_id: cid,
                name: data.name,
                level: data.level,
                race: data.race,
                class: data.class,
            };
            sender.send::<AuthChannel>(CreateCharacterResponse {
                success: true,
                character: Some(entry),
                error: None,
            });
        }
        Err(e) => {
            sender.send::<AuthChannel>(CreateCharacterResponse {
                success: false,
                character: None,
                error: Some(e),
            });
        }
    }
}

/// Process character deletion requests from authenticated connections.
pub fn process_delete_character(
    mut receivers: Query<(Entity, &Authenticated, &mut MessageReceiver<DeleteCharacter>)>,
    db: Option<Res<GameDatabase>>,
    mut senders: Query<&mut MessageSender<DeleteCharacterResponse>>,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_delete_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        handle_delete(entity, account_id, &req, db, &mut senders);
    }
}

fn collect_delete_requests(
    receivers: &mut Query<(Entity, &Authenticated, &mut MessageReceiver<DeleteCharacter>)>,
) -> Vec<(Entity, u64, DeleteCharacter)> {
    let mut out = Vec::new();
    for (entity, auth, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            out.push((entity, auth.account_id, req));
        }
    }
    out
}

fn handle_delete(
    entity: Entity,
    account_id: u64,
    req: &DeleteCharacter,
    db: &GameDatabase,
    senders: &mut Query<&mut MessageSender<DeleteCharacterResponse>>,
) {
    let Ok(mut sender) = senders.get_mut(entity) else { return };
    match persistence::delete_character(&db.db, account_id, req.character_id) {
        Ok(()) => {
            sender.send::<AuthChannel>(DeleteCharacterResponse {
                success: true,
                character_id: req.character_id,
                error: None,
            });
        }
        Err(e) => {
            sender.send::<AuthChannel>(DeleteCharacterResponse {
                success: false,
                character_id: req.character_id,
                error: Some(e),
            });
        }
    }
}

/// Process character selection and world entry.
pub fn process_select_character(
    mut receivers: Query<
        (Entity, &Authenticated, &mut MessageReceiver<SelectCharacter>),
        Without<InWorld>,
    >,
    db: Option<Res<GameDatabase>>,
    mut enter_senders: Query<&mut MessageSender<EnterWorldResponse>>,
    mut terrain_senders: Query<&mut MessageSender<LoadTerrain>>,
    mut commands: Commands,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_select_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        handle_select(
            entity, account_id, &req, db,
            &mut enter_senders, &mut terrain_senders, &mut commands,
        );
    }
}

fn collect_select_requests(
    receivers: &mut Query<
        (Entity, &Authenticated, &mut MessageReceiver<SelectCharacter>),
        Without<InWorld>,
    >,
) -> Vec<(Entity, u64, SelectCharacter)> {
    let mut out = Vec::new();
    for (entity, auth, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            out.push((entity, auth.account_id, req));
        }
    }
    out
}

fn handle_select(
    conn: Entity,
    account_id: u64,
    req: &SelectCharacter,
    db: &GameDatabase,
    enter_senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    terrain_senders: &mut Query<&mut MessageSender<LoadTerrain>>,
    commands: &mut Commands,
) {
    if !validate_ownership(db, account_id, req.character_id) {
        send_enter_error(conn, enter_senders, "Character not found");
        return;
    }
    let Some(data) = persistence::load_character(&db.db, req.character_id) else {
        send_enter_error(conn, enter_senders, "Character data missing");
        return;
    };
    let player_entity = spawn_player_entity(commands, &data, req.character_id);
    insert_world_components(commands, conn, player_entity, req.character_id);
    send_enter_success(conn, enter_senders, player_entity);
    send_load_terrain(conn, terrain_senders);
    info!("Character {} entered world (account {})", req.character_id, account_id);
}

fn validate_ownership(db: &GameDatabase, account_id: u64, character_id: u64) -> bool {
    let chars = persistence::list_characters_for_account(&db.db, account_id);
    chars.iter().any(|(id, _)| *id == character_id)
}

fn insert_world_components(
    commands: &mut Commands,
    conn: Entity,
    player_entity: Entity,
    character_id: u64,
) {
    let mut initial_tiles = HashSet::new();
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            let ty = (32i32 + dy).clamp(0, 63) as u32;
            let tx = (48i32 + dx).clamp(0, 63) as u32;
            initial_tiles.insert((ty, tx));
        }
    }
    commands.entity(conn).insert((
        PlayerLink(player_entity),
        SentTiles { tiles: initial_tiles },
        InWorld { character_id },
    ));
}

fn send_enter_success(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    player_entity: Entity,
) {
    let Ok(mut sender) = senders.get_mut(entity) else { return };
    sender.send::<AuthChannel>(EnterWorldResponse {
        success: true,
        player_entity: Some(player_entity.to_bits()),
        error: None,
    });
}

fn send_enter_error(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    error: &str,
) {
    let Ok(mut sender) = senders.get_mut(entity) else { return };
    sender.send::<AuthChannel>(EnterWorldResponse {
        success: false,
        player_entity: None,
        error: Some(error.to_string()),
    });
}

/// Spawn a player entity with replicated components.
pub fn spawn_player_entity(
    commands: &mut Commands,
    data: &CharacterData,
    character_id: u64,
) -> Entity {
    let pos = Position { x: data.x, y: data.y, z: data.z };
    let health = Health { current: data.health, max: data.max_health };
    let rotation = Rotation { x: 0.0, y: 0.0, z: 0.0 };
    let entity = commands
        .spawn((
            pos, health, rotation,
            MovementSpeed(DEFAULT_RUN_SPEED),
            Player { name: data.name.clone() },
            ClientId(character_id),
            CombatTarget::default(),
            Replicate::default(),
            NetworkVisibility,
        ))
        .id();
    info!("Spawned player '{}' at ({}, {}, {})", data.name, pos.x, pos.y, pos.z);
    entity
}
