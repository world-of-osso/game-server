use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{
    CombatRatings, GuildMembership, Health, Level, Mana, MovementSpeed, Player, Position, Rotation,
    UnitStats,
};
use shared::protocol::{
    AuthChannel, CreateCharacter, CreateCharacterResponse, DeleteCharacter,
    DeleteCharacterResponse, EnterWorldResponse, LoadTerrain, LoginRequest, LoginResponse,
    RegisterRequest, RegisterResponse, SelectCharacter,
};
use std::collections::{HashMap, HashSet};

use crate::networking::{CombatTarget, PlayerLink, queue_player_despawn, send_load_terrain};
use crate::persistence::{self, CharacterData, ClientId, GameDatabase};
use crate::persistence_accounts;

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

/// Tracks the active connection for each authenticated account.
#[derive(Resource, Default)]
pub struct ConnectedAccounts(pub HashMap<u64, Entity>);

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
            commands.entity(entity).remove::<PendingAuth>();
            commands.trigger(lightyear::prelude::Disconnect { entity });
        }
    }
}

/// Process login requests from pending-auth connections.
pub fn process_login_requests(
    mut receivers: Query<(Entity, &mut MessageReceiver<LoginRequest>)>,
    db: Option<Res<GameDatabase>>,
    mut connected: ResMut<ConnectedAccounts>,
    mut senders: Query<&mut MessageSender<LoginResponse>>,
    mut commands: Commands,
) {
    let Some(db) = db.as_ref() else { return };
    let logins: Vec<_> = collect_login_requests(&mut receivers);
    for (entity, req) in logins {
        handle_login(
            entity,
            &req,
            db,
            &mut connected,
            &mut senders,
            &mut commands,
        );
    }
}

