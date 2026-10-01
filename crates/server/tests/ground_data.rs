//! The server process started from a working directory without the client's terrain
//! (`../game-engine/data`): it ran without ground, and every creature kept its spawn
//! height wherever it ran (the Northshire Blackrock Spy chased and wandered polymorphed
//! 5 yd above the ridge it spawned on). It now refuses to start.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const STARTUP_LIMIT: Duration = Duration::from_secs(180);

fn free_udp_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn server_without_client_terrain_exits_at_startup_naming_the_missing_directory() {
    let cwd = tempfile::tempdir().unwrap();
    let server_data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    std::os::unix::fs::symlink(server_data.canonicalize().unwrap(), cwd.path().join("data"))
        .unwrap();
    let mut server = Command::new(env!("CARGO_BIN_EXE_game-server"))
        .current_dir(cwd.path())
        .env("GAME_SERVER_PORT", free_udp_port().to_string())
        .env("GAME_SERVER_DB_PATH", cwd.path().join("game.redb"))
        .env("GAME_SERVER_ADMIN_SOCKET", cwd.path().join("admin.sock"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(cwd.path().join("server.err")).unwrap())
        .spawn()
        .unwrap();

    let started = Instant::now();
    let status = loop {
        if let Some(status) = server.try_wait().unwrap() {
            break Some(status);
        }
        if started.elapsed() > STARTUP_LIMIT {
            server.kill().unwrap();
            server.wait().unwrap();
            break None;
        }
        std::thread::sleep(Duration::from_millis(200));
    };

    let stderr = std::fs::read_to_string(cwd.path().join("server.err")).unwrap();
    let lines: Vec<_> = stderr.lines().collect();
    let tail = lines[lines.len().saturating_sub(30)..].join("\n");
    let status = status.unwrap_or_else(|| {
        panic!("server still running after {STARTUP_LIMIT:?} without ground; stderr tail:\n{tail}")
    });
    assert!(
        !status.success(),
        "server exited successfully: {status}; stderr tail:\n{tail}"
    );
    assert!(
        stderr.contains("No ground: ../game-engine/data/terrain is not a directory"),
        "stderr tail:\n{tail}"
    );
}
