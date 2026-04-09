use std::path::PathBuf;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use peercred_ipc::{Connection, Server};
use shared::components::{EquipmentAppearance, Position};
use shared::protocol::{CharacterListUpdate, LoginResponse};

use crate::admin_handlers::*;
use crate::admin_types::*;
use crate::persistence;
use crate::persistence_accounts;

const SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

/// Internal command passed from IPC server thread to Bevy main loop.
struct AdminCommand {
    request: AdminRequest,
    respond: mpsc::Sender<AdminResponse>,
}

/// RAII guard that removes the socket file on drop.
struct SocketGuard {
    path: PathBuf,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub struct AdminIpcPlugin;

impl Plugin for AdminIpcPlugin {
    fn build(&self, app: &mut App) {
        let (receiver, guard) = init();

        app.insert_non_send_resource(receiver)
            .insert_non_send_resource(guard)
            .add_systems(Update, poll_admin_ipc);
    }
}

/// Spawn the IPC server on a tokio runtime in a background thread.
fn init() -> (mpsc::Receiver<AdminCommand>, SocketGuard) {
    let path = PathBuf::from(SOCKET_PATH);

    // Remove stale socket from a previous run.
    let _ = std::fs::remove_file(&path);

    let (tx, rx) = mpsc::channel();
    let sock = path.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(serve(sock, tx));
    });

    (rx, SocketGuard { path })
}

async fn serve(path: PathBuf, tx: mpsc::Sender<AdminCommand>) {
    let server = match Server::bind(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("admin ipc: bind failed: {e}");
            return;
        }
    };

    loop {
        let (conn, _caller) = match server.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("admin ipc: accept error: {e}");
                continue;
            }
        };
        let tx = tx.clone();
        tokio::spawn(handle_connection(conn, tx));
    }
}

async fn handle_connection(mut conn: Connection, tx: mpsc::Sender<AdminCommand>) {
    let request: AdminRequest = match conn.read().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("admin ipc: read error: {e}");
            return;
        }
    };

    let (resp_tx, resp_rx) = mpsc::channel();
    let cmd = AdminCommand {
        request,
        respond: resp_tx,
    };

    if tx.send(cmd).is_err() {
        return;
    }

    let response = match resp_rx.recv() {
        Ok(r) => r,
        Err(_) => AdminResponse::Error("internal: channel closed".into()),
    };

    if let Err(e) = conn.write(&response).await {
        eprintln!("admin ipc: write error: {e}");
    }
}

/// Resources needed by the admin IPC early-dispatch handlers.
#[derive(SystemParam)]
struct AdminResources<'w> {
    time: Res<'w, Time>,
    combat_log: Res<'w, crate::combat::CombatLog>,
    ticket_mgr: ResMut<'w, shared::ticket::TicketManager>,
    motd: ResMut<'w, shared::motd::Motd>,
    autobroadcast: ResMut<'w, shared::motd::Autobroadcast>,
}

