use crate::profession::PlayerProfessions;
use bevy::prelude::*;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use shared::reputation::CharacterReputation;
use std::path::{Path, PathBuf};

#[path = "persistence_appearances.rs"]
pub(crate) mod appearance_store;
#[path = "persistence_banks.rs"]
pub(crate) mod banks;
#[path = "persistence_charters.rs"]
pub(crate) mod charters;
#[path = "persistence_equipment.rs"]
pub(crate) mod equipment;
#[path = "persistence_guild_tabards.rs"]
pub(crate) mod guild_tabards;
#[path = "persistence_housing.rs"]
pub(crate) mod housing_store;
#[path = "persistence_inventory.rs"]
pub(crate) mod inventory_store;
#[path = "persistence_mail.rs"]
pub(crate) mod mail_store;
#[path = "persistence_mounts.rs"]
pub(crate) mod mount_store;
#[path = "persistence_runtime.rs"]
mod persistence_runtime;
#[path = "persistence_pets.rs"]
pub(crate) mod pet_store;
#[path = "persistence_quests.rs"]
pub(crate) mod quest_store;
#[path = "persistence_taxi.rs"]
pub(crate) mod taxi_store;
#[path = "persistence_toys.rs"]
pub(crate) mod toy_store;

#[cfg(test)]
#[path = "persistence_charters_tests.rs"]
mod charter_tests;
#[cfg(test)]
#[path = "persistence_guild_tabards_tests.rs"]
mod guild_tabard_tests;

pub use self::persistence_runtime::{ClientId, save_live_character};
use self::persistence_runtime::{flush_all_characters_on_exit, save_dirty_characters};
pub use crate::persistence_character::*;
pub use crate::persistence_types::*;

/// Character data table: character_id (u64) -> serialized CharacterData (bytes).
pub(crate) const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

/// Inventory table: PlayerId (u64) -> serialized inventory (bytes).
/// Inventory table: character_id -> bitcode `InventoryData` (gold in `u64` copper).
pub(crate) const INVENTORIES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_inventories");
/// Inventories written before gold became `u64` (gold in `u32` copper); moved into
/// `INVENTORIES` and dropped at startup.
pub(crate) const LEGACY_INVENTORIES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("inventories");

#[derive(bitcode::Decode)]
struct LegacyInventoryItemData {
    item_guid: u64,
    item_id: u32,
    name: String,
    quality: u8,
    required_level: u16,
    stack_count: u32,
    vendor_sell_price: u32,
}

/// The first inventory layout: items without `unique_auction`.
#[derive(bitcode::Decode)]
struct LegacyInventoryData {
    gold: u32,
    items: Vec<LegacyInventoryItemData>,
}

/// The layout before gold became `u64`.
#[derive(bitcode::Decode)]
struct LegacyInventoryDataU32Gold {
    gold: u32,
    items: Vec<InventoryItemData>,
}

/// Profession table: PlayerId (u64) -> bitcode `PlayerProfessions` (skill lines + spells).
pub(crate) const PROFESSIONS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("profession_state");
/// The AzerothCore-era profession table (placeholder Mining/Blacksmithing state), deleted.
const LEGACY_PROFESSIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("professions");

/// Reputation table: PlayerId (u64) -> serialized reputation state (bytes).
pub(crate) const REPUTATIONS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("reputations_retail_v2");
const LEGACY_REPUTATIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("reputations");

/// Currency table: PlayerId (u64) -> serialized currency wallet (bytes).
pub(crate) const CURRENCIES: TableDefinition<u64, &[u8]> = TableDefinition::new("currencies");

/// Achievement table: PlayerId (u64) -> serialized achievement state (bytes).
pub(crate) const ACHIEVEMENTS: TableDefinition<u64, &[u8]> = TableDefinition::new("achievements");

/// PVP table: PlayerId (u64) -> serialized pvp state (bytes).
const PVP_STATES: TableDefinition<u64, &[u8]> = TableDefinition::new("pvp_states");

/// World map table: PlayerId (u64) -> serialized world map discovery state (bytes).
const WORLD_MAPS: TableDefinition<u64, &[u8]> = TableDefinition::new("world_maps");

/// Friends table: PlayerId (u64) -> serialized friends state (bytes).
const FRIENDS: TableDefinition<u64, &[u8]> = TableDefinition::new("friends");

