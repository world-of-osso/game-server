use std::path::PathBuf;
use std::sync::mpsc;

use bevy::prelude::*;
use peercred_ipc::{Connection, Server};
use serde::{Deserialize, Serialize};
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry,
    Position,
};

use crate::persistence;
use crate::persistence_accounts;

const SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

/// IPC request from CLI to server.
#[derive(Debug, Serialize, Deserialize)]
pub enum AdminRequest {
    ListAccounts,
    ListCharacters {
        username: Option<String>,
    },
    FindCharacter {
        name: String,
    },
    MoveCharacters {
        from_username: String,
        to_username: String,
    },
    CheckAccount {
        username: String,
    },
    CheckPassword {
        username: String,
        password: String,
    },
    CreateAccount {
        username: String,
        password: String,
    },
    ApproveRegistration {
        username: String,
    },
    CreateGuild {
        character_name: String,
        guild_id: u32,
        guild_name: String,
    },
    SetLevel {
        character_name: String,
        level: u16,
    },
    SetCredentials {
        token: String,
        username: String,
        password: String,
    },
    SetPosition {
        character_name: String,
        x: f32,
        y: f32,
        z: f32,
    },
    SetAppearance {
        character_name: String,
        appearance: CharacterAppearance,
    },
    SetEquipmentAppearance {
        character_name: String,
        slot: String,
        display_info_id: u32,
    },
    ClearEquipmentAppearance {
        character_name: String,
        slot: String,
    },
    Ping,
}

/// IPC response from server to CLI.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AdminResponse {
    AccountsListed {
        accounts: Vec<AccountInfo>,
    },
    CharactersListed {
        characters: Vec<CharacterInfo>,
    },
    CharacterFound {
        character: Option<CharacterInfo>,
    },
    CharactersMoved {
        from_username: String,
        to_username: String,
        moved_count: usize,
    },
    AccountChecked {
        username: String,
        exists: bool,
        account_id: Option<u64>,
        token_count: usize,
        pending_registration: bool,
    },
    PasswordChecked {
        username: String,
        valid: bool,
    },
    AccountCreated {
        account_id: u64,
        token: String,
    },
    RegistrationApproved {
        account_id: u64,
        token: String,
    },
    GuildCreated {
        character_id: u64,
        guild_id: u32,
        guild_name: String,
    },
    LevelSet {
        character_name: String,
        level: u16,
    },
    CredentialsSet {
        account_id: u64,
        username: String,
    },
    PositionSet {
        character_name: String,
        x: f32,
        y: f32,
        z: f32,
    },
    AppearanceSet {
        character_name: String,
        appearance: CharacterAppearance,
    },
    EquipmentAppearanceSet {
        character_name: String,
        slot: String,
        display_info_id: u32,
    },
    EquipmentAppearanceCleared {
        character_name: String,
        slot: String,
    },
    Pong,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountInfo {
    pub username: String,
    pub account_id: u64,
    pub token_count: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterInfo {
    pub username: String,
    pub account_id: u64,
    pub character_id: u64,
    pub name: String,
    pub level: u16,
    pub race: u8,
    pub class: u8,
}

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

/// Poll IPC commands each frame and dispatch them.
fn poll_admin_ipc(
    receiver: NonSend<mpsc::Receiver<AdminCommand>>,
    db: Option<Res<persistence::GameDatabase>>,
    mut commands: Commands,
    mut players: Query<(&mut Position, &persistence::ClientId)>,
    mut equipment_query: Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
) {
    let Some(db) = db else { return };
    while let Ok(cmd) = receiver.try_recv() {
        dispatch(
            cmd,
            &db,
            &mut commands,
            &mut players,
            &mut equipment_query,
        );
    }
}

fn dispatch(
    cmd: AdminCommand,
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    players: &mut Query<(&mut Position, &persistence::ClientId)>,
    equipment_query: &mut Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
) {
    let response = match cmd.request {
        AdminRequest::Ping => AdminResponse::Pong,
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
        } => handle_set_appearance(db, &character_name, appearance),
        AdminRequest::SetEquipmentAppearance {
            character_name,
            slot,
            display_info_id,
        } => handle_set_equipment_appearance(
            db,
            commands,
            equipment_query,
            &character_name,
            &slot,
            display_info_id,
        ),
        AdminRequest::ClearEquipmentAppearance {
            character_name,
            slot,
        } => handle_clear_equipment_appearance(db, commands, equipment_query, &character_name, &slot),
    };
    let _ = cmd.respond.send(response);
}

