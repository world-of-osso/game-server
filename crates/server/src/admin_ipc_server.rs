use std::path::PathBuf;
use std::sync::mpsc;

use peercred_ipc::{Connection, Server};

use super::SOCKET_PATH;
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

/// Spawn the IPC server on a tokio runtime in a background thread.
pub(super) fn init() -> (mpsc::Receiver<AdminCommand>, SocketGuard) {
    let path = PathBuf::from(SOCKET_PATH);

    // Remove stale socket from a previous run.
    let _ = std::fs::remove_file(&path);

    let (tx, rx) = mpsc::channel();
    let socket_path = path.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(serve(socket_path, tx));
    });

    (rx, SocketGuard { path })
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
