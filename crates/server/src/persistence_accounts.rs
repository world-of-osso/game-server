use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use redb::{Database, ReadableDatabase, ReadableTable};
use std::collections::HashMap;

use super::persistence::{
    ACCOUNTS, BANNED_ACCOUNTS, PASSWORDS, PENDING_REGISTRATIONS, USERNAME_ACCOUNTS,
};
use crate::persistence::next_id;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    pub username: String,
    pub account_id: u64,
    pub token_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInspection {
    pub username: String,
    pub exists: bool,
    pub account_id: Option<u64>,
    pub token_count: usize,
    pub pending_registration: bool,
    pub banned: bool,
    pub ban_reason: Option<String>,
}

/// Synchronous fixture API; runtime password tokens use the FIFO receipt.
#[cfg(test)]
pub fn login_with_credentials(
    db: &Database,
    token: Option<&str>,
    username: &str,
    password: &str,
) -> Result<(u64, String), String> {
    let (account, token) = validate_login_credentials(db, token, username, password)?;
    Ok((
        account,
        token.unwrap_or_else(|| generate_token_for_account(db, account)),
    ))
}

/// Read-only credential validation. None means a password login needs a durable token.
pub(crate) fn validate_login_credentials(
    db: &Database,
    token: Option<&str>,
    username: &str,
    password: &str,
) -> Result<(u64, Option<String>), String> {
    if let Some(tok) = token
        && !tok.is_empty()
        && let Some(account_id) = find_account_by_token(db, tok)
    {
        ensure_account_not_banned(db, account_id)?;
        return Ok((account_id, Some(tok.to_string())));
    }
    let account_id = find_account_by_username(db, username).ok_or_else(|| {
        if is_username_pending(db, username) {
            "Account pending admin approval".to_string()
        } else {
            "Account not found — please register first".to_string()
        }
    })?;
    ensure_account_not_banned(db, account_id)?;
    verify_account_password(db, account_id, password)?;
    Ok((account_id, None))
}

pub(crate) fn ensure_account_not_banned(db: &Database, account_id: u64) -> Result<(), String> {
    if let Some(reason) = ban_reason_for_account(db, account_id) {
        Err(format!("Account banned: {reason}"))
    } else {
        Ok(())
    }
}

/// Look up account_id by username.
pub(crate) fn find_account_by_username(db: &Database, username: &str) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(USERNAME_ACCOUNTS)
        .expect("Failed to open username_accounts");
    table
        .get(username)
        .expect("Failed to read username")
        .map(|g| g.value())
}

/// Verify password against stored argon2 hash.
fn verify_account_password(db: &Database, account_id: u64, password: &str) -> Result<(), String> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(PASSWORDS).expect("Failed to open passwords");
    let hash_bytes = table
        .get(account_id)
        .expect("Failed to read password hash")
        .ok_or_else(|| "No password set for account".to_string())?;
    let hash_str =
        std::str::from_utf8(hash_bytes.value()).map_err(|_| "Corrupt password hash".to_string())?;
    let parsed = PasswordHash::new(hash_str).map_err(|_| "Corrupt password hash".to_string())?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| "Invalid password".to_string())
}

pub fn check_password(db: &Database, username: &str, password: &str) -> Result<bool, String> {
    let Some(account_id) = find_account_by_username(db, username) else {
        return Ok(false);
    };
    match verify_account_password(db, account_id, password) {
        Ok(()) => Ok(true),
        Err(err) if err == "Invalid password" => Ok(false),
        Err(err) => Err(err),
    }
}

/// Generate and store a new token for an existing account.
pub(crate) fn generate_token_for_account(db: &Database, account_id: u64) -> String {
    let token = uuid::Uuid::new_v4().to_string();
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn.open_table(ACCOUNTS).expect("Failed to open accounts");
        table
            .insert(token.as_str(), account_id)
            .expect("Failed to insert token");
    }
    txn.commit().expect("Failed to commit token");
    token
}