fn handle_list_accounts(db: &persistence::GameDatabase) -> AdminResponse {
    let accounts = persistence_accounts::list_accounts(&db.db)
        .into_iter()
        .map(|account| AccountInfo {
            username: account.username,
            account_id: account.account_id,
            token_count: account.token_count,
        })
        .collect();
    AdminResponse::AccountsListed { accounts }
}

fn handle_list_characters(
    db: &persistence::GameDatabase,
    username_filter: Option<&str>,
) -> AdminResponse {
    let mut characters: Vec<_> = persistence_accounts::list_accounts(&db.db)
        .into_iter()
        .filter(|account| {
            username_filter
                .map(|username| account.username == username)
                .unwrap_or(true)
        })
        .flat_map(|account| {
            let username = account.username.clone();
            persistence::list_characters_for_account(&db.db, account.account_id)
                .into_iter()
                .map(move |(character_id, data)| CharacterInfo {
                    username: username.clone(),
                    account_id: account.account_id,
                    character_id,
                    name: data.name,
                    level: data.level,
                    race: data.race,
                    class: data.class,
                })
        })
        .collect();
    characters.sort_by(|a, b| {
        a.username
            .cmp(&b.username)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.character_id.cmp(&b.character_id))
    });
    AdminResponse::CharactersListed { characters }
}

fn handle_find_character(db: &persistence::GameDatabase, name: &str) -> AdminResponse {
    let character = match handle_list_characters(db, None) {
        AdminResponse::CharactersListed { characters } => {
            characters.into_iter().find(|character| character.name == name)
        }
        _ => None,
    };
    AdminResponse::CharacterFound { character }
}

fn handle_move_characters(
    db: &persistence::GameDatabase,
    from_username: &str,
    to_username: &str,
) -> AdminResponse {
    let Some(from_account_id) = persistence_accounts::find_account_by_username(&db.db, from_username)
    else {
        return AdminResponse::Error(format!("account '{from_username}' not found"));
    };
    let Some(to_account_id) = persistence_accounts::find_account_by_username(&db.db, to_username)
    else {
        return AdminResponse::Error(format!("account '{to_username}' not found"));
    };
    match persistence::move_characters_between_accounts(&db.db, from_account_id, to_account_id) {
        Ok(moved_count) => AdminResponse::CharactersMoved {
            from_username: from_username.to_owned(),
            to_username: to_username.to_owned(),
            moved_count,
        },
        Err(err) => AdminResponse::Error(err),
    }
}

fn handle_check_account(db: &persistence::GameDatabase, username: &str) -> AdminResponse {
    let account = persistence_accounts::inspect_account(&db.db, username);
    AdminResponse::AccountChecked {
        username: account.username,
        exists: account.exists,
        account_id: account.account_id,
        token_count: account.token_count,
        pending_registration: account.pending_registration,
    }
}

fn handle_check_password(
    db: &persistence::GameDatabase,
    username: &str,
    password: &str,
) -> AdminResponse {
    match persistence_accounts::check_password(&db.db, username, password) {
        Ok(valid) => AdminResponse::PasswordChecked {
            username: username.to_owned(),
            valid,
        },
        Err(err) => AdminResponse::Error(err),
    }
}

fn handle_create_account(
    db: &persistence::GameDatabase,
    username: &str,
    password: &str,
) -> AdminResponse {
    match persistence_accounts::register_account(&db.db, username, password) {
        Ok((account_id, token)) => AdminResponse::AccountCreated { account_id, token },
        Err(e) => AdminResponse::Error(e),
    }
}

