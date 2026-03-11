use bevy::prelude::*;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use rusqlite::{Connection, OpenFlags, params};
use shared::components::{GuildMembership, Health, Player, Position};
use std::path::Path;

/// Character data table: character_id (u64) -> serialized CharacterData (bytes).
const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

/// Inventory table: PlayerId (u64) -> serialized inventory (bytes).
const INVENTORIES: TableDefinition<u64, &[u8]> = TableDefinition::new("inventories");

/// Active auction table: auction_id (u64) -> serialized auction data (bytes).
const AUCTIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("auctions");

/// Mailbox table: character_id (u64) -> serialized Vec<MailEntryData>.
const MAILBOXES: TableDefinition<u64, &[u8]> = TableDefinition::new("mailboxes");

/// Account table: token (&str) -> account_id (u64).
pub(crate) const ACCOUNTS: TableDefinition<&str, u64> = TableDefinition::new("accounts");

/// Account characters: account_id (u64) -> bitcode-encoded Vec<u64> of character_ids.
const ACCOUNT_CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("account_characters");

/// Character name uniqueness: name (&str) -> character_id (u64).
const CHARACTER_NAMES: TableDefinition<&str, u64> = TableDefinition::new("character_names");

/// Character guild membership: character_id (u64) -> serialized GuildMembershipData.
const CHARACTER_GUILDS: TableDefinition<u64, &[u8]> = TableDefinition::new("character_guilds");

/// Pending guild invites: character_id (u64) -> serialized PendingGuildInviteData.
const PENDING_GUILD_INVITES: TableDefinition<u64, &[u8]> =
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

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct GuildMembershipData {
    pub guild_id: u32,
    pub guild_name: String,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct PendingGuildInviteData {
    pub guild_id: u32,
    pub guild_name: String,
    pub inviter_name: String,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct InventoryItemData {
    pub item_guid: u64,
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub stack_count: u32,
    pub vendor_sell_price: u32,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct InventoryData {
    pub gold: u32,
    pub items: Vec<InventoryItemData>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct MailEntryData {
    pub mail_id: u64,
    pub subject: String,
    pub body: String,
    pub attached_money: u32,
    pub attached_item: Option<InventoryItemData>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct AuctionData {
    pub auction_id: u64,
    pub seller_character_id: u64,
    pub seller_name: String,
    pub item: InventoryItemData,
    pub stack_count: u32,
    pub min_bid: u32,
    pub current_bid: Option<u32>,
    pub current_bidder_character_id: Option<u64>,
    pub buyout_price: Option<u32>,
    pub expires_at_unix_secs: u64,
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
        Some(guard) => {
            Some(bitcode::decode::<CharacterData>(guard.value()).expect("Failed to decode"))
        }
        None => None,
    }
}

pub fn load_character_guild(db: &Database, character_id: u64) -> Option<GuildMembership> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTER_GUILDS)
        .expect("Failed to open character_guilds table");
    let Some(guard) = table
        .get(character_id)
        .expect("Failed to read character guild")
    else {
        return None;
    };
    let data = bitcode::decode::<GuildMembershipData>(guard.value()).expect("decode guild");
    Some(GuildMembership {
        guild_id: data.guild_id,
        guild_name: data.guild_name,
    })
}

pub fn set_character_guild_by_name(
    db: &Database,
    character_name: &str,
    guild_id: u32,
    guild_name: &str,
) -> Result<u64, String> {
    let character_id = find_character_id_by_name(db, character_name)
        .ok_or_else(|| format!("Character '{character_name}' not found"))?;
    set_character_guild_by_id(db, character_id, guild_id, guild_name)?;
    Ok(character_id)
}

pub fn set_character_guild_by_id(
    db: &Database,
    character_id: u64,
    guild_id: u32,
    guild_name: &str,
) -> Result<(), String> {
    let guild = GuildMembershipData {
        guild_id,
        guild_name: guild_name.to_string(),
    };
    let encoded = bitcode::encode(&guild);
    let write = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = write
            .open_table(CHARACTER_GUILDS)
            .expect("open character_guilds");
        table
            .insert(character_id, encoded.as_slice())
            .expect("insert character guild");
    }
    write.commit().expect("Failed to commit character guild");
    Ok(())
}

pub fn set_pending_guild_invite(
    db: &Database,
    character_id: u64,
    guild_id: u32,
    guild_name: &str,
    inviter_name: &str,
) {
    let invite = PendingGuildInviteData {
        guild_id,
        guild_name: guild_name.to_string(),
        inviter_name: inviter_name.to_string(),
    };
    let encoded = bitcode::encode(&invite);
    let write = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = write
            .open_table(PENDING_GUILD_INVITES)
            .expect("open pending_guild_invites");
        table
            .insert(character_id, encoded.as_slice())
            .expect("insert pending guild invite");
    }
    write
        .commit()
        .expect("Failed to commit pending guild invite");
}

