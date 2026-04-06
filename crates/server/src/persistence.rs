use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use shared::components::{Health, Player, Position};
use std::path::Path;

pub use crate::persistence_character::*;
pub use crate::persistence_types::*;

/// Character data table: character_id (u64) -> serialized CharacterData (bytes).
pub(crate) const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

/// Inventory table: PlayerId (u64) -> serialized inventory (bytes).
const INVENTORIES: TableDefinition<u64, &[u8]> = TableDefinition::new("inventories");

/// Active auction table: auction_id (u64) -> serialized auction data (bytes).
const AUCTIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("auctions");

/// Mailbox table: character_id (u64) -> serialized Vec<MailEntryData>.
const MAILBOXES: TableDefinition<u64, &[u8]> = TableDefinition::new("mailboxes");

/// Account table: token (&str) -> account_id (u64).
pub(crate) const ACCOUNTS: TableDefinition<&str, u64> = TableDefinition::new("accounts");

/// Account characters: account_id (u64) -> bitcode-encoded Vec<u64> of character_ids.
pub(crate) const ACCOUNT_CHARACTERS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("account_characters");

/// Character name uniqueness: name (&str) -> character_id (u64).
pub(crate) const CHARACTER_NAMES: TableDefinition<&str, u64> =
    TableDefinition::new("character_names");

/// Character guild membership: character_id (u64) -> serialized GuildMembershipData.
pub(crate) const CHARACTER_GUILDS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_guilds");

/// Pending guild invites: character_id (u64) -> serialized PendingGuildInviteData.
pub(crate) const PENDING_GUILD_INVITES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("pending_guild_invites");

/// Password hashes: account_id (u64) -> argon2 hash string (bytes).
pub(crate) const PASSWORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("passwords");

/// Username to account_id mapping: username (&str) -> account_id (u64).
pub(crate) const USERNAME_ACCOUNTS: TableDefinition<&str, u64> =
    TableDefinition::new("username_accounts");

/// Pending registrations: username (&str) -> argon2 hash string (bytes).
pub(crate) const PENDING_REGISTRATIONS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("pending_registrations");

/// Auto-increment counters: key (&str) -> next_id (u64).
pub(crate) const NEXT_IDS: TableDefinition<&str, u64> = TableDefinition::new("next_ids");

/// Persistence plugin — manages redb database lifecycle and periodic saves.
pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, open_database);
        app.add_systems(FixedUpdate, save_dirty_characters);
        app.add_systems(Last, flush_all_characters_on_exit);
    }
}

#[derive(Resource)]
pub struct GameDatabase {
    pub db: Database,
}

fn try_open_database_once(path: &Path) -> Result<Database, redb::DatabaseError> {
    Database::create(path)
}

fn open_database_with_retry(path: &Path) -> Database {
    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY_MS: u64 = 1000;

    for attempt in 1..=MAX_RETRIES {
        match try_open_database_once(path) {
            Ok(db) => return db,
            Err(redb::DatabaseError::DatabaseAlreadyOpen) => {
                error!(
                    "{path:?} is locked by another process (attempt {attempt}/{MAX_RETRIES}). \
                     Retrying in {RETRY_DELAY_MS}ms..."
                );
                std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
            }
            Err(e) => {
                error!("Failed to open database {path:?}: {e}");
                std::process::exit(1);
            }
        }
    }
    error!(
        "Failed to open {path:?} after {MAX_RETRIES} attempts. \
         Is another server or admin CLI running?"
    );
    std::process::exit(1);
}

fn open_database(mut commands: Commands) {
    let path = Path::new("data/game.redb");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create data directory");
    }
    let db = open_database_with_retry(path);
    create_tables(&db);
    info!("Database opened at {}", path.display());
    commands.insert_resource(GameDatabase { db });
}

pub(crate) fn create_tables(db: &Database) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    let _ = txn.open_table(CHARACTERS);
    let _ = txn.open_table(INVENTORIES);
    let _ = txn.open_table(AUCTIONS);
    let _ = txn.open_table(MAILBOXES);
    let _ = txn.open_table(ACCOUNTS);
    let _ = txn.open_table(ACCOUNT_CHARACTERS);
    let _ = txn.open_table(CHARACTER_NAMES);
    let _ = txn.open_table(CHARACTER_GUILDS);
    let _ = txn.open_table(PENDING_GUILD_INVITES);
    let _ = txn.open_table(PASSWORDS);
    let _ = txn.open_table(USERNAME_ACCOUNTS);
    let _ = txn.open_table(PENDING_REGISTRATIONS);
    let _ = txn.open_table(NEXT_IDS);
    txn.commit().expect("Failed to commit initial tables");
}