/// Ignore list table: PlayerId (u64) -> serialized ignore state (bytes).
const IGNORE_LISTS: TableDefinition<u64, &[u8]> = TableDefinition::new("ignore_lists");

/// Collection table: PlayerId (u64) -> serialized collection state (bytes).
const COLLECTIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("collections");

/// Active auction table: auction_id (u64) -> bitcode `AuctionData` (money in `u64` copper).
pub(crate) const AUCTIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("auction_listings");
/// Auctions written before auction money became `u64`; moved into `AUCTIONS` and
/// dropped at startup.
pub(crate) const LEGACY_AUCTIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("auctions");

/// The auction layout before money became `u64`.
#[derive(bitcode::Encode, bitcode::Decode)]
pub(crate) struct LegacyAuctionDataU32Money {
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
    pub faction: u8,
    pub deposit: u32,
}

impl From<LegacyAuctionDataU32Money> for AuctionData {
    fn from(legacy: LegacyAuctionDataU32Money) -> Self {
        Self {
            auction_id: legacy.auction_id,
            seller_character_id: legacy.seller_character_id,
            seller_name: legacy.seller_name,
            item: legacy.item,
            stack_count: legacy.stack_count,
            min_bid: u64::from(legacy.min_bid),
            current_bid: legacy.current_bid.map(u64::from),
            current_bidder_character_id: legacy.current_bidder_character_id,
            buyout_price: legacy.buyout_price.map(u64::from),
            expires_at_unix_secs: legacy.expires_at_unix_secs,
            faction: legacy.faction,
            deposit: u64::from(legacy.deposit),
        }
    }
}

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

/// Guild profile data: guild_id (u32) -> serialized GuildProfileData.
pub(crate) const GUILD_PROFILES: TableDefinition<u32, &[u8]> =
    TableDefinition::new("guild_profiles");

/// Guild officer notes: character_id (u64) -> serialized GuildMemberNoteData.
pub(crate) const GUILD_MEMBER_NOTES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("guild_member_notes");

/// Public guild notes stay separate from the existing bitcode officer-note records.
pub(crate) const GUILD_PUBLIC_NOTES: TableDefinition<u64, &str> =
    TableDefinition::new("guild_public_notes");

/// Password hashes: account_id (u64) -> argon2 hash string (bytes).
pub(crate) const PASSWORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("passwords");

/// Username to account_id mapping: username (&str) -> account_id (u64).
pub(crate) const USERNAME_ACCOUNTS: TableDefinition<&str, u64> =
    TableDefinition::new("username_accounts");

/// Pending registrations: username (&str) -> argon2 hash string (bytes).
pub(crate) const PENDING_REGISTRATIONS: TableDefinition<&str, &[u8]> =
    TableDefinition::new("pending_registrations");

/// Banned accounts: account_id (u64) -> UTF-8 ban reason (bytes).
pub(crate) const BANNED_ACCOUNTS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("banned_accounts");

/// Auto-increment counters: key (&str) -> next_id (u64).
pub(crate) const NEXT_IDS: TableDefinition<&str, u64> = TableDefinition::new("next_ids");

/// Corpses of released (ghost) characters: character_id (u64) -> bitcode
/// `CorpseData` (AzerothCore `corpse` table, read by `Player::LoadCorpse`).
pub(crate) const CHARACTER_CORPSES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_corpses");

/// Full death state, saved atomically with character HP. Separate from the
/// unchanged bitcode character/corpse layouts so pre-existing rows stay readable.
pub(crate) const CHARACTER_DEATH_STATES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_death_states");

/// A ghost's corpse, in the Bevy-space coordinates of `shared::death::Corpse`.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct CorpseData {
    pub map_id: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub created_at: u64,
}

/// Server settings: key (&str) -> value (&[u8]).
const SERVER_SETTINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("server_settings");

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
    pub db: std::sync::Arc<Database>,
    pub(crate) writer: crate::persistence_writer::PersistenceWriter,
    pub(crate) accounts: crate::achievement::AccountProgressCache,
}

impl GameDatabase {
    #[cfg(test)]
    pub(crate) fn into_database(self) -> Database {
        let Self {
            db,
            writer,
            accounts,
        } = self;
        drop(writer);
        drop(accounts);
        std::sync::Arc::try_unwrap(db)
            .unwrap_or_else(|_| panic!("database remains shared after writer drain"))
    }