/// Register a new account with username and password. Returns (account_id, token).
pub fn register_account(
    db: &Database,
    username: &str,
    password: &str,
) -> Result<(u64, String), String> {
    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if password.is_empty() {
        return Err("Password cannot be empty".to_string());
    }
    if find_account_by_username(db, username).is_some() {
        return Err("Username already taken".to_string());
    }
    if is_username_pending(db, username) {
        return Err("Username has a pending registration".to_string());
    }
    let hash = hash_password(password)?;
    let account_id = next_id(db, "account");
    let token = uuid::Uuid::new_v4().to_string();
    store_new_account(db, account_id, username, &hash, &token);
    Ok((account_id, token))
}

/// Store a registration request for later admin approval.
pub fn request_registration(db: &Database, username: &str, password: &str) -> Result<(), String> {
    if find_account_by_username(db, username).is_some() {
        return Err("Username already taken".to_string());
    }
    if is_username_pending(db, username) {
        return Err("Registration already pending approval".to_string());
    }
    let hash = hash_password(password)?;
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut pending = txn
            .open_table(PENDING_REGISTRATIONS)
            .expect("open pending_registrations");
        pending
            .insert(username, hash.as_bytes())
            .expect("insert pending registration");
    }
    txn.commit().expect("Failed to commit pending registration");
    Ok(())
}

/// Approve a pending registration and create an active account.
pub fn approve_registration(db: &Database, username: &str) -> Result<(u64, String), String> {
    if find_account_by_username(db, username).is_some() {
        return Err("Username already approved".to_string());
    }
    let hash = load_pending_hash(db, username)
        .ok_or_else(|| "No pending registration for username".to_string())?;
    let account_id = next_id(db, "account");
    let token = uuid::Uuid::new_v4().to_string();
    write_approved_account(db, username, account_id, &token, &hash);
    Ok((account_id, token))
}

pub fn ban_account(db: &Database, username: &str, reason: &str) -> Result<u64, String> {
    let Some(account_id) = find_account_by_username(db, username) else {
        return Err(format!("account '{username}' not found"));
    };
    let reason = reason.trim();
    if reason.is_empty() {
        return Err("Ban reason cannot be empty".to_string());
    }
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(BANNED_ACCOUNTS)
            .expect("open banned_accounts");
        table
            .insert(account_id, reason.as_bytes())
            .expect("insert ban reason");
    }
    txn.commit().expect("Failed to commit account ban");
    Ok(account_id)
}

pub fn unban_account(db: &Database, username: &str) -> Result<u64, String> {
    let Some(account_id) = find_account_by_username(db, username) else {
        return Err(format!("account '{username}' not found"));
    };
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(BANNED_ACCOUNTS)
            .expect("open banned_accounts");
        table.remove(account_id).expect("remove ban reason");
    }
    txn.commit().expect("Failed to commit account unban");
    Ok(account_id)
}

pub fn ban_reason_for_account(db: &Database, account_id: u64) -> Option<String> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(BANNED_ACCOUNTS)
        .expect("Failed to open banned_accounts");
    table
        .get(account_id)
        .expect("Failed to read banned account")
        .map(|reason| {
            String::from_utf8(reason.value().to_vec()).expect("ban reason should be valid UTF-8")
        })
}

fn write_approved_account(db: &Database, username: &str, account_id: u64, token: &str, hash: &str) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut accounts = txn.open_table(ACCOUNTS).expect("open accounts");
        accounts
            .insert(token, account_id)
            .expect("insert account token");
        let mut usernames = txn
            .open_table(USERNAME_ACCOUNTS)
            .expect("open username_accounts");
        usernames
            .insert(username, account_id)
            .expect("insert username mapping");
        let mut passwords = txn.open_table(PASSWORDS).expect("open passwords");
        passwords
            .insert(account_id, hash.as_bytes())
            .expect("insert password hash");
        let mut pending = txn
            .open_table(PENDING_REGISTRATIONS)
            .expect("open pending_registrations");
        pending
            .remove(username)
            .expect("remove pending registration");
    }
    txn.commit().expect("Failed to commit account approval");
}

/// Hash a password with argon2.
pub(crate) fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Failed to hash password: {e}"))
}