fn handle_approve_registration(db: &persistence::GameDatabase, username: &str) -> AdminResponse {
    match persistence_accounts::approve_registration(&db.db, username) {
        Ok((account_id, token)) => AdminResponse::RegistrationApproved { account_id, token },
        Err(e) => AdminResponse::Error(e),
    }
}

fn handle_set_credentials(
    db: &persistence::GameDatabase,
    token: &str,
    username: &str,
    password: &str,
) -> AdminResponse {
    match persistence_accounts::set_account_credentials(&db.db, token, username, password) {
        Ok(account_id) => AdminResponse::CredentialsSet {
            account_id,
            username: username.to_owned(),
        },
        Err(e) => AdminResponse::Error(e),
    }
}

fn handle_set_level(db: &persistence::GameDatabase, name: &str, level: u16) -> AdminResponse {
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };
    data.level = level;
    persistence::save_character(&db.db, char_id, &data);
    AdminResponse::LevelSet {
        character_name: name.to_owned(),
        level,
    }
}

fn handle_set_position(
    db: &persistence::GameDatabase,
    players: &mut Query<(&mut Position, &persistence::ClientId)>,
    name: &str,
    x: f32,
    y: f32,
    z: f32,
) -> AdminResponse {
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };
    data.x = x;
    data.y = y;
    data.z = z;
    persistence::save_character(&db.db, char_id, &data);

    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, z);
    for (mut pos, cid) in players.iter_mut() {
        if cid.0 == char_id {
            pos.x = bx;
            pos.y = by;
            pos.z = bz;
        }
    }

    AdminResponse::PositionSet {
        character_name: name.to_owned(),
        x,
        y,
        z,
    }
}

fn handle_set_appearance(
    db: &persistence::GameDatabase,
    name: &str,
    appearance: CharacterAppearance,
) -> AdminResponse {
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };
    data.appearance = appearance;
    persistence::save_character(&db.db, char_id, &data);

    AdminResponse::AppearanceSet {
        character_name: name.to_owned(),
        appearance,
    }
}

fn handle_set_equipment_appearance(
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    equipment_query: &mut Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
    name: &str,
    slot_name: &str,
    display_info_id: u32,
) -> AdminResponse {
    let Some(slot) = parse_equipment_visual_slot(slot_name) else {
        return AdminResponse::Error(format!(
            "invalid equipment appearance slot '{slot_name}'"
        ));
    };
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };

    upsert_equipment_appearance_entry(&mut data.equipment_appearance, slot, display_info_id);
    persistence::save_character(&db.db, char_id, &data);

    for (entity, mut appearance, cid) in equipment_query.iter_mut() {
        if cid.0 == char_id {
            upsert_equipment_appearance_entry(&mut appearance, slot, display_info_id);
            commands.entity(entity).insert(appearance.clone());
        }
    }

    AdminResponse::EquipmentAppearanceSet {
        character_name: name.to_owned(),
        slot: format!("{slot:?}"),
        display_info_id,
    }
}

fn handle_clear_equipment_appearance(
    db: &persistence::GameDatabase,
    commands: &mut Commands,
    equipment_query: &mut Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>,
    name: &str,
    slot_name: &str,
) -> AdminResponse {
    let Some(slot) = parse_equipment_visual_slot(slot_name) else {
        return AdminResponse::Error(format!(
            "invalid equipment appearance slot '{slot_name}'"
        ));
    };
    let Some(char_id) = persistence::find_character_id_by_name(&db.db, name) else {
        return AdminResponse::Error(format!("character '{name}' not found"));
    };
    let Some(mut data) = persistence::load_character(&db.db, char_id) else {
        return AdminResponse::Error(format!("character data for '{name}' not found"));
    };

    upsert_hidden_equipment_appearance_entry(&mut data.equipment_appearance, slot);
    persistence::save_character(&db.db, char_id, &data);

    for (entity, mut appearance, cid) in equipment_query.iter_mut() {
        if cid.0 == char_id {
            upsert_hidden_equipment_appearance_entry(&mut appearance, slot);
            commands.entity(entity).insert(appearance.clone());
        }
    }

    AdminResponse::EquipmentAppearanceCleared {
        character_name: name.to_owned(),
        slot: format!("{slot:?}"),
    }
}

