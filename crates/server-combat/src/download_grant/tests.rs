use super::*;
use std::io::{Read, Write};
use std::net::TcpStream;

fn post(port: u16, body: serde_json::Value) -> (u16, serde_json::Value) {
    let body = body.to_string();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(stream, "POST /v1/download-grant HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap();
    let status = headers
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (status, serde_json::from_str(body).unwrap())
}

#[test]
fn download_grant_http_and_vector() {
    let secret =
        decode_secret("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").unwrap();
    let expected = "v1.eyJhIjo0MiwiZSI6MjAwMDAwMDAwMH0.mXcooaLo1EvvcgDsnp2V13lQqYVuQaExIz5nRiq0vpk";
    let payload = GrantPayload {
        a: 42,
        e: 2_000_000_000,
    };
    assert_eq!(issue_grant(&secret, &payload), expected);
    assert_eq!(
        verify_grant(&secret, expected, 1_999_999_999),
        Some(payload)
    );
    assert!(verify_grant(&secret, expected, 2_000_000_000).is_none());
    assert!(verify_grant(&secret, &format!("{expected}x"), 0).is_none());
    assert!(verify_grant(&secret, &expected.replacen("v1.", "v2.", 1), 0).is_none());

    let dir = tempfile::tempdir().unwrap();
    let db = Arc::new(Database::create(dir.path().join("accounts.redb")).unwrap());
    crate::persistence::create_tables(&db);
    let (account, token) =
        crate::persistence_accounts::register_account(&db, "alice", "correct").unwrap();
    let server = Server::http(("127.0.0.1", 0)).unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let server_db = Arc::clone(&db);
    let handle = std::thread::spawn(move || {
        for _ in 0..4 {
            handle_request(server.recv().unwrap(), &server_db, &secret);
        }
    });
    let before = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let (status, response) = post(
        port,
        serde_json::json!({"username":"alice", "password":"correct"}),
    );
    assert_eq!(status, 200);
    let expires = response["expires_at"].as_u64().unwrap();
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!((before + TTL_SECONDS..=after + TTL_SECONDS).contains(&expires));
    assert_eq!(
        verify_grant(&secret, response["grant"].as_str().unwrap(), before),
        Some(GrantPayload {
            a: account,
            e: expires
        })
    );
    assert!(response["token"].as_str().is_some());
    let (status, response) = post(port, serde_json::json!({"token":token}));
    assert_eq!(status, 200);
    assert_eq!(response["token"], token);
    assert_eq!(
        verify_grant(&secret, response["grant"].as_str().unwrap(), before)
            .unwrap()
            .a,
        account
    );
    assert_eq!(
        post(
            port,
            serde_json::json!({"username":"alice", "password":"wrong"})
        )
        .0,
        401
    );
    crate::persistence_accounts::ban_account(&db, "alice", "test ban").unwrap();
    assert_eq!(post(port, serde_json::json!({"token":token})).0, 401);
    handle.join().unwrap();
}