    pub(crate) fn new(db: Database) -> Self {
        let db = std::sync::Arc::new(db);
        Self {
            writer: crate::persistence_writer::PersistenceWriter::start(std::sync::Arc::clone(&db)),
            db,
            accounts: Default::default(),
        }
    }
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
    let path = game_database_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create data directory");
    }
    let db = open_database_with_retry(&path);
    create_tables(&db);
    let motd = load_motd(&db);
    if motd.is_set() {
        info!("Loaded MOTD: {}", motd.message);
    }
    commands.insert_resource(motd);
    info!("Database opened at {}", path.display());
    commands.insert_resource(GameDatabase::new(db));
}

fn game_database_path() -> PathBuf {
    std::env::var_os("GAME_SERVER_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/game.redb"))
}

pub(crate) fn create_tables(db: &Database) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    let _ = txn.open_table(CHARACTERS);
    let _ = txn.open_table(INVENTORIES);
    txn.open_table(equipment::EQUIPMENT)
        .expect("Failed to create equipment table");
    let _ = txn.open_table(inventory_store::INVENTORY_LAYOUTS);
    let _ = txn.open_table(inventory_store::BAG_SLOTS);
    txn.open_table(inventory_store::NONVISUAL_EQUIPMENT)
        .expect("create nonvisual equipment table");
    let _ = txn.open_table(inventory_store::ITEM_INSTANCES);
    txn.open_table(inventory_store::ITEM_CRAFTING_ORIGINS)
        .expect("create crafting origins table");
    txn.open_table(inventory_store::ITEM_UNLOCKED)
        .expect("create item unlock state table");
    txn.open_table(inventory_store::ITEM_DURABILITIES)
        .expect("create item durability table");
    txn.open_table(inventory_store::ITEM_DEFINITION_SOURCES)
        .expect("Failed to create item definition sources table");
    quest_store::migrate(&txn, quest_store::unix_now());
    let _ = txn.open_table(taxi_store::KNOWN_TAXI_NODES);
    let _ = txn.open_table(pet_store::PET_STABLES);
    let _ = txn.open_table(PROFESSIONS);
    crate::profession::favorites::initialize_store(&txn);
    crate::profession::specializations::initialize_store(&txn);
    txn.delete_table(LEGACY_PROFESSIONS)
        .expect("Failed to delete legacy professions table");
    let _ = txn.open_table(REPUTATIONS);
    let _ = txn.open_table(LEGACY_REPUTATIONS);
    let _ = txn.open_table(CURRENCIES);
    let _ = txn.open_table(ACHIEVEMENTS);
    let _ = txn.open_table(PVP_STATES);
    let _ = txn.open_table(WORLD_MAPS);
    let _ = txn.open_table(FRIENDS);
    let _ = txn.open_table(IGNORE_LISTS);
    let _ = txn.open_table(COLLECTIONS);
    let _ = txn.open_table(AUCTIONS);
    let _ = txn.open_table(mail_store::MAILBOXES);
    let _ = txn.open_table(ACCOUNTS);
    let _ = txn.open_table(ACCOUNT_CHARACTERS);
    let _ = txn.open_table(CHARACTER_NAMES);
    let _ = txn.open_table(CHARACTER_GUILDS);
    txn.open_table(crate::guild_event_log::EVENTS)
        .expect("create guild lifecycle event history");
    txn.open_table(crate::guild_news::NEWS)
        .expect("create guild news history");
    let _ = txn.open_table(PENDING_GUILD_INVITES);
    txn.open_table(charters::STATE)
        .expect("create charter state table");
    txn.open_table(charters::CREATED_AT)
        .expect("create actual guild date table");
    let _ = txn.open_table(PASSWORDS);
    let _ = txn.open_table(USERNAME_ACCOUNTS);
    let _ = txn.open_table(PENDING_REGISTRATIONS);
    let _ = txn.open_table(BANNED_ACCOUNTS);
    let _ = txn.open_table(NEXT_IDS);
    let _ = txn.open_table(SERVER_SETTINGS);
    let _ = txn.open_table(CHARACTER_CORPSES);
    txn.open_table(CHARACTER_DEATH_STATES)
        .expect("Failed to create character death states table");
    txn.open_table(crate::pvp::reconnect::ENTRIES)
        .expect("Failed to create battleground entry points table");
    let _ = txn.open_table(crate::maps::persistence::CHARACTER_MAPS);
    txn.open_table(crate::spell_cast::homebind::HOME_BINDS)
        .expect("Failed to create character homebinds");
    txn.open_table(crate::maps::persistence::NORMAL_INSTANCE_LOCKS)
        .expect("Failed to create normal instance locks");
    let _ = txn.open_table(crate::persistence_progression::CHARACTER_PROGRESSION);
    let _ = txn.open_table(crate::persistence_progression::CHARACTER_TRAITS);
    let _ = txn.open_table(crate::persistence_progression::CHARACTER_XP);
    let _ = txn.open_table(crate::persistence_progression::CHARACTER_FLIGHT_STYLE);
    let _ = txn.open_table(crate::persistence_progression::CHARACTER_REST);
    banks::create_bank_tables(&txn);
    txn.open_table(guild_tabards::DESIGNS)
        .expect("migrate legacy guilds to optional tabard designs");
    appearance_store::create_appearance_tables(&txn);
    toy_store::create_tables(&txn);
    housing_store::create_tables(&txn);
    mount_store::create_tables(&txn);
    txn.commit().expect("Failed to commit initial tables");
    migrate_legacy_inventories(db);
    migrate_legacy_characters(db);
    migrate_legacy_auctions(db);
    mail_store::migrate_sender_mailboxes(db);
    mail_store::migrate_legacy_mailboxes(db, crate::guild_bank::unix_now());
    banks::migrate_guild_ranks(db);
    charters::initialize(db);
}

