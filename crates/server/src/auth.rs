use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    AuthChannel, ForcedDisconnect, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse,
};
use std::collections::{HashMap, HashSet};

pub use crate::auth_character::*;
use crate::persistence::{self, CharacterData, GameDatabase};
use crate::persistence_accounts;

/// Auth timeout in seconds.
const AUTH_TIMEOUT_SECS: f64 = 30.0;
const FORCED_DISCONNECT_GRACE_SECS: f64 = 0.2;

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

/// Server-authored disconnect that should be delivered to the client before the socket closes.
#[derive(Component, Debug, Clone)]
pub struct PendingForcedDisconnect {
    pub notice: ForcedDisconnect,
    pub sent_at: Option<f64>,
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
    pub last_tile: Option<(u32, u32)>,
}

pub fn queue_forced_disconnect(
    commands: &mut Commands,
    entity: Entity,
    message: impl Into<String>,
    reconnect_allowed: bool,
) {
    commands.entity(entity).insert(PendingForcedDisconnect {
        notice: ForcedDisconnect {
            message: message.into(),
            reconnect_allowed,
        },
        sent_at: None,
    });
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
            queue_forced_disconnect(&mut commands, entity, "Authentication timed out.", true);
        }
    }
}

pub fn deliver_forced_disconnects(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut PendingForcedDisconnect,
        &mut MessageSender<ForcedDisconnect>,
    )>,
    mut commands: Commands,
) {
    let now = time.elapsed_secs_f64();
    for (entity, mut pending, mut sender) in &mut query {
        if pending.sent_at.is_none() {
            sender.send::<AuthChannel>(pending.notice.clone());
            pending.sent_at = Some(now);
            continue;
        }
        if now - pending.sent_at.unwrap_or(now) < FORCED_DISCONNECT_GRACE_SECS {
            continue;
        }
        commands.entity(entity).remove::<PendingForcedDisconnect>();
        commands.trigger(lightyear::prelude::Disconnect { entity });
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
    let logins = collect_pending_auth_requests(&mut receivers);
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

fn collect_pending_auth_requests<M: lightyear::prelude::Message>(
    receivers: &mut Query<(Entity, &mut MessageReceiver<M>)>,
) -> Vec<(Entity, M)> {
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
        queue_forced_disconnect(
            commands,
            previous,
            "This account connected from another session.",
            false,
        );
    }

    let chars = persistence::list_characters_for_account(&db.db, account_id);
    let entries = load_roster_entries(&db.db, &chars);
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

fn load_roster_entries(
    db: &redb::Database,
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
            appearance: data.appearance.clone(),
            equipment_appearance: data.equipment_appearance.clone(),
            tabard: persistence::guild_tabards::load_character_design(db, *id),
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
    let reqs = collect_pending_auth_requests(&mut receivers);
    for (entity, req) in reqs {
        handle_register(entity, &req, db, &mut reg_senders);
    }
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

#[cfg(test)]
#[path = "auth_writer_tests.rs"]
mod writer_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::ClientId;
    use shared::protocol::SelectCharacter;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[derive(Component)]
    struct DeferredCleanupMarker;
    #[derive(Component)]
    struct ReplicationTag;
    #[derive(Component)]
    struct VisibilityTag;

    #[test]
    fn customization_choices_survive_creation_reopen_and_login_roster() {
        use shared::components::{CharacterAppearance, CustomizationChoiceSelection};
        let appearance = CharacterAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            eye_color: 4,
            hair_style: 5,
            hair_color: 6,
            facial_style: 7,
            customization_choices: vec![
                CustomizationChoiceSelection {
                    option_id: 87,
                    choice_id: 1734,
                },
                CustomizationChoiceSelection {
                    option_id: 214,
                    choice_id: 6109,
                },
            ],
            visage: None,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("characters.redb");
        let id = {
            let db = redb::Database::create(&path).unwrap();
            crate::persistence::create_tables(&db);
            let request = shared::protocol::CreateCharacter {
                name: "Elara".into(),
                race: 1,
                class: 8,
                appearance: appearance.clone(),
            };
            let (id, data) = crate::persistence::create_character_with_appearance(
                &db,
                17,
                &request.name,
                request.race,
                request.class,
                request.appearance,
                &crate::player_create_info::OwnedKit::bare().kit(),
            )
            .unwrap();
            assert_eq!(data.appearance, appearance);
            id
        };
        let db = redb::Database::open(&path).unwrap();
        crate::persistence::create_tables(&db);
        let data = crate::persistence::load_character(&db, id).unwrap();
        assert_eq!(data.appearance, appearance);
        crate::persistence::save_character(&db, id, &data);
        let entries = load_roster_entries(
            &db,
            &crate::persistence::list_characters_for_account(&db, 17),
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].character_id, id);
        assert_eq!(entries[0].appearance, appearance);
    }

    #[test]
    fn guild_tabard_roster_exposes_durable_design_without_changing_character_appearance() {
        use shared::guild_creation::{GuildTabardDesign, TABARD_COST};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tabard-roster.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let (character, data) =
            persistence::create_character(&db, 17, "Tabardmaster", 1, 2).unwrap();
        persistence::set_character_guild_by_id(&db, character, 7, "Raid Team").unwrap();
        persistence::banks::save_guild_member_rank(&db, character, 0);
        let mut inventory =
            persistence::inventory_store::load_inventory_record(&db, character).unwrap();
        inventory.inventory.gold = u64::from(TABARD_COST);
        persistence::inventory_store::save_inventory_record(&db, character, &inventory);
        let initial = load_roster_entries(&db, &persistence::list_characters_for_account(&db, 17));
        assert_eq!(initial[0].tabard, None);
        let design = GuildTabardDesign {
            icon: 1,
            icon_color: 2,
            border: 3,
            border_color: 4,
            background_color: 5,
        };
        persistence::guild_tabards::register(&db, character, design).unwrap();
        drop(db);
        let db = redb::Database::open(&path).unwrap();
        persistence::create_tables(&db);
        let entries = load_roster_entries(&db, &persistence::list_characters_for_account(&db, 17));
        assert_eq!(entries[0].tabard, Some(design));
        assert_eq!(entries[0].appearance, data.appearance);
        assert_eq!(entries[0].equipment_appearance, data.equipment_appearance);
    }

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
        let dir = tempfile::tempdir().unwrap();
        let db = redb::Database::create(dir.path().join("game.redb")).unwrap();
        let player = app
            .world_mut()
            .spawn((ClientId(character_id), ReplicationTag, VisibilityTag))
            .id();

        app.add_systems(
            Update,
            (
                move |existing: Query<LivePlayer>, mut commands: Commands| {
                    save_and_despawn_stale_player(&db, &existing, character_id, &mut commands);
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

    #[test]
    fn auth_timeout_is_30_seconds() {
        assert_eq!(AUTH_TIMEOUT_SECS, 30.0);
    }

    #[test]
    fn auth_timeout_triggers_after_threshold() {
        // PendingAuthTimer stores the connection time. If now - timer > 30s, timeout.
        let timer = PendingAuthTimer(10.0);

        // At 39.9s → not yet timed out
        let now_before = 39.9;
        assert!(
            now_before - timer.0 <= AUTH_TIMEOUT_SECS,
            "no timeout at 29.9s"
        );

        // At 40.1s → timed out (30.1s elapsed)
        let now_after = 40.1;
        assert!(now_after - timer.0 > AUTH_TIMEOUT_SECS, "timeout at 30.1s");
    }
}