fn collect_login_requests(
    receivers: &mut Query<(Entity, &mut MessageReceiver<LoginRequest>)>,
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
    let result = persistence_accounts::login_with_credentials(
        &db.db,
        token_ref,
        &req.username,
        &req.password,
    );

    let (account_id, token) = match result {
        Ok(pair) => pair,
        Err(e) => {
            send_login_error(entity, senders, &e);
            return;
        }
    };

    if let Some(previous) = replace_connected_account(connected, account_id, entity) {
        info!(
            "Account {account_id} re-authenticated on {entity:?}; disconnecting older session {previous:?}"
        );
        commands.trigger(lightyear::prelude::Disconnect { entity: previous });
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

fn replace_connected_account(
    connected: &mut ConnectedAccounts,
    account_id: u64,
    entity: Entity,
) -> Option<Entity> {
    connected
        .0
        .insert(account_id, entity)
        .filter(|previous| *previous != entity)
}

pub(crate) fn clear_connected_account(
    connected: &mut ConnectedAccounts,
    account_id: u64,
    entity: Entity,
) -> bool {
    match connected.0.get(&account_id).copied() {
        Some(active) if active == entity => {
            connected.0.remove(&account_id);
            true
        }
        _ => false,
    }
}

fn chars_to_entries(chars: &[(u64, CharacterData)]) -> Vec<shared::protocol::CharacterListEntry> {
    chars
        .iter()
        .map(|(id, data)| shared::protocol::CharacterListEntry {
            character_id: *id,
            name: data.name.clone(),
            level: data.level,
            race: data.race,
            class: data.class,
            appearance: data.appearance,
            equipment_appearance: data.equipment_appearance.clone(),
        })
        .collect()
}

fn send_login_success(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<LoginResponse>>,
    token: &str,
    characters: Vec<shared::protocol::CharacterListEntry>,
) {
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
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
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    sender.send::<AuthChannel>(LoginResponse {
        success: false,
        token: String::new(),
        characters: Vec::new(),
        error: Some(error.to_string()),
    });
}

/// Process registration requests from pending-auth connections.
pub fn process_register_requests(
    mut receivers: Query<(Entity, &mut MessageReceiver<RegisterRequest>)>,
    db: Option<Res<GameDatabase>>,
    mut reg_senders: Query<&mut MessageSender<RegisterResponse>>,
) {
    let Some(db) = db.as_ref() else { return };
    let reqs: Vec<_> = collect_register_requests(&mut receivers);
    for (entity, req) in reqs {
        handle_register(entity, &req, db, &mut reg_senders);
    }
}

fn collect_register_requests(
    receivers: &mut Query<(Entity, &mut MessageReceiver<RegisterRequest>)>,
) -> Vec<(Entity, RegisterRequest)> {
    let mut out = Vec::new();
    for (entity, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            out.push((entity, req));
        }
    }
    out
}

fn handle_register(
    entity: Entity,
    req: &RegisterRequest,
    db: &GameDatabase,
    reg_senders: &mut Query<&mut MessageSender<RegisterResponse>>,
) {
    match persistence_accounts::request_registration(&db.db, &req.username, &req.password) {
        Ok(()) => {
            send_register_response(entity, reg_senders, false, "", true, None);
            info!(
                "Registration submitted for '{}' (pending approval)",
                req.username
            );
        }
        Err(e) => {
            send_register_response(entity, reg_senders, false, "", false, Some(&e));
        }
    }
}

fn send_register_response(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<RegisterResponse>>,
    success: bool,
    token: &str,
    pending_approval: bool,
    error: Option<&str>,
) {
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    sender.send::<AuthChannel>(RegisterResponse {
        success,
        token: token.to_string(),
        pending_approval,
        error: error.map(|s| s.to_string()),
    });
}

/// Process character creation requests from authenticated connections.
pub fn process_create_character(
    mut receivers: Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<CreateCharacter>,
    )>,
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
    receivers: &mut Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<CreateCharacter>,
    )>,
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
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    match persistence::create_character_with_appearance(
        &db.db,
        account_id,
        &req.name,
        req.race,
        req.class,
        req.appearance,
    ) {
        Ok((cid, data)) => {
            let entry = shared::protocol::CharacterListEntry {
                character_id: cid,
                name: data.name,
                level: data.level,
                race: data.race,
                class: data.class,
                appearance: data.appearance,
                equipment_appearance: data.equipment_appearance,
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
    mut receivers: Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<DeleteCharacter>,
    )>,
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
    receivers: &mut Query<(
        Entity,
        &Authenticated,
        &mut MessageReceiver<DeleteCharacter>,
    )>,
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
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
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
        (
            Entity,
            &Authenticated,
            &mut MessageReceiver<SelectCharacter>,
        ),
        Without<InWorld>,
    >,
    db: Option<Res<GameDatabase>>,
    mut enter_senders: Query<&mut MessageSender<EnterWorldResponse>>,
    mut terrain_senders: Query<&mut MessageSender<LoadTerrain>>,
    existing_players: Query<(Entity, &ClientId)>,
    mut commands: Commands,
) {
    let Some(db) = db.as_ref() else { return };
    let requests: Vec<_> = collect_select_requests(&mut receivers);
    for (entity, account_id, req) in requests {
        despawn_stale_player(&existing_players, req.character_id, &mut commands);
        handle_select(
            entity,
            account_id,
            &req,
            db,
            &mut enter_senders,
            &mut terrain_senders,
            &mut commands,
        );
    }
}

fn collect_select_requests(
    receivers: &mut Query<
        (
            Entity,
            &Authenticated,
            &mut MessageReceiver<SelectCharacter>,
        ),
        Without<InWorld>,
    >,
) -> Vec<(Entity, u64, SelectCharacter)> {
    let mut out = Vec::new();
    for (entity, auth, mut receiver) in receivers.iter_mut() {
        for req in receiver.receive() {
            if !push_unique_select_request(&mut out, entity, auth.account_id, req) {
                warn!(
                    "Ignoring duplicate SelectCharacter for connection {:?} in same tick",
                    entity
                );
            }
        }
    }
    out
}

fn push_unique_select_request(
    out: &mut Vec<(Entity, u64, SelectCharacter)>,
    entity: Entity,
    account_id: u64,
    req: SelectCharacter,
) -> bool {
    if out.iter().any(|(existing, _, _)| *existing == entity) {
        return false;
    }
    out.push((entity, account_id, req));
    true
}

fn despawn_stale_player(
    existing: &Query<(Entity, &ClientId)>,
    character_id: u64,
    commands: &mut Commands,
) {
    for (entity, cid) in existing.iter() {
        if cid.0 == character_id {
            warn!("Despawning stale player entity {entity:?} for character {character_id}");
            queue_player_despawn(commands, entity);
        }
    }
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
    let guild = persistence::load_character_guild(&db.db, req.character_id);
    let player_entity = spawn_player_entity(commands, &data, req.character_id, guild.as_ref());
    insert_world_components(commands, conn, player_entity, req.character_id);
    send_enter_success(conn, enter_senders, player_entity);
    send_load_terrain(conn, terrain_senders);
    info!(
        "Character {} entered world (account {})",
        req.character_id, account_id
    );
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
        SentTiles {
            tiles: initial_tiles,
        },
        InWorld { character_id },
    ));
}

fn send_enter_success(
    entity: Entity,
    senders: &mut Query<&mut MessageSender<EnterWorldResponse>>,
    player_entity: Entity,
) {
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
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
    let Ok(mut sender) = senders.get_mut(entity) else {
        return;
    };
    sender.send::<AuthChannel>(EnterWorldResponse {
        success: false,
        player_entity: None,
        error: Some(error.to_string()),
    });
}