/// Queries needed by the admin IPC dispatch loop.
#[derive(SystemParam)]
struct AdminQueries<'w, 's> {
    commands: Commands<'w, 's>,
    players: Query<'w, 's, (&'static mut Position, &'static persistence::ClientId)>,
    equipment: Query<
        'w,
        's,
        (
            Entity,
            &'static mut EquipmentAppearance,
            &'static persistence::ClientId,
        ),
    >,
    authenticated: Query<
        'w,
        's,
        (Entity, &'static crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_senders: Query<'w, 's, &'static mut MessageSender<CharacterListUpdate>>,
}

/// Poll IPC commands each frame and dispatch them.
fn poll_admin_ipc(
    receiver: NonSend<mpsc::Receiver<AdminCommand>>,
    db: Option<Res<persistence::GameDatabase>>,
    mut res: AdminResources,
    mut queries: AdminQueries,
) {
    let Some(db) = db else { return };
    while let Ok(cmd) = receiver.try_recv() {
        if let Some(response) = dispatch_resource_commands(&cmd.request, &db, &mut res) {
            let _ = cmd.respond.send(response);
            continue;
        }
        dispatch(
            cmd,
            &db,
            &mut queries.commands,
            &mut queries.players,
            &mut queries.equipment,
            &queries.authenticated,
            &mut queries.roster_senders,
        );
    }
}

/// Handle commands that only need resources (no ECS queries).
fn dispatch_resource_commands(
    request: &AdminRequest,
    db: &persistence::GameDatabase,
    res: &mut AdminResources,
) -> Option<AdminResponse> {
    if let AdminRequest::CombatLog { last_seconds } = request {
        return Some(handle_combat_log(&res.combat_log, &res.time, *last_seconds));
    }
    if let Some(r) = dispatch_ticket(request, &res.time, &mut res.ticket_mgr) {
        return Some(r);
    }
    if let Some(r) = dispatch_motd(request, &res.time, &mut res.motd, db) {
        return Some(r);
    }
    if let Some(r) = dispatch_autobroadcast(request, &mut res.autobroadcast) {
        return Some(r);
    }
    None
}

/// Dispatch ticket commands, returning Some if handled.
fn dispatch_ticket(
    request: &AdminRequest,
    time: &Time,
    ticket_mgr: &mut shared::ticket::TicketManager,
) -> Option<AdminResponse> {
    match request {
        AdminRequest::TicketList => Some(handle_ticket_list(ticket_mgr)),
        AdminRequest::TicketClaim { ticket_id, gm_name } => {
            Some(handle_ticket_claim(ticket_mgr, *ticket_id, gm_name))
        }
        AdminRequest::TicketClaimNext { gm_name } => {
            Some(handle_ticket_claim_next(ticket_mgr, gm_name))
        }
        AdminRequest::TicketRespond {
            ticket_id,
            gm_name,
            message,
        } => {
            let now = time.elapsed_secs_f64() as u64;
            Some(handle_ticket_respond(
                ticket_mgr, *ticket_id, gm_name, message, now,
            ))
        }
        AdminRequest::TicketResolve { ticket_id } => {
            Some(handle_ticket_resolve(ticket_mgr, *ticket_id))
        }
        AdminRequest::TicketEscalate { ticket_id } => {
            Some(handle_ticket_escalate(ticket_mgr, *ticket_id))
        }
        AdminRequest::TicketHistory { player_id } => {
            Some(handle_ticket_history(ticket_mgr, *player_id))
        }
        _ => None,
    }
}

/// Dispatch MOTD commands, returning Some if handled.
fn dispatch_motd(
    request: &AdminRequest,
    time: &Time,
    motd: &mut shared::motd::Motd,
    db: &persistence::GameDatabase,
) -> Option<AdminResponse> {
    match request {
        AdminRequest::GetMotd => Some(AdminResponse::MotdRetrieved {
            message: motd.message.clone(),
            updated_at: motd.updated_at,
        }),
        AdminRequest::SetMotd { message } => {
            let now = time.elapsed_secs_f64() as u64;
            match motd.set(message, now) {
                Ok(()) => {
                    persistence::save_motd(&db.db, motd);
                    Some(AdminResponse::MotdSet)
                }
                Err(e) => Some(AdminResponse::Error(format!("{e:?}"))),
            }
        }
        AdminRequest::ClearMotd => {
            let now = time.elapsed_secs_f64() as u64;
            motd.clear(now);
            persistence::save_motd(&db.db, motd);
            Some(AdminResponse::MotdCleared)
        }
        _ => None,
    }
}

/// Dispatch autobroadcast commands, returning Some if handled.
fn dispatch_autobroadcast(
    request: &AdminRequest,
    ab: &mut shared::motd::Autobroadcast,
) -> Option<AdminResponse> {
    match request {
        AdminRequest::BroadcastList => Some(AdminResponse::BroadcastListed {
            messages: ab.messages.clone(),
            interval: ab.interval,
            enabled: ab.enabled,
        }),
        AdminRequest::BroadcastAdd { message } => {
            ab.add_message(message.clone());
            Some(AdminResponse::BroadcastAdded {
                index: ab.message_count() - 1,
            })
        }
        AdminRequest::BroadcastRemove { index } => match ab.remove_message(*index) {
            Some(msg) => Some(AdminResponse::BroadcastRemoved { message: msg }),
            None => Some(AdminResponse::Error("Index out of range".into())),
        },
        AdminRequest::BroadcastSetInterval { seconds } => {
            ab.interval = *seconds;
            Some(AdminResponse::BroadcastIntervalSet { seconds: *seconds })
        }
        AdminRequest::BroadcastEnable => {
            ab.enabled = true;
            Some(AdminResponse::BroadcastEnabled)
        }
        AdminRequest::BroadcastDisable => {
            ab.enabled = false;
            Some(AdminResponse::BroadcastDisabled)
        }
        _ => None,
    }
}

fn queue_account_disconnect(
    commands: &mut Commands,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    account_id: u64,
    message: &str,
) -> bool {
    let Some(entity) = authenticated_query
        .iter()
        .find_map(|(entity, auth)| (auth.account_id == account_id).then_some(entity))
    else {
        return false;
    };
    crate::auth::queue_forced_disconnect(commands, entity, message, false);
    true
}

fn kick_message(reason: Option<String>) -> String {
    match reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
    {
        Some(reason) => format!("You were kicked: {reason}"),
        None => "You were kicked by an administrator.".to_string(),
    }
}

fn ban_reason(reason: Option<String>) -> String {
    match reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
    {
        Some(reason) => reason.to_string(),
        None => "Banned by an administrator.".to_string(),
    }
}

fn dispatch(
    cmd: AdminCommand,
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    players: &mut Query<(&mut Position, &persistence::ClientId)>,
    equipment_query: &mut Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
    authenticated_query: &Query<
        (Entity, &crate::auth::Authenticated),
        With<MessageSender<LoginResponse>>,
    >,
    roster_update_senders: &mut Query<&mut MessageSender<CharacterListUpdate>>,
) {
    let response = match cmd.request {
        AdminRequest::Ping => AdminResponse::Pong,
        AdminRequest::CombatLog { .. } => {
            // Handled in poll_admin_ipc before dispatch is called
            unreachable!()
        }
        AdminRequest::ListAccounts => handle_list_accounts(db),
        AdminRequest::ListCharacters { username } => {
            handle_list_characters(db, username.as_deref())
        }
        AdminRequest::FindCharacter { name } => handle_find_character(db, &name),
        AdminRequest::MoveCharacters {
            from_username,
            to_username,
        } => handle_move_characters(db, &from_username, &to_username),
        AdminRequest::CheckAccount { username } => handle_check_account(db, &username),
        AdminRequest::CheckPassword { username, password } => {
            handle_check_password(db, &username, &password)
        }
        AdminRequest::CreateAccount { username, password } => {
            handle_create_account(db, &username, &password)
        }
        AdminRequest::KickAccount { username, reason } => {
            match persistence_accounts::find_account_by_username(&db.db, &username) {
                Some(account_id) => {
                    let message = kick_message(reason);
                    let disconnected = queue_account_disconnect(
                        commands,
                        authenticated_query,
                        account_id,
                        &message,
                    );
                    AdminResponse::AccountKicked {
                        username,
                        disconnected,
                    }
                }
                None => AdminResponse::Error(format!("account '{username}' not found")),
            }
        }
        AdminRequest::BanAccount { username, reason } => {
            let reason = ban_reason(reason);
            match persistence_accounts::ban_account(&db.db, &username, &reason) {
                Ok(account_id) => {
                    let disconnected = queue_account_disconnect(
                        commands,
                        authenticated_query,
                        account_id,
                        &format!("Account banned: {reason}"),
                    );
                    AdminResponse::AccountBanned {
                        username,
                        disconnected,
                        reason,
                    }
                }
                Err(error) => AdminResponse::Error(error),
            }
        }
        AdminRequest::UnbanAccount { username } => {
            match persistence_accounts::unban_account(&db.db, &username) {
                Ok(_) => AdminResponse::AccountUnbanned { username },
                Err(error) => AdminResponse::Error(error),
            }
        }
        AdminRequest::ApproveRegistration { username } => {
            handle_approve_registration(db, &username)
        }
        AdminRequest::CreateGuild {
            character_name,
            guild_id,
            guild_name,
        } => handle_create_guild(db, &character_name, guild_id, &guild_name),
        AdminRequest::SetLevel {
            character_name,
            level,
        } => handle_set_level(db, &character_name, level),
        AdminRequest::SetCredentials {
            token,
            username,
            password,
        } => handle_set_credentials(db, &token, &username, &password),
        AdminRequest::SetPosition {
            character_name,
            x,
            y,
            z,
        } => handle_set_position(db, players, &character_name, x, y, z),
        AdminRequest::SetAppearance {
            character_name,
            appearance,
        } => handle_set_appearance(
            db,
            authenticated_query,
            roster_update_senders,
            &character_name,
            appearance,
        ),
        AdminRequest::SetEquipmentAppearance {
            character_name,
            slot,
            display_info_id,
        } => handle_set_equipment_appearance(
            db,
            commands,
            equipment_query,
            authenticated_query,
            roster_update_senders,
            EquipmentAppearanceRequest {
                name: &character_name,
                slot_name: &slot,
                display_info_id,
            },
        ),
        AdminRequest::ClearEquipmentAppearance {
            character_name,
            slot,
        } => handle_clear_equipment_appearance(
            db,
            commands,
            equipment_query,
            authenticated_query,
            roster_update_senders,
            &character_name,
            &slot,
        ),
        // Handled in early-dispatch functions before this match
        AdminRequest::TicketList
        | AdminRequest::TicketClaim { .. }
        | AdminRequest::TicketClaimNext { .. }
        | AdminRequest::TicketRespond { .. }
        | AdminRequest::TicketResolve { .. }
        | AdminRequest::TicketEscalate { .. }
        | AdminRequest::TicketHistory { .. }
        | AdminRequest::GetMotd
        | AdminRequest::SetMotd { .. }
        | AdminRequest::ClearMotd
        | AdminRequest::BroadcastList
        | AdminRequest::BroadcastAdd { .. }
        | AdminRequest::BroadcastRemove { .. }
        | AdminRequest::BroadcastSetInterval { .. }
        | AdminRequest::BroadcastEnable
        | AdminRequest::BroadcastDisable => unreachable!(),
    };
    let _ = cmd.respond.send(response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    use crate::persistence_accounts;

    #[test]
    fn handle_set_position_updates_persisted_character_coords() {
        let db = persistence::temp_database();
        let (character_id, _) =
            persistence::create_character(&db, 1, "Mover", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let mut app = App::new();
        let live_entity = app
            .world_mut()
            .spawn((
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                persistence::ClientId(character_id),
            ))
            .id();
        let mut system_state =
            SystemState::<Query<(&mut Position, &persistence::ClientId)>>::new(app.world_mut());
        let mut query = system_state.get_mut(app.world_mut());

        let response = handle_set_position(&game_db, &mut query, "Mover", -8920.5, -188.25, 97.75);

        assert_eq!(
            response,
            AdminResponse::PositionSet {
                character_name: "Mover".to_string(),
                x: -8920.5,
                y: -188.25,
                z: 97.75,
            }
        );
        system_state.apply(app.world_mut());
        let saved = persistence::load_character(&game_db.db, character_id).expect("saved");
        assert_eq!((saved.x, saved.y, saved.z), (-8920.5, -188.25, 97.75));
        let pos = app.world().entity(live_entity).get::<Position>().unwrap();
        let (bx, by, bz) = crate::terrain_height::wow_to_bevy(-8920.5, -188.25, 97.75);
        assert_eq!((pos.x, pos.y, pos.z), (bx, by, bz));
    }

    #[test]
    fn handle_check_account_reports_existing_user() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("account");
        let game_db = persistence::GameDatabase { db };

        let response = handle_check_account(&game_db, "admin");

        assert!(matches!(
            response,
            AdminResponse::AccountChecked {
                username,
                exists: true,
                account_id: Some(id),
                token_count: 1,
                pending_registration: false,
                banned: false,
                ban_reason: None,
            } if username == "admin" && id == account_id
        ));
    }

    #[test]
    fn handle_list_accounts_returns_all_accounts() {
        let db = persistence::temp_database();
        persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence_accounts::register_account(&db, "alice", "secret").expect("alice");
        let game_db = persistence::GameDatabase { db };

        let response = handle_list_accounts(&game_db);

        assert!(matches!(
            response,
            AdminResponse::AccountsListed { accounts }
                if accounts.len() == 2
                    && accounts.iter().any(|account| account.username == "admin")
                    && accounts.iter().any(|account| account.username == "alice")
        ));
    }

    #[test]
    fn handle_check_password_reports_validity() {
        let db = persistence::temp_database();
        persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        let game_db = persistence::GameDatabase { db };

        assert_eq!(
            handle_check_password(&game_db, "admin", "admin"),
            AdminResponse::PasswordChecked {
                username: "admin".to_string(),
                valid: true,
            }
        );
        assert_eq!(
            handle_check_password(&game_db, "admin", "wrong"),
            AdminResponse::PasswordChecked {
                username: "admin".to_string(),
                valid: false,
            }
        );
    }

    #[test]
    fn handle_list_characters_returns_account_characters() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };

        let response = handle_list_characters(&game_db, Some("admin"));

        assert!(matches!(
            response,
            AdminResponse::CharactersListed { characters }
                if characters.len() == 1
                    && characters[0].username == "admin"
                    && characters[0].account_id == account_id
                    && characters[0].name == "Theron"
        ));
    }