/// Version of a persisted layout, stored as `SERVER_SETTINGS` `key` (`u32` little-endian);
/// 0 when never written.
pub(crate) fn layout_version(txn: &redb::WriteTransaction, key: &str) -> u32 {
    let table = txn
        .open_table(SERVER_SETTINGS)
        .expect("Failed to open server_settings for a layout version");
    table
        .get(key)
        .expect("Failed to read layout version")
        .map_or(0, |row| {
            let bytes: [u8; 4] = row
                .value()
                .try_into()
                .unwrap_or_else(|_| panic!("layout version {key} is not a u32"));
            u32::from_le_bytes(bytes)
        })
}

pub(crate) fn set_layout_version(txn: &redb::WriteTransaction, key: &str, version: u32) {
    txn.open_table(SERVER_SETTINGS)
        .expect("Failed to open server_settings for a layout version")
        .insert(key, version.to_le_bytes().as_slice())
        .expect("Failed to write layout version");
}

/// Move every row of the `u32`-money `auctions` table into `auction_listings` and
/// drop the old table, in one transaction.
fn migrate_legacy_auctions(db: &Database) {
    let txn = db
        .begin_write()
        .expect("Failed to begin auction migration write");
    let migrated = {
        let legacy = txn
            .open_table(LEGACY_AUCTIONS)
            .expect("Failed to open legacy auctions");
        let rows: Vec<AuctionData> = legacy
            .iter()
            .expect("Failed to iterate legacy auctions")
            .map(|row| {
                let (key, value) = row.expect("Failed to read legacy auction row");
                bitcode::decode::<LegacyAuctionDataU32Money>(value.value())
                    .unwrap_or_else(|error| {
                        panic!("Failed to migrate auction {}: {error:?}", key.value())
                    })
                    .into()
            })
            .collect();
        let mut table = txn
            .open_table(AUCTIONS)
            .expect("Failed to open auctions table for migration");
        for auction in &rows {
            table
                .insert(auction.auction_id, bitcode::encode(auction).as_slice())
                .expect("Failed to write migrated auction");
        }
        rows.len()
    };
    txn.delete_table(LEGACY_AUCTIONS)
        .expect("Failed to drop legacy auctions");
    txn.commit().expect("Failed to commit auction migration");
    if migrated > 0 {
        info!("Migrated {migrated} legacy auctions to u64 money");
    }
}

fn migrate_legacy_characters(db: &Database) {
    let migrations = collect_legacy_character_migrations(db);
    if migrations.is_empty() {
        return;
    }
    let txn = db
        .begin_write()
        .expect("Failed to begin character migration write");
    {
        let mut table = txn
            .open_table(CHARACTERS)
            .expect("Failed to open characters for migration");
        for (id, bytes) in &migrations {
            table
                .insert(*id, bytes.as_slice())
                .expect("Failed to write migrated character");
        }
    }
    txn.commit().expect("Failed to commit character migration");
    info!("Migrated {} legacy character records", migrations.len());
}

