use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use bevy::prelude::*;
use rand::rngs::OsRng;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use shared::components::{Health, Player, Position};
use std::path::Path;

/// Character data table: character_id (u64) -> serialized CharacterData (bytes).
const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

/// Inventory table: PlayerId (u64) -> serialized inventory (bytes).
const INVENTORIES: TableDefinition<u64, &[u8]> = TableDefinition::new("inventories");

/// Account table: token (&str) -> account_id (u64).
const ACCOUNTS: TableDefinition<&str, u64> = TableDefinition::new("accounts");

/// Account characters: account_id (u64) -> bitcode-encoded Vec<u64> of character_ids.
const ACCOUNT_CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("account_characters");

/// Character name uniqueness: name (&str) -> character_id (u64).
const CHARACTER_NAMES: TableDefinition<&str, u64> = TableDefinition::new("character_names");

/// Password hashes: account_id (u64) -> argon2 hash string (bytes).
const PASSWORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("passwords");

/// Username to account_id mapping: username (&str) -> account_id (u64).
const USERNAME_ACCOUNTS: TableDefinition<&str, u64> = TableDefinition::new("username_accounts");

/// Auto-increment counters: key (&str) -> next_id (u64).
const NEXT_IDS: TableDefinition<&str, u64> = TableDefinition::new("next_ids");

/// Serializable character data for persistence.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct CharacterData {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32,
    pub max_health: f32,
    pub race: u8,
    pub class: u8,
    pub level: u16,
}

impl CharacterData {
    pub fn from_components(player: &Player, pos: &Position, health: &Health) -> Self {
        Self {
            name: player.name.clone(),
            x: pos.x,
            y: pos.y,
            z: pos.z,
            health: health.current,
            max_health: health.max,
            race: 1,
            class: 1,
            level: 1,
        }
    }
}

/// Persistence plugin — manages redb database lifecycle and periodic saves.
pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, open_database);
        app.add_systems(FixedUpdate, save_dirty_characters);
    }
}

#[derive(Resource)]
pub struct GameDatabase {
    pub db: Database,
}

fn open_database(mut commands: Commands) {
    let path = Path::new("data/game.redb");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create data directory");
    }
    let db = Database::create(path).expect("Failed to open database");
    create_tables(&db);
    info!("Database opened at {}", path.display());
    commands.insert_resource(GameDatabase { db });
}

fn create_tables(db: &Database) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    let _ = txn.open_table(CHARACTERS);
    let _ = txn.open_table(INVENTORIES);
    let _ = txn.open_table(ACCOUNTS);
    let _ = txn.open_table(ACCOUNT_CHARACTERS);
    let _ = txn.open_table(CHARACTER_NAMES);
    let _ = txn.open_table(PASSWORDS);
    let _ = txn.open_table(USERNAME_ACCOUNTS);
    let _ = txn.open_table(NEXT_IDS);
    txn.commit().expect("Failed to commit initial tables");
}

/// Save a single character to the database.
pub fn save_character(db: &Database, character_id: u64, data: &CharacterData) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTERS)
            .expect("Failed to open characters table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert character");
    }
    txn.commit().expect("Failed to commit character save");
}

/// Load a character from the database, if one exists for this character_id.
pub fn load_character(db: &Database, character_id: u64) -> Option<CharacterData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTERS)
        .expect("Failed to open characters table");
    match table.get(character_id).expect("Failed to read character") {
        Some(guard) => Some(bitcode::decode::<CharacterData>(guard.value()).expect("Failed to decode")),
        None => None,
    }
}

/// Atomic increment for NEXT_IDS table. Returns the new ID (starts at 1).
pub fn next_id(db: &Database, key: &str) -> u64 {
    let txn = db.begin_write().expect("Failed to begin write txn");
    let id;
    {
        let mut table = txn.open_table(NEXT_IDS).expect("Failed to open next_ids");
        let prev = match table.get(key).expect("Failed to read next_id") {
            Some(guard) => guard.value(),
            None => 0,
        };
        id = prev + 1;
        table.insert(key, id).expect("Failed to insert next_id");
    }
    txn.commit().expect("Failed to commit next_id");
    id
}

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
    let account_id = find_account_by_username(db, username)
        .ok_or_else(|| "Account not found — please register first".to_string())?;
    verify_account_password(db, account_id, password)?;
    let token = generate_token_for_account(db, account_id);
    Ok((account_id, token))
}

/// Look up account_id by username.
fn find_account_by_username(db: &Database, username: &str) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(USERNAME_ACCOUNTS)
        .expect("Failed to open username_accounts");
    match table.get(username).expect("Failed to read username") {
        Some(guard) => Some(guard.value()),
        None => None,
    }
}

