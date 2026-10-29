//! Loopback HTTP issuer and v1 download grant verification.
use std::io::Read;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use redb::Database;
use ring::hmac;
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Response, Server};

const TTL_SECONDS: u64 = 6 * 60 * 60;
const MAX_BODY: u64 = 8192;
const DEFAULT_PORT: u16 = 8088;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrantPayload {
    pub a: u64,
    pub e: u64,
}

pub fn issue_grant(secret: &[u8; 32], payload: &GrantPayload) -> String {
    let payload = serde_json::to_vec(payload).expect("grant payload serializes");
    let message = format!("v1.{}", URL_SAFE_NO_PAD.encode(payload));
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    let signature = hmac::sign(&key, message.as_bytes());
    format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature.as_ref()))
}

pub fn verify_grant(secret: &[u8; 32], grant: &str, now: u64) -> Option<GrantPayload> {
    let (message, signature) = grant.rsplit_once('.')?;
    let encoded = message.strip_prefix("v1.")?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    hmac::verify(&key, message.as_bytes(), &signature).ok()?;
    let payload: GrantPayload =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).ok()?).ok()?;
    (payload.e > now).then_some(payload)
}

fn decode_secret(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 || !hex.is_ascii() {
        return Err("WOO_DOWNLOAD_GRANT_SECRET must be 64 hexadecimal characters".into());
    }
    let mut secret = [0; 32];
    for (i, byte) in secret.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| "WOO_DOWNLOAD_GRANT_SECRET must be hexadecimal")?;
    }
    Ok(secret)
}

/// Start only when the secret is configured; malformed configuration fails startup.
pub(crate) fn start_from_env(db: Arc<Database>) -> Result<(), String> {
    let Some(hex) = std::env::var_os("WOO_DOWNLOAD_GRANT_SECRET") else {
        return Ok(());
    };
    let hex = hex.into_string().map_err(|_| "secret is not UTF-8")?;
    let secret = decode_secret(&hex)?;
    let port = match std::env::var("WOO_DOWNLOAD_GRANT_PORT") {
        Ok(value) => value
            .parse::<u16>()
            .map_err(|_| "invalid WOO_DOWNLOAD_GRANT_PORT")?,
        Err(std::env::VarError::NotPresent) => DEFAULT_PORT,
        Err(err) => return Err(format!("invalid WOO_DOWNLOAD_GRANT_PORT: {err}")),
    };
    let server = Server::http(("127.0.0.1", port)).map_err(|err| err.to_string())?;
    bevy::log::info!(
        "Download grant issuer listening on {}",
        server.server_addr()
    );
    std::thread::Builder::new()
        .name("download-grant".into())
        .spawn(move || {
            for request in server.incoming_requests() {
                handle_request(request, &db, &secret);
            }
        })
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum Login {
    Token { token: String },
    Credentials { username: String, password: String },
}

fn handle_request(mut request: tiny_http::Request, db: &Database, secret: &[u8; 32]) {
    let (status, body) = if request.url() != "/v1/download-grant" {
        (404, serde_json::json!({"error": "not found"}))
    } else if request.method() != &Method::Post {
        (405, serde_json::json!({"error": "method not allowed"}))
    } else {
        let mut body = Vec::new();
        let read = request
            .as_reader()
            .take(MAX_BODY + 1)
            .read_to_end(&mut body);
        if read.is_err() || body.len() as u64 > MAX_BODY {
            (400, serde_json::json!({"error": "invalid request"}))
        } else if let Ok(login) = serde_json::from_slice::<Login>(&body) {
            issue_login(db, secret, login)
        } else {
            (400, serde_json::json!({"error": "invalid request"}))
        }
    };
    let response = Response::from_string(body.to_string())
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").expect("valid header"));
    if let Err(err) = request.respond(response) {
        bevy::log::warn!("Download grant response failed: {err}");
    }
}

fn issue_login(db: &Database, secret: &[u8; 32], login: Login) -> (u16, serde_json::Value) {
    let (token, username, password) = match &login {
        Login::Token { token } => (Some(token.as_str()), "", ""),
        Login::Credentials { username, password } => (None, username.as_str(), password.as_str()),
    };
    let Ok((account, token)) =
        crate::persistence_accounts::validate_login_credentials(db, token, username, password)
    else {
        return (401, serde_json::json!({"error": "invalid credentials"}));
    };
    let token = token
        .unwrap_or_else(|| crate::persistence_accounts::generate_token_for_account(db, account));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs();
    let expires_at = now + TTL_SECONDS;
    let grant = issue_grant(
        secret,
        &GrantPayload {
            a: account,
            e: expires_at,
        },
    );
    (
        200,
        serde_json::json!({"grant": grant, "expires_at": expires_at, "token": token}),
    )
}

#[cfg(test)]
mod tests;