fn collect_legacy_character_migrations(db: &Database) -> Vec<(u64, Vec<u8>)> {
    let txn = db
        .begin_read()
        .expect("Failed to begin character migration read");
    let table = txn
        .open_table(CHARACTERS)
        .expect("Failed to open characters for migration");
    table
        .iter()
        .expect("Failed to iterate character records")
        .filter_map(|row| {
            let (id, bytes) = row.expect("Failed to read character migration row");
            if CharacterData::decode_stored(bytes.value()).is_ok() {
                return None;
            }
            let data = CharacterData::decode_compatible(bytes.value()).unwrap_or_else(|error| {
                panic!("Failed to migrate character {}: {error}", id.value())
            });
            Some((id.value(), data.encode_stored()))
        })
        .collect()
}

/// Move every row of the `u32`-gold `inventories` table into `character_inventories`
/// and drop the old table, in one transaction. Rows decode as the pre-`u64` layout
/// or the older layout without `unique_auction`.
fn migrate_legacy_inventories(db: &Database) {
    let txn = db
        .begin_write()
        .expect("Failed to begin inventory migration write");
    let migrated = {
        let legacy = match txn.open_table(LEGACY_INVENTORIES) {
            Ok(table) => table,
            Err(error) => panic!("Failed to open legacy inventories: {error}"),
        };
        let rows: Vec<(u64, InventoryData)> = legacy
            .iter()
            .expect("Failed to iterate legacy inventories")
            .map(|row| {
                let (key, value) = row.expect("Failed to read legacy inventory row");
                let character_id = key.value();
                let inventory = decode_legacy_inventory(value.value()).unwrap_or_else(|error| {
                    panic!("Failed to migrate inventory for character {character_id}: {error}")
                });
                (character_id, inventory)
            })
            .collect();
        let mut table = txn
            .open_table(INVENTORIES)
            .expect("Failed to open inventories table for migration");
        for (character_id, inventory) in &rows {
            table
                .insert(*character_id, bitcode::encode(inventory).as_slice())
                .expect("Failed to write migrated inventory");
        }
        rows.len()
    };
    txn.delete_table(LEGACY_INVENTORIES)
        .expect("Failed to drop legacy inventories");
    txn.commit().expect("Failed to commit inventory migration");
    if migrated > 0 {
        info!("Migrated {migrated} legacy inventory records to u64 gold");
    }
}

fn decode_legacy_inventory(bytes: &[u8]) -> Result<InventoryData, String> {
    match bitcode::decode::<LegacyInventoryDataU32Gold>(bytes) {
        Ok(record) => Ok(InventoryData {
            gold: u64::from(record.gold),
            items: record.items,
        }),
        Err(current_error) => bitcode::decode::<LegacyInventoryData>(bytes)
            .map(inventory_data_from_legacy)
            .map_err(|legacy_error| {
                format!("u32-gold decode {current_error:?}; first layout decode {legacy_error:?}")
            }),
    }
}

fn inventory_data_from_legacy(legacy: LegacyInventoryData) -> InventoryData {
    InventoryData {
        gold: u64::from(legacy.gold),
        items: legacy
            .items
            .into_iter()
            .map(|item| InventoryItemData {
                item_guid: item.item_guid,
                item_id: item.item_id,
                name: item.name,
                quality: item.quality,
                required_level: item.required_level,
                stack_count: item.stack_count,
                vendor_sell_price: item.vendor_sell_price,
                unique_auction: false,
            })
            .collect(),
    }
}