/// Verify password against stored argon2 hash.
fn verify_account_password(db: &Database, account_id: u64, password: &str) -> Result<(), String> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(PASSWORDS)
        .expect("Failed to open passwords");
    let hash_bytes = table
        .get(account_id)
        .expect("Failed to read password hash")
        .ok_or_else(|| "No password set for account".to_string())?;
    let hash_str = std::str::from_utf8(hash_bytes.value())
        .map_err(|_| "Corrupt password hash".to_string())?;
    let parsed = PasswordHash::new(hash_str)
        .map_err(|_| "Corrupt password hash".to_string())?;
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
    let hash = hash_password(password)?;
    let account_id = next_id(db, "account");
    let token = uuid::Uuid::new_v4().to_string();
    store_new_account(db, account_id, username, &hash, &token);
    Ok((account_id, token))
}

/// Hash a password with argon2.
fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Failed to hash password: {e}"))
}

/// Store account, username mapping, password hash, and token in one transaction.
fn store_new_account(
    db: &Database,
    account_id: u64,
    username: &str,
    hash: &str,
    token: &str,
) {
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
fn find_account_by_token(db: &Database, token: &str) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(ACCOUNTS).expect("Failed to open accounts");
    match table.get(token).expect("Failed to read account") {
        Some(guard) => Some(guard.value()),
        None => None,
    }
}


/// List all characters for an account.
pub fn list_characters_for_account(db: &Database, account_id: u64) -> Vec<(u64, CharacterData)> {
    let char_ids = load_account_char_ids(db, account_id);
    let mut result = Vec::new();
    for id in char_ids {
        if let Some(data) = load_character(db, id) {
            result.push((id, data));
        }
    }
    result
}

/// Load the character ID list for an account.
fn load_account_char_ids(db: &Database, account_id: u64) -> Vec<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(ACCOUNT_CHARACTERS)
        .expect("Failed to open account_characters");
    match table.get(account_id).expect("Failed to read account chars") {
        Some(guard) => bitcode::decode::<Vec<u64>>(guard.value()).expect("Failed to decode char ids"),
        None => Vec::new(),
    }
}

/// Create a new character for an account. Returns the character or an error.
pub fn create_character(
    db: &Database,
    account_id: u64,
    name: &str,
    race: u8,
    class: u8,
) -> Result<(u64, CharacterData), String> {
    check_name_available(db, name)?;
    let character_id = next_id(db, "character");
    let data = default_character_data(name, race, class);
    save_new_character(db, account_id, character_id, name, &data);
    Ok((character_id, data))
}

/// Check that a character name is not already taken.
fn check_name_available(db: &Database, name: &str) -> Result<(), String> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTER_NAMES)
        .expect("Failed to open character_names");
    if table.get(name).expect("Failed to read name").is_some() {
        return Err(format!("Name '{name}' is already taken"));
    }
    Ok(())
}

/// Build default CharacterData for a new character at Northshire spawn.
fn default_character_data(name: &str, race: u8, class: u8) -> CharacterData {
    CharacterData {
        name: name.to_string(),
        x: -8949.0,
        y: -132.0,
        z: 83.0,
        health: 100.0,
        max_health: 100.0,
        race,
        class,
        level: 1,
    }
}

/// Persist a new character across all tables in a single transaction.
fn save_new_character(
    db: &Database,
    account_id: u64,
    character_id: u64,
    name: &str,
    data: &CharacterData,
) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut chars = txn.open_table(CHARACTERS).expect("open characters");
        chars
            .insert(character_id, bytes.as_slice())
            .expect("insert character");
        let mut names = txn.open_table(CHARACTER_NAMES).expect("open names");
        names
            .insert(name, character_id)
            .expect("insert character name");
        let mut acct = txn.open_table(ACCOUNT_CHARACTERS).expect("open acct chars");
        let mut ids = read_char_ids_from_table(&acct, account_id);
        ids.push(character_id);
        let encoded = bitcode::encode(&ids);
        acct.insert(account_id, encoded.as_slice())
            .expect("insert acct chars");
    }
    txn.commit().expect("Failed to commit new character");
}

/// Read character IDs from an open account_characters table.
fn read_char_ids_from_table(
    table: &redb::Table<u64, &[u8]>,
    account_id: u64,
) -> Vec<u64> {
    match table.get(account_id).expect("read acct chars") {
        Some(guard) => bitcode::decode::<Vec<u64>>(guard.value()).expect("decode char ids"),
        None => Vec::new(),
    }
}