fn upsert_equipment_appearance_entry(
    appearance: &mut EquipmentAppearance,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
) {
    if let Some(entry) = appearance.entries.iter_mut().find(|entry| entry.slot == slot) {
        entry.display_info_id = Some(display_info_id);
        entry.hidden = false;
        return;
    }
    appearance.entries.push(EquippedAppearanceEntry {
        slot,
        item_id: None,
        display_info_id: Some(display_info_id),
        inventory_type: 0,
        hidden: false,
    });
}

fn upsert_hidden_equipment_appearance_entry(
    appearance: &mut EquipmentAppearance,
    slot: EquipmentVisualSlot,
) {
    if let Some(entry) = appearance.entries.iter_mut().find(|entry| entry.slot == slot) {
        entry.item_id = None;
        entry.display_info_id = None;
        entry.inventory_type = 0;
        entry.hidden = true;
        return;
    }
    appearance.entries.push(EquippedAppearanceEntry {
        slot,
        item_id: None,
        display_info_id: None,
        inventory_type: 0,
        hidden: true,
    });
}

fn parse_equipment_visual_slot(value: &str) -> Option<EquipmentVisualSlot> {
    match value.to_ascii_lowercase().as_str() {
        "head" => Some(EquipmentVisualSlot::Head),
        "shoulder" | "shoulders" => Some(EquipmentVisualSlot::Shoulder),
        "back" | "cloak" | "cape" => Some(EquipmentVisualSlot::Back),
        "chest" => Some(EquipmentVisualSlot::Chest),
        "shirt" => Some(EquipmentVisualSlot::Shirt),
        "tabard" => Some(EquipmentVisualSlot::Tabard),
        "wrist" | "wrists" | "bracers" => Some(EquipmentVisualSlot::Wrist),
        "hands" | "hand" | "gloves" => Some(EquipmentVisualSlot::Hands),
        "waist" | "belt" => Some(EquipmentVisualSlot::Waist),
        "legs" | "pants" => Some(EquipmentVisualSlot::Legs),
        "feet" | "boots" => Some(EquipmentVisualSlot::Feet),
        "mainhand" | "main-hand" | "mh" => Some(EquipmentVisualSlot::MainHand),
        "offhand" | "off-hand" | "oh" => Some(EquipmentVisualSlot::OffHand),
        _ => None,
    }
}

fn handle_create_guild(
    db: &persistence::GameDatabase,
    name: &str,
    guild_id: u32,
    guild_name: &str,
) -> AdminResponse {
    match persistence::set_character_guild_by_name(&db.db, name, guild_id, guild_name) {
        Ok(character_id) => AdminResponse::GuildCreated {
            character_id,
            guild_id,
            guild_name: guild_name.to_owned(),
        },
        Err(e) => AdminResponse::Error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

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
        let mut state = bevy::ecs::system::SystemState::<
            (Commands, Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>),
        >::new(&mut world);
        let (mut commands, mut query) = state.get_mut(&mut world);

        let response =
            handle_set_equipment_appearance(
                &game_db,
                &mut commands,
                &mut query,
                "Theron",
                "chest",
                777,
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
        assert_eq!(data.equipment_appearance.entries[0].slot, EquipmentVisualSlot::Chest);
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
        let mut state = bevy::ecs::system::SystemState::<
            (Commands, Query<(Entity, &mut EquipmentAppearance, &persistence::ClientId)>),
        >::new(&mut world);
        let (mut commands, mut query) = state.get_mut(&mut world);

        let response = handle_clear_equipment_appearance(
            &game_db,
            &mut commands,
            &mut query,
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
        assert_eq!(data.equipment_appearance.entries[0].slot, EquipmentVisualSlot::Feet);
        assert_eq!(data.equipment_appearance.entries[0].display_info_id, None);
        assert!(data.equipment_appearance.entries[0].hidden);
    }
}