pub fn load_inventory(db: &Database, character_id: u64) -> InventoryData {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(INVENTORIES)
        .expect("Failed to open inventories table");
    match table.get(character_id).expect("Failed to read inventory") {
        Some(guard) => bitcode::decode::<InventoryData>(guard.value()).expect("decode inventory"),
        None => {
            let inventory = InventoryData::default();
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

pub fn load_professions(db: &Database, character_id: u64) -> PlayerProfessions {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(PROFESSIONS)
        .expect("Failed to open professions table");
    match table.get(character_id).expect("Failed to read professions") {
        Some(guard) => {
            bitcode::decode::<PlayerProfessions>(guard.value()).expect("decode professions")
        }
        None => PlayerProfessions::default(),
    }
}

pub fn save_professions(db: &Database, character_id: u64, professions: &PlayerProfessions) {
    let bytes = bitcode::encode(professions);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(PROFESSIONS)
            .expect("Failed to open professions table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert professions");
    }
    txn.commit().expect("Failed to commit professions save");
}

pub fn load_reputation(
    db: &Database,
    character_id: u64,
    default: impl FnOnce() -> CharacterReputation,
) -> CharacterReputation {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(REPUTATIONS)
        .expect("Failed to open reputations table");
    if let Some(guard) = table.get(character_id).expect("Failed to read reputation") {
        return bitcode::decode::<CharacterReputation>(guard.value()).expect("decode reputation");
    }
    let legacy = txn
        .open_table(LEGACY_REPUTATIONS)
        .expect("open legacy reputation")
        .get(character_id)
        .expect("read legacy reputation")
        .map(|guard| {
            bitcode::decode::<CharacterReputation>(guard.value()).expect("decode legacy reputation")
        });
    drop(table);
    drop(txn);
    let reputation = if let Some(mut legacy) = legacy {
        let character =
            load_character(db, character_id).expect("legacy reputation character must exist");
        legacy.migrate_legacy_baselines(
            &shared::reputation::default_faction_registry(),
            character.race,
            character.class,
        );
        legacy
    } else {
        default()
    };
    persist_reputation_migration(db, character_id, &reputation);
    reputation
}

fn persist_reputation_migration(
    db: &Database,
    character_id: u64,
    reputation: &CharacterReputation,
) {
    let txn = db.begin_write().expect("begin signed reputation migration");
    txn.open_table(REPUTATIONS)
        .expect("open signed reputations")
        .insert(character_id, bitcode::encode(reputation).as_slice())
        .expect("save signed reputation");
    txn.open_table(LEGACY_REPUTATIONS)
        .expect("open legacy reputations")
        .remove(character_id)
        .expect("retire migrated reputation");
    txn.commit().expect("commit signed reputation migration");
}

pub fn save_reputation(db: &Database, character_id: u64, reputation: &CharacterReputation) {
    let bytes = bitcode::encode(reputation);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(REPUTATIONS)
            .expect("Failed to open reputations table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert reputation");
    }
    txn.commit().expect("Failed to commit reputation save");
}

pub fn load_currencies(db: &Database, character_id: u64) -> shared::currency::CurrencyWallet {
    let mut wallet = {
        let txn = db.begin_read().expect("Failed to begin currency read");
        let table = txn
            .open_table(CURRENCIES)
            .expect("Failed to open currencies table");
        table
            .get(character_id)
            .expect("Failed to read currencies")
            .map(|guard| {
                serde_json::from_slice::<shared::currency::CurrencyWallet>(guard.value())
                    .expect("decode currencies")
            })
            .unwrap_or_default()
    };
    if migrate_pvp_currency_ids(&mut wallet) {
        save_currencies(db, character_id, &wallet);
    }
    wallet
}

/// One-way storage conversion: obsolete internal IDs are never alternate API IDs.
fn migrate_pvp_currency_ids(wallet: &mut shared::currency::CurrencyWallet) -> bool {
    let mut changed = false;
    for (old, current) in [
        (1, shared::currency::HONOR_CURRENCY_ID),
        (2, shared::currency::CONQUEST_CURRENCY_ID),
    ] {
        let Some(index) = wallet.entries.iter().position(|entry| entry.id == old) else {
            continue;
        };
        let mut legacy = wallet.entries.remove(index);
        if let Some(entry) = wallet.entries.iter_mut().find(|entry| entry.id == current) {
            entry.amount = entry
                .amount
                .checked_add(legacy.amount)
                .expect("PvP balance migration overflow");
            entry.week_earned = entry
                .week_earned
                .checked_add(legacy.week_earned)
                .expect("PvP weekly-earned migration overflow");
        } else {
            legacy.id = current;
            wallet.entries.push(legacy);
        }
        changed = true;
    }
    changed
}

pub fn save_currencies(
    db: &Database,
    character_id: u64,
    wallet: &shared::currency::CurrencyWallet,
) {
    let bytes = serde_json::to_vec(wallet).expect("encode currencies");
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CURRENCIES)
            .expect("Failed to open currencies table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert currencies");
    }
    txn.commit().expect("Failed to commit currencies save");
}