/// Delete a character, validating account ownership first.
pub fn delete_character(
    db: &Database,
    account_id: u64,
    character_id: u64,
) -> Result<(), String> {
    let char_ids = load_account_char_ids(db, account_id);
    if !char_ids.contains(&character_id) {
        return Err("Character does not belong to this account".into());
    }
    let name = load_character(db, character_id)
        .map(|d| d.name)
        .ok_or("Character not found")?;
    remove_character(db, account_id, character_id, &name, &char_ids);
    Ok(())
}

/// Remove a character from all tables in a single transaction.
fn remove_character(
    db: &Database,
    account_id: u64,
    character_id: u64,
    name: &str,
    char_ids: &[u64],
) {
    let remaining: Vec<u64> = char_ids.iter().copied().filter(|&id| id != character_id).collect();
    let encoded = bitcode::encode(&remaining);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut chars = txn.open_table(CHARACTERS).expect("open characters");
        chars.remove(character_id).expect("remove character");
        let mut names = txn.open_table(CHARACTER_NAMES).expect("open names");
        names.remove(name).expect("remove character name");
        let mut acct = txn.open_table(ACCOUNT_CHARACTERS).expect("open acct chars");
        acct.insert(account_id, encoded.as_slice())
            .expect("update acct chars");
    }
    txn.commit().expect("Failed to commit character deletion");
}

/// Marker component linking a player entity to its character_id for periodic saves.
#[derive(Component, Debug)]
pub struct ClientId(pub u64);

/// Save interval in ticks (300 ticks at 20Hz = 15 seconds).
const SAVE_INTERVAL_TICKS: u64 = 300;

/// Periodically save all player data to the database.
fn save_dirty_characters(
    db: Option<Res<GameDatabase>>,
    players: Query<(&Position, &Health, &Player, &ClientId)>,
    mut tick_count: Local<u64>,
) {
    *tick_count += 1;
    if *tick_count % SAVE_INTERVAL_TICKS != 0 {
        return;
    }
    let Some(db) = db else { return };
    for (pos, health, player, cid) in players.iter() {
        let data = CharacterData::from_components(player, pos, health);
        save_character(&db.db, cid.0, &data);
    }
    info!(
        "Periodic save: {} characters written",
        players.iter().count()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_database() -> Database {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.redb");
        let db = Database::create(&path).unwrap();
        create_tables(&db);
        std::mem::forget(dir);
        db
    }

    fn sample_data() -> CharacterData {
        CharacterData {
            name: "TestPlayer".into(),
            x: -8949.0,
            y: -132.0,
            z: 83.0,
            health: 75.0,
            max_health: 100.0,
            race: 1,
            class: 1,
            level: 1,
        }
    }

    #[test]
    fn character_data_round_trip() {
        let db = temp_database();
        let data = sample_data();
        save_character(&db, 42, &data);

        let loaded = load_character(&db, 42).expect("should find saved character");
        assert_eq!(loaded, data);
    }

    #[test]
    fn load_missing_character_returns_none() {
        let db = temp_database();
        assert!(load_character(&db, 999).is_none());
    }

    #[test]
    fn save_overwrites_existing() {
        let db = temp_database();
        let mut data = sample_data();
        save_character(&db, 1, &data);

        data.x = 100.0;
        data.health = 50.0;
        save_character(&db, 1, &data);

        let loaded = load_character(&db, 1).unwrap();
        assert_eq!(loaded.x, 100.0);
        assert_eq!(loaded.health, 50.0);
    }

    #[test]
    fn register_and_login_by_token() {
        let db = temp_database();
        let (id1, token) = register_account(&db, "user1", "pass123").unwrap();
        assert!(id1 > 0);

        let (id2, token2) =
            login_with_credentials(&db, Some(&token), "user1", "").unwrap();
        assert_eq!(id1, id2);
        assert_eq!(token, token2);
    }

    #[test]
    fn login_with_password() {
        let db = temp_database();
        let (id1, _) = register_account(&db, "user1", "secret").unwrap();

        let (id2, _) =
            login_with_credentials(&db, None, "user1", "secret").unwrap();
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
    fn create_and_list_characters() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();

        let (cid, data) = create_character(&db, account_id, "Hero", 1, 2).unwrap();
        assert_eq!(data.name, "Hero");
        assert_eq!(data.race, 1);
        assert_eq!(data.class, 2);

        let list = list_characters_for_account(&db, account_id);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, cid);
        assert_eq!(list[0].1.name, "Hero");
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
    fn delete_character_removes_from_all_tables() {
        let db = temp_database();
        let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
        let (cid, _) = create_character(&db, account_id, "ToDelete", 1, 1).unwrap();

        delete_character(&db, account_id, cid).unwrap();

        assert!(load_character(&db, cid).is_none());
        let list = list_characters_for_account(&db, account_id);
        assert!(list.is_empty());
        // Name should be available again
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
}