fn build_player_components(data: &CharacterData, character_id: u64) -> impl Bundle {
    // CharacterData stores WoW coords (from DB); convert to Bevy space for ECS.
    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(data.x, data.y, data.z);
    (
        Position {
            x: bx,
            y: by,
            z: bz,
        },
        Health {
            current: data.health,
            max: data.max_health,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        MovementSpeed(DEFAULT_RUN_SPEED),
        Player {
            name: data.name.clone(),
            race: data.race,
            class: data.class,
            appearance: data.appearance,
        },
        UnitStats::default(),
        CombatRatings::default(),
        Level(data.level as u8),
        data.equipment_appearance.clone(),
        ClientId(character_id),
        CombatTarget::default(),
        Replicate::new(ReplicationMode::SingleServer(NetworkTarget::All)),
        NetworkVisibility,
    )
}

/// Spawn a player entity with replicated components.
pub fn spawn_player_entity(
    commands: &mut Commands,
    data: &CharacterData,
    character_id: u64,
    guild: Option<&GuildMembership>,
) -> Entity {
    let entity = commands
        .spawn(build_player_components(data, character_id))
        .id();
    if shared::formulas::base_mana(data.class, data.level as u8).is_some() {
        commands.entity(entity).insert(Mana {
            current: 0.0,
            max: 0.0,
        });
    }
    if let Some(g) = guild {
        commands.entity(entity).insert(g.clone());
    }
    info!(
        "Spawned player '{}' at ({}, {}, {})",
        data.name, data.x, data.y, data.z
    );
    entity
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[derive(Component)]
    struct DeferredCleanupMarker;
    #[derive(Component)]
    struct ReplicationTag;
    #[derive(Component)]
    struct VisibilityTag;

    #[test]
    fn relogin_returns_previous_session_entity() {
        let account_id = 42;
        let first = Entity::from_bits(1);
        let second = Entity::from_bits(2);
        let mut connected = ConnectedAccounts::default();

        assert_eq!(
            replace_connected_account(&mut connected, account_id, first),
            None
        );
        assert_eq!(
            replace_connected_account(&mut connected, account_id, second),
            Some(first)
        );
        assert_eq!(connected.0.get(&account_id), Some(&second));
    }

    #[test]
    fn disconnect_cleanup_ignores_replaced_session() {
        let account_id = 42;
        let old = Entity::from_bits(1);
        let current = Entity::from_bits(2);
        let mut connected = ConnectedAccounts::default();

        connected.0.insert(account_id, current);

        assert!(!clear_connected_account(&mut connected, account_id, old));
        assert_eq!(connected.0.get(&account_id), Some(&current));

        assert!(clear_connected_account(&mut connected, account_id, current));
        assert!(!connected.0.contains_key(&account_id));
    }

    #[test]
    fn push_unique_select_request_keeps_first_request_per_connection() {
        let entity = Entity::from_bits(1);
        let account_id = 42;
        let mut out = Vec::new();

        assert!(push_unique_select_request(
            &mut out,
            entity,
            account_id,
            SelectCharacter { character_id: 10 },
        ));
        assert!(!push_unique_select_request(
            &mut out,
            entity,
            account_id,
            SelectCharacter { character_id: 11 },
        ));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].2.character_id, 10);
    }

    #[test]
    fn push_unique_select_request_allows_different_connections() {
        let mut out = Vec::new();

        assert!(push_unique_select_request(
            &mut out,
            Entity::from_bits(1),
            42,
            SelectCharacter { character_id: 10 },
        ));
        assert!(push_unique_select_request(
            &mut out,
            Entity::from_bits(2),
            43,
            SelectCharacter { character_id: 11 },
        ));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn stale_player_cleanup_does_not_panic_when_other_system_queues_deferred_cleanup() {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);

        let character_id = 77;
        let player = app
            .world_mut()
            .spawn((ClientId(character_id), ReplicationTag, VisibilityTag))
            .id();

        app.add_systems(
            Update,
            (
                move |existing: Query<(Entity, &ClientId)>, mut commands: Commands| {
                    despawn_stale_player(&existing, character_id, &mut commands);
                },
                move |mut commands: Commands| {
                    commands
                        .entity(player)
                        .remove::<(ReplicationTag, VisibilityTag)>()
                        .insert(DeferredCleanupMarker);
                },
            )
                .chain(),
        );

        let result = catch_unwind(AssertUnwindSafe(|| app.update()));
        assert!(
            result.is_ok(),
            "stale player cleanup should not queue an immediate despawn ahead of deferred cleanup"
        );
    }
}
