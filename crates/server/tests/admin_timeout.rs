//! Process-level admin IPC tests; no game server is started.
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const TEST_TIMEOUT_MS: &str = "100";
const PROCESS_LIMIT: Duration = Duration::from_secs(3);

fn run_admin(socket: &Path, timeout: Option<&str>) -> (Output, Duration) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_game-server-admin"));
    command
        .arg("ping")
        .env("GAME_SERVER_ADMIN_SOCKET", socket)
        .env_remove("GAME_SERVER_ADMIN_TIMEOUT_MS")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(timeout) = timeout {
        command.env("GAME_SERVER_ADMIN_TIMEOUT_MS", timeout);
    }
    let start = Instant::now();
    let mut child = command.spawn().unwrap();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > PROCESS_LIMIT {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("admin client exceeded process limit: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    (child.wait_with_output().unwrap(), start.elapsed())
}

#[test]
fn admin_timeout_accepts_request_but_never_replies() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("admin.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let (release, held) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        use std::io::Read;
        let (mut connection, _) = listener.accept().unwrap();
        let mut request = [0; 16];
        assert!(connection.read(&mut request).unwrap() > 0);
        held.recv().unwrap();
    });
    let (output, elapsed) = run_admin(&socket, Some(TEST_TIMEOUT_MS));
    release.send(()).unwrap();
    server.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(elapsed < PROCESS_LIMIT);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("timed out after 100 ms"), "{stderr}");
    assert!(stderr.contains("ping"), "{stderr}");
    assert!(stderr.contains(socket.to_str().unwrap()), "{stderr}");
}

#[test]
fn admin_timeout_normal_reply_preserves_output() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("admin.sock");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = {
        let _entered = runtime.enter();
        peercred_ipc::Server::bind(&socket).unwrap()
    };
    let server = std::thread::spawn(move || {
        runtime.block_on(async move {
            #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
            enum Request {
                Ping,
            }
            #[derive(serde::Serialize)]
            enum Response {
                Pong,
            }
            let (mut connection, _) = listener.accept().await.unwrap();
            assert_eq!(connection.read::<Request>().await.unwrap(), Request::Ping);
            connection.write(&Response::Pong).await.unwrap();
        });
    });
    let (output, _) = run_admin(&socket, None);
    server.join().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "pong\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn admin_timeout_missing_socket_preserves_error() {
    let directory = tempfile::tempdir().unwrap();
    let (output, _) = run_admin(&directory.path().join("missing.sock"), None);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Failed to connect: io: No such file or directory (os error 2)\n"
    );
}

#[test]
fn admin_timeout_rejects_invalid_configuration() {
    let directory = tempfile::tempdir().unwrap();
    for value in ["0", "-1", "bad", "18446744073709551616"] {
        let (output, _) = run_admin(&directory.path().join("missing.sock"), Some(value));
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("GAME_SERVER_ADMIN_TIMEOUT_MS"), "{stderr}");
        assert!(stderr.contains("positive integer"), "{stderr}");
    }
}
