//! The server's ground at process startup. It used to load from `../game-engine/data`
//! relative to the working directory; started elsewhere it ran without ground and every
//! creature kept its spawn height wherever it ran (the Northshire Blackrock Spy chased
//! and wandered polymorphed 5 yd above the ridge it spawned on).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const STARTUP_LIMIT: Duration = Duration::from_secs(180);

fn free_udp_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// The server started from `cwd`, a scratch directory holding only `data/` (the server's
/// world.db), so `../game-engine/data` does not exist from it.
fn start_server(cwd: &Path, ground_dir: Option<&Path>) -> Child {
    let server_data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    std::os::unix::fs::symlink(server_data.canonicalize().unwrap(), cwd.join("data")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_game-server"));
    command
        .current_dir(cwd)
        .env("GAME_SERVER_PORT", free_udp_port().to_string())
        .env("GAME_SERVER_DB_PATH", cwd.join("game.redb"))
        .env("GAME_SERVER_ADMIN_SOCKET", cwd.join("admin.sock"))
        .env_remove("GAME_SERVER_GROUND_DIR")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(cwd.join("server.err")).unwrap());
    if let Some(dir) = ground_dir {
        command.env("GAME_SERVER_GROUND_DIR", dir);
    }
    command.spawn().unwrap()
}

/// Waits until the server exits or its log shows `line`; `None` while still running.
fn wait_for(server: &mut Child, cwd: &Path, line: &str) -> (Option<ExitStatus>, String) {
    let started = Instant::now();
    loop {
        let stderr = std::fs::read_to_string(cwd.join("server.err")).unwrap();
        if let Some(status) = server.try_wait().unwrap() {
            return (Some(status), stderr);
        }
        if stderr.contains(line) || started.elapsed() > STARTUP_LIMIT {
            server.kill().unwrap();
            server.wait().unwrap();
            return (None, stderr);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn tail(log: &str) -> String {
    let lines: Vec<_> = log.lines().collect();
    lines[lines.len().saturating_sub(30)..].join("\n")
}

#[test]
fn server_without_azeroth_ground_exits_at_startup_naming_the_wdt() {
    let cwd = tempfile::tempdir().unwrap();
    let missing = cwd.path().join("no-ground");
    let mut server = start_server(cwd.path(), Some(&missing));

    let (status, stderr) = wait_for(&mut server, cwd.path(), "Ground loads from");

    let status =
        status.unwrap_or_else(|| panic!("server started without ground:\n{}", tail(&stderr)));
    assert!(!status.success(), "{status}");
    let wdt = missing.join("terrain/775971.wdt");
    assert!(
        stderr.contains(&format!("No ground on map 0 from {}", missing.display()))
            && stderr.contains(&format!("{}: No such file or directory", wdt.display())),
        "{}",
        tail(&stderr)
    );
}

#[test]
fn server_started_outside_the_checkout_loads_ground_beside_the_checkout() {
    let cwd = tempfile::tempdir().unwrap();
    let mut server = start_server(cwd.path(), None);

    let (status, stderr) = wait_for(&mut server, cwd.path(), "Ground loads from");

    assert_eq!(status, None, "server exited:\n{}", tail(&stderr));
    let beside = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data");
    assert!(
        stderr.contains(&format!("Ground loads from {}", beside.display())),
        "{}",
        tail(&stderr)
    );
}