pub fn take_pending_guild_invite(
    db: &Database,
    character_id: u64,
) -> Option<PendingGuildInviteData> {
    let write = db.begin_write().expect("Failed to begin write txn");
    let invite = {
        let mut table = write
            .open_table(PENDING_GUILD_INVITES)
            .expect("open pending_guild_invites");
        let Some(raw) = table
            .get(character_id)
            .expect("read pending guild invite")
            .map(|g| g.value().to_vec())
        else {
            return None;
        };
        let decoded = bitcode::decode::<PendingGuildInviteData>(&raw).expect("decode invite");
        table
            .remove(character_id)
            .expect("remove pending guild invite");
        decoded
    };
    write
        .commit()
        .expect("Failed to commit pending guild invite removal");
    Some(invite)
}

pub fn find_character_id_by_name(db: &Database, character_name: &str) -> Option<u64> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let names = txn
        .open_table(CHARACTER_NAMES)
        .expect("Failed to open character_names");
    names
        .get(character_name)
        .expect("Failed to read character name")
        .map(|v| v.value())
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
        Some(guard) => {
            bitcode::decode::<Vec<u64>>(guard.value()).expect("Failed to decode char ids")
        }
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
    validate_playable_race(race)?;
    let character_id = next_id(db, "character");
    let data = default_character_data(name, race, class);
    save_new_character(db, account_id, character_id, name, &data);
    save_inventory(db, character_id, &default_inventory(db));
    Ok((character_id, data))
}

fn validate_playable_race(race: u8) -> Result<(), String> {
    if let Ok(conn) = Connection::open_with_flags("data/world.db", OpenFlags::SQLITE_OPEN_READ_ONLY)
        && let Ok(mut stmt) = conn.prepare("SELECT 1 FROM chr_races WHERE id = ?1 LIMIT 1")
    {
        let mut rows = stmt
            .query(params![i64::from(race)])
            .map_err(|e| format!("Failed to execute race query: {e}"))?;
        return if rows
            .next()
            .map_err(|e| format!("Failed to read race query result: {e}"))?
            .is_some()
        {
            Ok(())
        } else {
            Err(format!(
                "Invalid race id {race}. See data/world.db table chr_races"
            ))
        };
    }

    const FALLBACK_PLAYABLE_RACES: &[u8] = &[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37,
        52, 70, 84, 85,
    ];
    if FALLBACK_PLAYABLE_RACES.contains(&race) {
        Ok(())
    } else {
        Err(format!("Invalid race id {race}"))
    }
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

fn default_inventory(db: &Database) -> InventoryData {
    InventoryData {
        gold: 100_000,
        items: vec![
            InventoryItemData {
                item_guid: next_id(db, "item_instance"),
                item_id: 2589,
                name: "Linen Cloth".to_string(),
                quality: 1,
                required_level: 1,
                stack_count: 20,
                vendor_sell_price: 13,
            },
            InventoryItemData {
                item_guid: next_id(db, "item_instance"),
                item_id: 2447,
                name: "Peacebloom".to_string(),
                quality: 1,
                required_level: 1,
                stack_count: 10,
                vendor_sell_price: 10,
            },
            InventoryItemData {
                item_guid: next_id(db, "item_instance"),
                item_id: 2770,
                name: "Copper Ore".to_string(),
                quality: 1,
                required_level: 1,
                stack_count: 10,
                vendor_sell_price: 25,
            },
        ],
    }
}

/// Read character IDs from an open account_characters table.
fn read_char_ids_from_table(table: &redb::Table<u64, &[u8]>, account_id: u64) -> Vec<u64> {
    match table.get(account_id).expect("read acct chars") {
        Some(guard) => bitcode::decode::<Vec<u64>>(guard.value()).expect("decode char ids"),
        None => Vec::new(),
    }
}

/// Delete a character, validating account ownership first.
pub fn delete_character(db: &Database, account_id: u64, character_id: u64) -> Result<(), String> {
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
    let remaining: Vec<u64> = char_ids
        .iter()
        .copied()
        .filter(|&id| id != character_id)
        .collect();
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
pub(crate) fn temp_database() -> Database {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.redb");
    let db = Database::create(&path).unwrap();
    create_tables(&db);
    std::mem::forget(dir);
    db
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