    #[test]
    fn handle_find_character_reports_owner() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        let (character_id, _data) =
            persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };

        assert_eq!(
            handle_find_character(&game_db, "Theron"),
            AdminResponse::CharacterFound {
                character: Some(CharacterInfo {
                    username: "admin".to_string(),
                    account_id,
                    character_id,
                    name: "Theron".to_string(),
                    level: 1,
                    race: 1,
                    class: 1,
                })
            }
        );
    }

    #[test]
    fn handle_move_characters_moves_ownership_between_accounts() {
        let db = persistence::temp_database();
        let (from_account_id, _token) =
            persistence_accounts::register_account(&db, "alice", "secret").expect("alice");
        let (to_account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, from_account_id, "Theron", 1, 1).expect("character");
        persistence::create_character(&db, from_account_id, "Elara", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };

        assert_eq!(
            handle_move_characters(&game_db, "alice", "admin"),
            AdminResponse::CharactersMoved {
                from_username: "alice".to_string(),
                to_username: "admin".to_string(),
                moved_count: 2,
            }
        );
        assert!(persistence::list_characters_for_account(&game_db.db, from_account_id).is_empty());
        let moved = persistence::list_characters_for_account(&game_db.db, to_account_id);
        assert_eq!(moved.len(), 2);
        assert!(moved.iter().any(|(_, data)| data.name == "Theron"));
        assert!(moved.iter().any(|(_, data)| data.name == "Elara"));
    }

    #[test]
    fn handle_set_equipment_appearance_updates_character_data() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let mut world = World::new();
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(&mut world);
        let (mut commands, mut query, authenticated_query, mut roster_update_senders) =
            state.get_mut(&mut world);

        let response = handle_set_equipment_appearance(
            &game_db,
            &mut commands,
            &mut query,
            &authenticated_query,
            &mut roster_update_senders,
            EquipmentAppearanceRequest {
                name: "Theron",
                slot_name: "chest",
                display_info_id: 777,
            },
        );

        assert_eq!(
            response,
            AdminResponse::EquipmentAppearanceSet {
                character_name: "Theron".to_string(),
                slot: "Chest".to_string(),
                display_info_id: 777,
            }
        );
        let char_id = persistence::find_character_id_by_name(&game_db.db, "Theron").unwrap();
        let data = persistence::load_character(&game_db.db, char_id).unwrap();
        assert_eq!(data.equipment_appearance.entries.len(), 1);
        assert_eq!(
            data.equipment_appearance.entries[0].slot,
            shared::components::EquipmentVisualSlot::Chest
        );
        assert_eq!(
            data.equipment_appearance.entries[0].display_info_id,
            Some(777)
        );
    }

    #[test]
    fn handle_clear_equipment_appearance_marks_slot_hidden() {
        let db = persistence::temp_database();
        let (account_id, _token) =
            persistence_accounts::register_account(&db, "admin", "admin").expect("admin");
        persistence::create_character(&db, account_id, "Theron", 1, 1).expect("character");
        let game_db = persistence::GameDatabase { db };
        let mut world = World::new();
        let mut state = bevy::ecs::system::SystemState::<(
            Commands,
            Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
            Query<(Entity, &crate::auth::Authenticated), With<MessageSender<LoginResponse>>>,
            Query<&mut MessageSender<CharacterListUpdate>>,
        )>::new(&mut world);
        let (mut commands, mut query, authenticated_query, mut roster_update_senders) =
            state.get_mut(&mut world);

        let response = handle_clear_equipment_appearance(
            &game_db,
            &mut commands,
            &mut query,
            &authenticated_query,
            &mut roster_update_senders,
            "Theron",
            "feet",
        );

        assert_eq!(
            response,
            AdminResponse::EquipmentAppearanceCleared {
                character_name: "Theron".to_string(),
                slot: "Feet".to_string(),
            }
        );
        let char_id = persistence::find_character_id_by_name(&game_db.db, "Theron").unwrap();
        let data = persistence::load_character(&game_db.db, char_id).unwrap();
        assert_eq!(data.equipment_appearance.entries.len(), 1);
        assert_eq!(
            data.equipment_appearance.entries[0].slot,
            shared::components::EquipmentVisualSlot::Feet
        );
        assert_eq!(data.equipment_appearance.entries[0].display_info_id, None);
        assert!(data.equipment_appearance.entries[0].hidden);
    }
}