pub fn load_inventory(db: &Database, character_id: u64) -> InventoryData {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(INVENTORIES)
        .expect("Failed to open inventories table");
    match table.get(character_id).expect("Failed to read inventory") {
        Some(guard) => bitcode::decode::<InventoryData>(guard.value()).expect("decode inventory"),
        None => {
            let inventory = default_inventory(db);
            save_inventory(db, character_id, &inventory);
            inventory
        }
    }
}

pub fn save_inventory(db: &Database, character_id: u64, inventory: &InventoryData) {
    let bytes = bitcode::encode(inventory);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(INVENTORIES)
            .expect("Failed to open inventories table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert inventory");
    }
    txn.commit().expect("Failed to commit inventory save");
}

pub fn load_mailbox(db: &Database, character_id: u64) -> Vec<MailEntryData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(MAILBOXES)
        .expect("Failed to open mailboxes table");
    match table.get(character_id).expect("Failed to read mailbox") {
        Some(guard) => {
            bitcode::decode::<Vec<MailEntryData>>(guard.value()).expect("decode mailbox")
        }
        None => Vec::new(),
    }
}

pub fn save_mailbox(db: &Database, character_id: u64, mailbox: &[MailEntryData]) {
    let bytes = bitcode::encode(mailbox);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(MAILBOXES)
            .expect("Failed to open mailboxes table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert mailbox");
    }
    txn.commit().expect("Failed to commit mailbox save");
}

pub fn append_mail(db: &Database, character_id: u64, mail: MailEntryData) {
    let mut mailbox = load_mailbox(db, character_id);
    mailbox.push(mail);
    save_mailbox(db, character_id, &mailbox);
}

pub fn load_auctions(db: &Database) -> Vec<AuctionData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(AUCTIONS)
        .expect("Failed to open auctions table");
    table
        .iter()
        .expect("Failed to iterate auctions")
        .map(|row| {
            let (_key, value) = row.expect("Failed to read auction row");
            bitcode::decode::<AuctionData>(value.value()).expect("decode auction")
        })
        .collect()
}

pub fn save_auction(db: &Database, auction: &AuctionData) {
    let bytes = bitcode::encode(auction);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(AUCTIONS)
            .expect("Failed to open auctions table");
        table
            .insert(auction.auction_id, bytes.as_slice())
            .expect("Failed to insert auction");
    }
    txn.commit().expect("Failed to commit auction save");
}

pub fn delete_auction(db: &Database, auction_id: u64) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(AUCTIONS)
            .expect("Failed to open auctions table");
        table.remove(auction_id).expect("Failed to remove auction");
    }
    txn.commit().expect("Failed to commit auction delete");
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

/// Marker component linking a player entity to its character_id for periodic saves.
#[derive(Component, Debug)]
pub struct ClientId(pub u64);

/// Save interval in ticks (300 ticks at 20Hz = 15 seconds).
const SAVE_INTERVAL_TICKS: u64 = 300;

type DirtyPlayerData = (&'static Position, &'static Health, &'static Player, &'static ClientId);
type DirtyPlayerFilter = (
    Or<(Changed<Position>, Changed<Health>, Changed<Player>)>,
    With<Player>,
    With<ClientId>,
);

#[derive(SystemParam)]
struct DirtyPlayersQuery<'w, 's> {
    query: Query<'w, 's, DirtyPlayerData, DirtyPlayerFilter>,
}

/// Periodically save all player data to the database.
fn save_dirty_characters(
    db: Option<Res<GameDatabase>>,
    players: DirtyPlayersQuery,
    mut tick_count: Local<u64>,
) {
    *tick_count += 1;
    if !(*tick_count).is_multiple_of(SAVE_INTERVAL_TICKS) {
        return;
    }
    let Some(db) = db else { return };
    let mut saved_count = 0;
    for (pos, health, player, cid) in players.query.iter() {
        let data = CharacterData::from_components(player, pos, health);
        save_character(&db.db, cid.0, &data);
        saved_count += 1;
    }
    if saved_count == 0 {
        return;
    }
    info!("Periodic save: {} characters written", saved_count);
}

fn flush_all_characters_on_exit(
    db: Option<Res<GameDatabase>>,
    players: Query<(&Position, &Health, &Player, &ClientId)>,
    mut exit_events: MessageReader<AppExit>,
) {
    if exit_events.read().next().is_none() {
        return;
    }

    let Some(db) = db else { return };

    let mut saved_count = 0usize;
    for (pos, health, player, cid) in players.iter() {
        let data = CharacterData::from_components(player, pos, health);
        save_character(&db.db, cid.0, &data);
        saved_count += 1;
    }
    if saved_count > 0 {
        info!("Shutdown save: {} characters written", saved_count);
    }
}

#[cfg(test)]
pub(crate) fn temp_database() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.redb");
    let db = Database::create(&path).unwrap();
    create_tables(&db);
    std::mem::forget(dir);
    db
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
