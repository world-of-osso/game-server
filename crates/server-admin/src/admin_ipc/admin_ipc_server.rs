use std::path::{Path, PathBuf};
use std::sync::mpsc;

use peercred_ipc::{Connection, Server};

use super::socket_path;
use crate::admin_types::{AdminRequest, AdminResponse};

/// Internal command passed from IPC server thread to Bevy main loop.
pub(super) struct AdminCommand {
    pub request: AdminRequest,
    pub respond: mpsc::Sender<AdminResponse>,
}

/// RAII guard that removes the socket file on drop.
pub(super) struct SocketGuard {
    pub path: PathBuf,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Make `path` free for this server's admin socket: a socket file nothing listens on
/// (left by a server that did not shut down) is removed; one a running server answers
/// on is refused, so a second server never takes a live server's socket.
pub(super) fn claim_socket_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if std::os::unix::net::UnixStream::connect(path).is_ok() {
        return Err(format!(
            "admin socket {} is in use by a running game server; set \
             GAME_SERVER_ADMIN_SOCKET to another path for this server",
            path.display()
        ));
    }
    std::fs::remove_file(path)
        .map_err(|error| format!("remove stale admin socket {}: {error}", path.display()))
}

/// Spawn the IPC server on a tokio runtime in a background thread. Fails when a live
/// server holds the socket path (`claim_socket_path`).
pub(super) fn init() -> Result<(mpsc::Receiver<AdminCommand>, SocketGuard), String> {
    let path = socket_path();
    claim_socket_path(&path)?;

    let (tx, rx) = mpsc::channel();
    let socket_path = path.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(serve(socket_path, tx));
    });

    Ok((rx, SocketGuard { path }))
}

async fn serve(path: PathBuf, tx: mpsc::Sender<AdminCommand>) {
    let server = match Server::bind(&path) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("admin ipc: bind failed: {error}");
            return;
        }
    };

    loop {
        let (connection, _caller) = match server.accept().await {
            Ok(pair) => pair,
            Err(error) => {
                eprintln!("admin ipc: accept error: {error}");
                continue;
            }
        };
        let tx = tx.clone();
        tokio::spawn(handle_connection(connection, tx));
    }
}

async fn handle_connection(mut connection: Connection, tx: mpsc::Sender<AdminCommand>) {
    let request: AdminRequest = match connection.read().await {
        Ok(request) => request,
        Err(error) => {
            eprintln!("admin ipc: read error: {error}");
            return;
        }
    };

    let (response_tx, response_rx) = mpsc::channel();
    let command = AdminCommand {
        request,
        respond: response_tx,
    };

    if tx.send(command).is_err() {
        return;
    }

    let response = match response_rx.recv() {
        Ok(response) => response,
        Err(_) => AdminResponse::Error("internal: channel closed".into()),
    };

    if let Err(error) = connection.write(&response).await {
        eprintln!("admin ipc: write error: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;

    use super::claim_socket_path;

    fn temp_socket(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("admin-ipc-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("admin.sock")
    }

    /// A second server does not take over a socket a live server listens on.
    #[test]
    fn a_live_admin_socket_is_refused_and_left_alone() {
        let path = temp_socket("live");
        let _ = std::fs::remove_file(&path);
        let live = UnixListener::bind(&path).unwrap();
        let error = claim_socket_path(&path).unwrap_err();
        assert!(
            error.contains("in use") && error.contains("GAME_SERVER_ADMIN_SOCKET"),
            "{error}"
        );
        assert!(path.exists());
        // The first server still accepts connections.
        let _client = std::os::unix::net::UnixStream::connect(&path).unwrap();
        live.accept().unwrap();
    }

    /// A socket file nothing listens on (a killed server's) is removed; a missing one is fine.
    #[test]
    fn a_stale_admin_socket_file_is_replaced() {
        let path = temp_socket("stale");
        let _ = std::fs::remove_file(&path);
        drop(UnixListener::bind(&path).unwrap());
        assert!(path.exists());
        assert_eq!(claim_socket_path(&path), Ok(()));
        assert!(!path.exists());
        assert_eq!(claim_socket_path(&path), Ok(()));
        UnixListener::bind(&path).unwrap();
    }
}