/// Offline/test migration helper. Runtime readers use the owned account cache.
#[cfg(test)]
pub fn load_achievements(
    db: &Database,
    character_id: u64,
) -> crate::achievement::PlayerAchievementState {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(ACHIEVEMENTS)
        .expect("Failed to open achievements table");
    match table
        .get(character_id)
        .expect("Failed to read achievements")
    {
        Some(guard) => {
            let (state, migrated) =
                crate::achievement::decode_saved_achievement_state(guard.value());
            if migrated {
                save_achievements(db, character_id, &state);
            }
            state
        }
        None => {
            let state = crate::achievement::PlayerAchievementState::default();
            save_achievements(db, character_id, &state);
            state
        }
    }
}

#[cfg(test)]
pub fn save_achievements(
    db: &Database,
    character_id: u64,
    state: &crate::achievement::PlayerAchievementState,
) {
    let bytes = serde_json::to_vec(state).expect("encode achievements");
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(ACHIEVEMENTS)
            .expect("Failed to open achievements table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert achievements");
    }
    txn.commit().expect("Failed to commit achievements save");
}

pub fn load_player_death_state(
    db: &Database,
    character_id: u64,
) -> Option<crate::death::PlayerDeathState> {
    let txn = db.begin_read().expect("Failed to begin death state read");
    let table = txn
        .open_table(CHARACTER_DEATH_STATES)
        .expect("Failed to open character death states table");
    table
        .get(character_id)
        .expect("Failed to read character death state")
        .map(|row| {
            serde_json::from_slice(row.value()).expect("Failed to decode character death state")
        })
}

pub fn load_corpse(db: &Database, character_id: u64) -> Option<CorpseData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CHARACTER_CORPSES)
        .expect("Failed to open corpse table");
    table
        .get(character_id)
        .expect("Failed to read corpse")
        .map(|guard| bitcode::decode(guard.value()).expect("decode corpse"))
}

pub fn load_pvp(db: &Database, character_id: u64) -> crate::pvp::PlayerPvpProgress {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(PVP_STATES)
        .expect("Failed to open pvp table");
    match table.get(character_id).expect("Failed to read pvp state") {
        Some(guard) => serde_json::from_slice::<crate::pvp::PlayerPvpProgress>(guard.value())
            .expect("decode pvp state"),
        None => {
            let state = crate::pvp::PlayerPvpProgress::default();
            save_pvp(db, character_id, &state);
            state
        }
    }
}

pub fn save_pvp(db: &Database, character_id: u64, state: &crate::pvp::PlayerPvpProgress) {
    let bytes = serde_json::to_vec(state).expect("encode pvp state");
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(PVP_STATES)
            .expect("Failed to open pvp table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert pvp state");
    }
    txn.commit().expect("Failed to commit pvp save");
}

pub fn load_world_map(db: &Database, character_id: u64) -> crate::world_map::PlayerWorldMapState {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(WORLD_MAPS)
        .expect("Failed to open world_maps table");
    match table.get(character_id).expect("Failed to read world map") {
        Some(guard) => {
            serde_json::from_slice::<crate::world_map::PlayerWorldMapState>(guard.value())
                .expect("decode world map")
        }
        None => {
            let state = crate::world_map::PlayerWorldMapState::default();
            save_world_map(db, character_id, &state);
            state
        }
    }
}

pub fn save_world_map(
    db: &Database,
    character_id: u64,
    state: &crate::world_map::PlayerWorldMapState,
) {
    let bytes = serde_json::to_vec(state).expect("encode world map");
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(WORLD_MAPS)
            .expect("Failed to open world_maps table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert world map");
    }
    txn.commit().expect("Failed to commit world map save");
}

pub fn load_friends(db: &Database, character_id: u64) -> crate::friends::PlayerFriendsState {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(FRIENDS)
        .expect("Failed to open friends table");
    match table.get(character_id).expect("Failed to read friends") {
        Some(guard) => serde_json::from_slice::<crate::friends::PlayerFriendsState>(guard.value())
            .expect("decode friends"),
        None => {
            let state = crate::friends::PlayerFriendsState::default();
            save_friends(db, character_id, &state);
            state
        }
    }
}

