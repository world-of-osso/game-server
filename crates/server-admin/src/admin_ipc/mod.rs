pub mod admin_ipc_server;

const DEFAULT_SOCKET_PATH: &str = "/tmp/game-server-admin.sock";

/// Admin socket; `GAME_SERVER_ADMIN_SOCKET` gives a second server (own
/// `GAME_SERVER_PORT`) its own socket instead of taking over the default one.
pub(super) fn socket_path() -> std::path::PathBuf {
    std::env::var_os("GAME_SERVER_ADMIN_SOCKET")
        .map_or_else(|| DEFAULT_SOCKET_PATH.into(), Into::into)
}
