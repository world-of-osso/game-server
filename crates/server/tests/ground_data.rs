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
    // Stage deployed startup data, not slot-local CASC exports.
    std::fs::create_dir(cwd.join("data")).unwrap();
    std::os::unix::fs::symlink(
        server_data.join("world.db").canonicalize().unwrap(),
        cwd.join("data/world.db"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        server_data.join("economy").canonicalize().unwrap(),
        cwd.join("data/economy"),
    )
    .unwrap();
    // deploy.sh also ships the installed-build DB2 directory the realm reads at startup.
    std::os::unix::fs::symlink(
        server_data.join("db2").canonicalize().unwrap(),
        cwd.join("data/db2"),
    )
    .unwrap();
    for name in ["toy_required_areas.json", "toy_facing_requirements.json"] {
        std::os::unix::fs::symlink(
            server_data.join(name).canonicalize().unwrap(),
            cwd.join("data").join(name),
        )
        .unwrap();
    }
    let content = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content");
    for name in [
        "trading-post-monthly-catalog.json",
        "quest-reward-bonus.json",
    ] {
        std::fs::copy(content.join(name), cwd.join("data").join(name))
            .unwrap_or_else(|error| panic!("Stage tracked startup catalog {name}: {error}"));
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_game-server"));
    command
        .current_dir(cwd)
        .env("GAME_SERVER_BIND_IP", "127.0.0.1")
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
fn server_without_ground_exits_at_startup_naming_the_missing_files() {
    let cwd = tempfile::tempdir().unwrap();
    let missing = cwd.path().join("no-ground");
    let mut server = start_server(cwd.path(), Some(&missing));

    let (status, stderr) = wait_for(&mut server, cwd.path(), "Ground loads from");

    let status =
        status.unwrap_or_else(|| panic!("server started without ground:\n{}", tail(&stderr)));
    assert!(!status.success(), "{status}");
    assert!(
        stderr.contains(&format!(
            "No ground from {} (set GAME_SERVER_GROUND_DIR): ",
            missing.display()
        )) && stderr.contains("files missing: map 0: terrain/775971.wdt, "),
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
    let gap_files = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ground_gaps.tsv"),
    )
    .unwrap()
    .lines()
    .count();
    assert!(
        stderr.contains(&format!("Ground known gaps: {gap_files} files of maps ")),
        "{}",
        tail(&stderr)
    );
}

/// The client cache with `terrain/` whole and every `models/` file but `without`, by symlink.
fn cache_without(dir: &Path, without: &str) {
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../game-engine/data")
        .canonicalize()
        .unwrap();
    std::os::unix::fs::symlink(cache.join("terrain"), dir.join("terrain")).unwrap();
    std::fs::create_dir(dir.join("models")).unwrap();
    for entry in std::fs::read_dir(cache.join("models")).unwrap() {
        let name = entry.unwrap().file_name();
        if name != without {
            std::os::unix::fs::symlink(
                cache.join("models").join(&name),
                dir.join("models").join(&name),
            )
            .unwrap();
        }
    }
}

#[test]
fn server_missing_a_spawned_maps_wmo_group_exits_at_startup_naming_map_and_file() {
    let cwd = tempfile::tempdir().unwrap();
    let ground = cwd.path().join("ground");
    std::fs::create_dir(&ground).unwrap();
    // The Stockade (map 34) is WMO 108631; 108643 is one of its groups.
    cache_without(&ground, "108643.wmo");
    let mut server = start_server(cwd.path(), Some(&ground));

    let (status, stderr) = wait_for(&mut server, cwd.path(), "Ground loads from");

    let status = status
        .unwrap_or_else(|| panic!("server started without map 34 ground:\n{}", tail(&stderr)));
    assert!(!status.success(), "{status}");
    assert!(
        stderr.contains(&format!(
            "No ground from {} (set GAME_SERVER_GROUND_DIR): 1 files missing: map 34: models/108643.wmo",
            ground.display()
        )),
        "{}",
        tail(&stderr)
    );
}