/// Store account, username mapping, password hash, and token in one transaction.
fn store_new_account(db: &Database, account_id: u64, username: &str, hash: &str, token: &str) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut accounts = txn.open_table(ACCOUNTS).expect("open accounts");
        accounts
            .insert(token, account_id)
            .expect("insert account token");
        let mut usernames = txn
            .open_table(USERNAME_ACCOUNTS)
            .expect("open username_accounts");
        usernames
            .insert(username, account_id)
            .expect("insert username mapping");
        let mut passwords = txn.open_table(PASSWORDS).expect("open passwords");
        passwords
            .insert(account_id, hash.as_bytes())
            .expect("insert password hash");
    }
    txn.commit().expect("Failed to commit account creation");
}

/// Find an existing account by token.
pub(crate) fn find_account_by_token(db: &Database, token: &str) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(ACCOUNTS).expect("Failed to open accounts");
    table
        .get(token)
        .expect("Failed to read account")
        .map(|g| g.value())
}

pub fn list_accounts(db: &Database) -> Vec<AccountSummary> {
    let token_counts = token_counts_by_account(db);
    let txn = db.begin_read().expect("Failed to begin read txn");
    let usernames = txn
        .open_table(USERNAME_ACCOUNTS)
        .expect("Failed to open username_accounts");

    let mut accounts: Vec<_> = usernames
        .iter()
        .expect("Failed to iterate username_accounts")
        .map(|row| {
            let (username, account_id) = row.expect("Failed to read username_accounts row");
            let username = username.value().to_string();
            let account_id = account_id.value();
            AccountSummary {
                username,
                account_id,
                token_count: token_counts.get(&account_id).copied().unwrap_or(0),
            }
        })
        .collect();
    accounts.sort_by(|a, b| a.username.cmp(&b.username));
    accounts
}

pub fn inspect_account(db: &Database, username: &str) -> AccountInspection {
    let pending_registration = is_username_pending(db, username);
    let Some(account_id) = find_account_by_username(db, username) else {
        return AccountInspection {
            username: username.to_string(),
            exists: false,
            account_id: None,
            token_count: 0,
            pending_registration,
            banned: false,
            ban_reason: None,
        };
    };
    let ban_reason = ban_reason_for_account(db, account_id);

    AccountInspection {
        username: username.to_string(),
        exists: true,
        account_id: Some(account_id),
        token_count: token_counts_by_account(db)
            .get(&account_id)
            .copied()
            .unwrap_or(0),
        pending_registration,
        banned: ban_reason.is_some(),
        ban_reason,
    }
}

fn token_counts_by_account(db: &Database) -> HashMap<u64, usize> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let accounts = txn.open_table(ACCOUNTS).expect("Failed to open accounts");
    let mut counts = HashMap::new();
    for row in accounts.iter().expect("Failed to iterate accounts") {
        let (_token, account_id) = row.expect("Failed to read account row");
        *counts.entry(account_id.value()).or_insert(0) += 1;
    }
    counts
}

fn is_username_pending(db: &Database, username: &str) -> bool {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(PENDING_REGISTRATIONS)
        .expect("Failed to open pending_registrations");
    table
        .get(username)
        .expect("Failed to read pending username")
        .is_some()
}

fn load_pending_hash(db: &Database, username: &str) -> Option<String> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(PENDING_REGISTRATIONS)
        .expect("Failed to open pending_registrations");
    let hash = table
        .get(username)
        .expect("Failed to read pending registration")?;
    std::str::from_utf8(hash.value()).ok().map(str::to_owned)
}

/// Link username and password to an existing token-based account.
pub fn set_account_credentials(
    db: &Database,
    token: &str,
    username: &str,
    password: &str,
) -> Result<u64, String> {
    let account_id =
        find_account_by_token(db, token).ok_or_else(|| "Token not found".to_string())?;
    if find_account_by_username(db, username).is_some() {
        return Err("Username already taken".to_string());
    }
    let hash = hash_password(password)?;
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut usernames = txn
            .open_table(USERNAME_ACCOUNTS)
            .expect("open username_accounts");
        usernames
            .insert(username, account_id)
            .expect("insert username");
        let mut passwords = txn.open_table(PASSWORDS).expect("open passwords");
        passwords
            .insert(account_id, hash.as_bytes())
            .expect("insert password");
    }
    txn.commit().expect("Failed to commit credentials");
    Ok(account_id)
}

#[cfg(test)]
#[path = "persistence_accounts_tests.rs"]
mod tests;
