use std::path::PathBuf;
use std::sync::mpsc;

use bevy::prelude::*;
use peercred_ipc::{Connection, Server};
use serde::{Deserialize, Serialize};

use crate::persistence;
use crate::persistence_accounts;

const SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

/// IPC request from CLI to server.
#[derive(Debug, Serialize, Deserialize)]
pub enum AdminRequest {
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
    Ping,
}

/// IPC response from server to CLI.
#[derive(Debug, Serialize, Deserialize)]
pub enum AdminResponse {
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
    Pong,
    Error(String),
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
) {
    let Some(db) = db else { return };
    while let Ok(cmd) = receiver.try_recv() {
        dispatch(cmd, &db);
    }
}

fn dispatch(cmd: AdminCommand, db: &persistence::GameDatabase) {
    let response = match cmd.request {
        AdminRequest::Ping => AdminResponse::Pong,
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
    };
    let _ = cmd.respond.send(response);
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
