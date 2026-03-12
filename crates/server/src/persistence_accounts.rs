use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;
use redb::{Database, ReadableDatabase};

use super::persistence::{ACCOUNTS, PASSWORDS, PENDING_REGISTRATIONS, USERNAME_ACCOUNTS};
use crate::persistence::next_id;

/// Authenticate via token or username+password. Returns (account_id, token) or error.
pub fn login_with_credentials(
    db: &Database,
    token: Option<&str>,
    username: &str,
    password: &str,
) -> Result<(u64, String), String> {
    if let Some(tok) = token {
        if !tok.is_empty() {
            if let Some(account_id) = find_account_by_token(db, tok) {
                return Ok((account_id, tok.to_string()));
            }
        }
    }
    let account_id = find_account_by_username(db, username).ok_or_else(|| {
        if is_username_pending(db, username) {
            "Account pending admin approval".to_string()
        } else {
            "Account not found — please register first".to_string()
        }
    })?;
    verify_account_password(db, account_id, password)?;
    let token = generate_token_for_account(db, account_id);
    Ok((account_id, token))
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

/// Generate and store a new token for an existing account.
fn generate_token_for_account(db: &Database, account_id: u64) -> String {
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
mod tests {
    use super::*;
    use crate::persistence::{
        create_character, delete_character, list_characters_for_account, load_character,
        load_character_guild, save_character, set_character_guild_by_name,
        set_pending_guild_invite, take_pending_guild_invite, temp_database,
    };

    #[test]
    fn register_and_login_by_token() {
        let db = temp_database();
        let (id1, token) = register_account(&db, "user1", "pass123").unwrap();
        assert!(id1 > 0);

        let (id2, token2) = login_with_credentials(&db, Some(&token), "user1", "").unwrap();
        assert_eq!(id1, id2);
        assert_eq!(token, token2);
    }

    #[test]
    fn login_with_password() {
        let db = temp_database();
        let (id1, _) = register_account(&db, "user1", "secret").unwrap();

        let (id2, _) = login_with_credentials(&db, None, "user1", "secret").unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn login_wrong_password_fails() {
        let db = temp_database();
        register_account(&db, "user1", "correct").unwrap();

        let result = login_with_credentials(&db, None, "user1", "wrong");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid password"));
    }

    #[test]
    fn login_unknown_user_fails() {
        let db = temp_database();
        let result = login_with_credentials(&db, None, "nobody", "pass");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn register_duplicate_username_fails() {
        let db = temp_database();
        register_account(&db, "user1", "pass").unwrap();
        let result = register_account(&db, "user1", "other");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already taken"));
    }

    #[test]
    fn pending_registration_requires_approval_before_login() {
        let db = temp_database();
        request_registration(&db, "user1", "secret").unwrap();

        let result = login_with_credentials(&db, None, "user1", "secret");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("pending admin approval"));
    }

    #[test]
    fn approve_registration_activates_account() {
        let db = temp_database();
        request_registration(&db, "user1", "secret").unwrap();

        let (account_id, token) = approve_registration(&db, "user1").unwrap();
        assert!(account_id > 0);
        assert!(!token.is_empty());

        let (login_id, _new_token) = login_with_credentials(&db, None, "user1", "secret").unwrap();
        assert_eq!(account_id, login_id);
    }

    #[test]
    fn set_account_credentials_links_username() {
        let db = temp_database();
        let (account_id, token) = register_account(&db, "user1", "pass").unwrap();
        // Link a second username to same account via token (simulates token-only account getting credentials)
        // First create a fresh token-only account by manually inserting
        let token2 = uuid::Uuid::new_v4().to_string();
        let txn = db.begin_write().unwrap();
        {
            let mut accounts = txn.open_table(ACCOUNTS).unwrap();
            accounts.insert(token2.as_str(), account_id + 100).unwrap();
        }
        txn.commit().unwrap();

        let result = set_account_credentials(&db, &token2, "newuser", "newpass");
        assert_eq!(result.unwrap(), account_id + 100);

        let found = find_account_by_username(&db, "newuser");
        assert_eq!(found, Some(account_id + 100));
    }

    #[test]
    fn set_account_credentials_rejects_taken_username() {
        let db = temp_database();
        let (_, token) = register_account(&db, "user1", "pass").unwrap();
        let result = set_account_credentials(&db, &token, "user1", "pass");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already taken"));
    }

    #[test]
    fn set_account_credentials_rejects_bad_token() {
        let db = temp_database();
        let result = set_account_credentials(&db, "bad-token", "user1", "pass");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Token not found"));
    }

    #[test]
    fn create_and_list_characters() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
        let (cid, data) = create_character(&db, account_id, "Hero", 1, 2).unwrap();
        assert_eq!(data.name, "Hero");
        let list = list_characters_for_account(&db, account_id);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, cid);
    }

    #[test]
    fn create_character_rejects_duplicate_name() {
        let db = temp_database();
        let (acct1, _) = register_account(&db, "user1", "pass").unwrap();
        let (acct2, _) = register_account(&db, "user2", "pass").unwrap();
        create_character(&db, acct1, "UniqueHero", 1, 1).unwrap();
        let result = create_character(&db, acct2, "UniqueHero", 2, 2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already taken"));
    }

    #[test]
    fn create_character_rejects_invalid_race() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
        let result = create_character(&db, account_id, "BadRace", 255, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid race id"));
    }

    #[test]
    fn delete_character_removes_from_all_tables() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
        let (cid, _) = create_character(&db, account_id, "ToDelete", 1, 1).unwrap();
        delete_character(&db, account_id, cid).unwrap();
        assert!(crate::persistence::load_character(&db, cid).is_none());
        assert!(list_characters_for_account(&db, account_id).is_empty());
        assert!(create_character(&db, account_id, "ToDelete", 1, 1).is_ok());
    }

    #[test]
    fn delete_character_rejects_wrong_account() {
        let db = temp_database();
        let (acct1, _) = register_account(&db, "user1", "pass").unwrap();
        let (acct2, _) = register_account(&db, "user2", "pass").unwrap();
        let (cid, _) = create_character(&db, acct1, "MyChar", 1, 1).unwrap();
        let result = delete_character(&db, acct2, cid);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not belong"));
    }

    #[test]
    fn character_guild_membership_round_trip() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
        let (character_id, _) = create_character(&db, account_id, "Guildy", 1, 1).unwrap();
        set_character_guild_by_name(&db, "Guildy", 42, "Forty Two").unwrap();
        let guild = load_character_guild(&db, character_id).expect("guild should exist");
        assert_eq!(guild.guild_id, 42);
        assert_eq!(guild.guild_name, "Forty Two");
    }

    #[test]
    fn pending_guild_invite_is_consumed_once() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
        let (character_id, _) = create_character(&db, account_id, "Invitee", 1, 1).unwrap();
        set_pending_guild_invite(&db, character_id, 7, "Lucky Seven", "Inviter");
        let invite = take_pending_guild_invite(&db, character_id).expect("invite should exist");
        assert_eq!(invite.guild_id, 7);
        assert_eq!(invite.guild_name, "Lucky Seven");
        assert_eq!(invite.inviter_name, "Inviter");
        assert!(take_pending_guild_invite(&db, character_id).is_none());
    }

    #[test]
    fn set_level_persists_via_save_and_load() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
        let (cid, _) = create_character(&db, account_id, "Leveler", 1, 1).unwrap();
        let mut data = load_character(&db, cid).unwrap();
        assert_eq!(data.level, 1);
        data.level = 42;
        save_character(&db, cid, &data);
        let reloaded = load_character(&db, cid).unwrap();
        assert_eq!(reloaded.level, 42);
    }

    #[test]
    fn set_credentials_enables_password_login() {
        let db = temp_database();
        let (_, _token) = register_account(&db, "tokenuser", "oldpass").unwrap();
        let (_, token2) = register_account(&db, "other", "pass").unwrap();
        // token2's account has no username "aliasuser" yet
        set_account_credentials(&db, &token2, "aliasuser", "newpass").unwrap();
        let (account_id, _) = login_with_credentials(&db, None, "aliasuser", "newpass").unwrap();
        let (other_id, _) = login_with_credentials(&db, Some(&token2), "", "").unwrap();
        assert_eq!(account_id, other_id);
    }
}
