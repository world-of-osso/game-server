use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use shared::components::{Health, Player, Position};
use shared::profession::{PlayerProfessions, Profession};
use shared::reputation::CharacterReputation;
use std::path::Path;

pub use crate::persistence_character::*;
pub use crate::persistence_types::*;

/// Character data table: character_id (u64) -> serialized CharacterData (bytes).
pub(crate) const CHARACTERS: TableDefinition<u64, &[u8]> = TableDefinition::new("characters");

/// Inventory table: PlayerId (u64) -> serialized inventory (bytes).
const INVENTORIES: TableDefinition<u64, &[u8]> = TableDefinition::new("inventories");

/// Profession table: PlayerId (u64) -> serialized profession state (bytes).
const PROFESSIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("professions");

/// Reputation table: PlayerId (u64) -> serialized reputation state (bytes).
const REPUTATIONS: TableDefinition<u64, &[u8]> = TableDefinition::new("reputations");

/// Currency table: PlayerId (u64) -> serialized currency wallet (bytes).
const CURRENCIES: TableDefinition<u64, &[u8]> = TableDefinition::new("currencies");

/// Achievement table: PlayerId (u64) -> serialized achievement state (bytes).
const ACHIEVEMENTS: TableDefinition<u64, &[u8]> = TableDefinition::new("achievements");

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

/// Guild profile data: guild_id (u32) -> serialized GuildProfileData.
pub(crate) const GUILD_PROFILES: TableDefinition<u32, &[u8]> =
    TableDefinition::new("guild_profiles");

/// Guild officer notes: character_id (u64) -> serialized GuildMemberNoteData.
pub(crate) const GUILD_MEMBER_NOTES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("guild_member_notes");

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
    let motd = load_motd(&db);
    if motd.is_set() {
        info!("Loaded MOTD: {}", motd.message);
    }
    commands.insert_resource(motd);
    info!("Database opened at {}", path.display());
    commands.insert_resource(GameDatabase { db });
}

pub(crate) fn create_tables(db: &Database) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    let _ = txn.open_table(CHARACTERS);
    let _ = txn.open_table(INVENTORIES);
    let _ = txn.open_table(PROFESSIONS);
    let _ = txn.open_table(REPUTATIONS);
    let _ = txn.open_table(CURRENCIES);
    let _ = txn.open_table(ACHIEVEMENTS);
    let _ = txn.open_table(WORLD_MAPS);
    let _ = txn.open_table(FRIENDS);
    let _ = txn.open_table(IGNORE_LISTS);
    let _ = txn.open_table(COLLECTIONS);
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
    let _ = txn.open_table(BANNED_ACCOUNTS);
    let _ = txn.open_table(NEXT_IDS);
    let _ = txn.open_table(SERVER_SETTINGS);
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

pub fn load_professions(db: &Database, character_id: u64) -> PlayerProfessions {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(PROFESSIONS)
        .expect("Failed to open professions table");
    match table.get(character_id).expect("Failed to read professions") {
        Some(guard) => {
            bitcode::decode::<PlayerProfessions>(guard.value()).expect("decode professions")
        }
        None => {
            let professions = default_professions();
            save_professions(db, character_id, &professions);
            professions
        }
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
    match table.get(character_id).expect("Failed to read reputation") {
        Some(guard) => {
            bitcode::decode::<CharacterReputation>(guard.value()).expect("decode reputation")
        }
        None => {
            let reputation = default();
            save_reputation(db, character_id, &reputation);
            reputation
        }
    }
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
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn
        .open_table(CURRENCIES)
        .expect("Failed to open currencies table");
    match table.get(character_id).expect("Failed to read currencies") {
        Some(guard) => serde_json::from_slice::<shared::currency::CurrencyWallet>(guard.value())
            .expect("decode currencies"),
        None => {
            let wallet = shared::currency::CurrencyWallet::default();
            save_currencies(db, character_id, &wallet);
            wallet
        }
    }
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
            serde_json::from_slice::<crate::achievement::PlayerAchievementState>(guard.value())
                .expect("decode achievements")
        }
        None => {
            let state = crate::achievement::PlayerAchievementState::default();
            save_achievements(db, character_id, &state);
            state
        }
    }
}

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

fn default_professions() -> PlayerProfessions {
    let mut professions = PlayerProfessions::default();
    professions.learn(Profession::Mining).expect("learn Mining");
    professions
        .learn(Profession::Blacksmithing)
        .expect("learn Blacksmithing");
    professions
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

/// Load the MOTD from the database. Returns default if not set.
pub fn load_motd(db: &Database) -> shared::motd::Motd {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = match txn.open_table(SERVER_SETTINGS) {
        Ok(t) => t,
        Err(_) => return shared::motd::Motd::default(),
    };
    match table.get("motd").expect("Failed to read motd") {
        Some(bytes) => bitcode::decode(bytes.value()).unwrap_or_default(),
        None => shared::motd::Motd::default(),
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

/// Marker component linking a player entity to its character_id for periodic saves.
#[derive(Component, Debug)]
pub struct ClientId(pub u64);

/// Save interval in ticks (300 ticks at 20Hz = 15 seconds).
const SAVE_INTERVAL_TICKS: u64 = 300;

type DirtyPlayerData = (
    &'static Position,
    &'static Health,
    &'static Player,
    &'static ClientId,
);
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
        let mut data = load_character(&db.db, cid.0)
            .unwrap_or_else(|| CharacterData::from_components(player, pos, health));
        data.sync_live_components(player, pos, health);
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
        let mut data = load_character(&db.db, cid.0)
            .unwrap_or_else(|| CharacterData::from_components(player, pos, health));
        data.sync_live_components(player, pos, health);
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

#[cfg(test)]
#[path = "startup_tests.rs"]
mod startup_tests;