pub fn save_friends(db: &Database, character_id: u64, state: &crate::friends::PlayerFriendsState) {
    let bytes = serde_json::to_vec(state).expect("encode friends");
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(FRIENDS)
            .expect("Failed to open friends table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert friends");
    }
    txn.commit().expect("Failed to commit friends save");
}

pub fn load_ignore_list(
    db: &Database,
    character_id: u64,
) -> crate::ignore_list::PlayerIgnoreListState {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(IGNORE_LISTS)
        .expect("Failed to open ignore list table");
    match table.get(character_id).expect("Failed to read ignore list") {
        Some(guard) => {
            serde_json::from_slice::<crate::ignore_list::PlayerIgnoreListState>(guard.value())
                .expect("decode ignore list")
        }
        None => {
            let state = crate::ignore_list::PlayerIgnoreListState::default();
            save_ignore_list(db, character_id, &state);
            state
        }
    }
}

pub fn save_ignore_list(
    db: &Database,
    character_id: u64,
    state: &crate::ignore_list::PlayerIgnoreListState,
) {
    let bytes = serde_json::to_vec(state).expect("encode ignore list");
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(IGNORE_LISTS)
            .expect("Failed to open ignore list table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert ignore list");
    }
    txn.commit().expect("Failed to commit ignore list save");
}

pub fn load_collections(
    db: &Database,
    character_id: u64,
) -> crate::collection::PlayerCollectionState {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(COLLECTIONS)
        .expect("Failed to open collections table");
    match table.get(character_id).expect("Failed to read collections") {
        Some(guard) => {
            serde_json::from_slice::<crate::collection::PlayerCollectionState>(guard.value())
                .expect("decode collections")
        }
        None => {
            let collections = crate::collection::default_collection_state();
            save_collections(db, character_id, &collections);
            collections
        }
    }
}

pub fn save_collections(
    db: &Database,
    character_id: u64,
    collections: &crate::collection::PlayerCollectionState,
) {
    let bytes = serde_json::to_vec(collections).expect("encode collections");
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(COLLECTIONS)
            .expect("Failed to open collections table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert collections");
    }
    txn.commit().expect("Failed to commit collections save");
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

/// Fixture writes use the same atomic market path as runtime operations.
#[cfg(test)]
pub fn save_auction(db: &Database, auction: &AuctionData) {
    let changes = crate::economy::store::MarketChanges {
        auctions: vec![auction.clone()],
        ..Default::default()
    };
    crate::economy::store::commit_market(db, None, &changes).expect("save auction fixture");
}

#[cfg(test)]
pub fn delete_auction(db: &Database, auction_id: u64) {
    let changes = crate::economy::store::MarketChanges {
        deleted_auctions: vec![auction_id],
        ..Default::default()
    };
    crate::economy::store::commit_market(db, None, &changes).expect("delete auction fixture");
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

/// MOTD until an admin sets or clears one.
const DEFAULT_MOTD: &str = "Welcome to World of Osso.";

fn default_motd() -> shared::motd::Motd {
    shared::motd::Motd {
        message: DEFAULT_MOTD.to_string(),
        updated_at: 0,
    }
}

/// Load the MOTD from the database. Returns `DEFAULT_MOTD` if never saved.
pub fn load_motd(db: &Database) -> shared::motd::Motd {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = match txn.open_table(SERVER_SETTINGS) {
        Ok(t) => t,
        Err(_) => return default_motd(),
    };
    match table.get("motd").expect("Failed to read motd") {
        Some(bytes) => bitcode::decode(bytes.value()).unwrap_or_default(),
        None => default_motd(),
    }
}

/// Save the MOTD to the database.
pub fn save_motd(db: &Database, motd: &shared::motd::Motd) {
    let bytes = bitcode::encode(motd);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(SERVER_SETTINGS)
            .expect("Failed to open server_settings table");
        table
            .insert("motd", bytes.as_slice())
            .expect("Failed to insert motd");
    }
    txn.commit().expect("Failed to commit motd save");
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

#[cfg(test)]
#[path = "startup_tests.rs"]
mod startup_tests;

#[cfg(test)]
#[path = "persistence_customization_tests.rs"]
mod customization_tests;

#[cfg(test)]
#[path = "persistence_character_abi_tests.rs"]
mod character_abi_tests;
