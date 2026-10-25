//! Bank storage: the character bank (by character), the Warband bank (by account),
//! guilds (ranks, bank tabs, money), guild member ranks, daily guild bank usage and
//! guild bank logs. Every bank move writes the bank and the owner's inventory in one
//! transaction (`save_bank_move`).
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};

use crate::bank_storage::BankTabData;
use crate::persistence::inventory_store::{InventoryRecord, write_inventory_record};

/// Character bank: character_id -> bitcode `BankData`.
pub(crate) const CHARACTER_BANKS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_banks");
/// Warband bank: account_id -> bitcode `BankData`.
pub(crate) const ACCOUNT_BANKS: TableDefinition<u64, &[u8]> = TableDefinition::new("account_banks");
/// Guild record: guild_id -> bitcode `GuildData`.
pub(crate) const GUILDS: TableDefinition<u32, &[u8]> = TableDefinition::new("guilds");
/// Guild rank index of a member: character_id -> rank (0 = Guild Master).
pub(crate) const GUILD_MEMBER_RANKS: TableDefinition<u64, u8> =
    TableDefinition::new("guild_member_ranks");
/// Guild bank withdrawals today: character_id -> bitcode `GuildBankUsage`.
pub(crate) const GUILD_BANK_USAGE: TableDefinition<u64, &[u8]> =
    TableDefinition::new("guild_bank_usage");
/// Guild bank logs: guild_id -> bitcode `GuildBankLogs`.
pub(crate) const GUILD_BANK_LOGS: TableDefinition<u32, &[u8]> =
    TableDefinition::new("guild_bank_logs");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Default)]
pub struct BankData {
    pub tabs: Vec<BankTabData>,
    /// Copper (Warband bank only).
    pub money: u64,
}

/// Per-tab rights of a rank (AzerothCore `GuildBankRightsAndSlots`).
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuildTabRights {
    pub view: bool,
    pub deposit: bool,
    /// Stacks per day.
    pub withdrawals_per_day: u32,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct GuildRankData {
    pub name: String,
    /// Social rights (`GUILD_RIGHT_*`); monetary rights retain their existing fields.
    pub rights: u32,
    /// AzerothCore `GR_RIGHT_WITHDRAW_GOLD`.
    pub withdraw_gold: bool,
    /// TrinityCore `GR_RIGHT_WITHDRAW_REPAIR` (Guild.h:123): repairs paid from the guild
    /// bank, out of the same daily allowance as gold withdrawals.
    pub withdraw_repair: bool,
    /// Copper per day (AzerothCore `BankMoneyPerDay`).
    pub gold_per_day: u64,
    /// One entry per guild bank tab (`GUILD_BANK_MAX_TABS`).
    pub tabs: Vec<GuildTabRights>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct GuildData {
    pub ranks: Vec<GuildRankData>,
    pub money: u64,
    pub tabs: Vec<BankTabData>,
}

/// Rank 0; unlimited rights on every tab and on money (AzerothCore `GR_GUILDMASTER`).
pub const GUILD_MASTER_RANK: u8 = 0;

impl Default for GuildData {
    /// TrinityCore `Guild::_CreateDefaultGuildRanks` (Guild.cpp:3184-3188): Guild Master
    /// and Officer get `GR_RIGHT_ALL` (gold withdrawal and guild repair included), the
    /// others chat rights only; no rank but the Guild Master has bank tab rights or a
    /// gold allowance.
    fn default() -> Self {
        let rank = |name: &str, all_rights: bool| GuildRankData {
            name: name.into(),
            rights: default_social_rights(all_rights),
            withdraw_gold: all_rights,
            withdraw_repair: all_rights,
            gold_per_day: 0,
            tabs: vec![GuildTabRights::default(); shared::protocol::GUILD_BANK_MAX_TABS],
        };
        Self {
            ranks: vec![
                rank("Guild Master", true),
                rank("Officer", true),
                rank("Veteran", false),
                rank("Member", false),
                rank("Initiate", false),
            ],
            money: 0,
            tabs: Vec::new(),
        }
    }
}

impl GuildData {
    pub fn lowest_rank(&self) -> u8 {
        (self.ranks.len() - 1) as u8
    }
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Default)]
pub struct GuildBankUsage {
    pub guild_id: u32,
    /// Server day (`daily_reset::day`) the counts belong to.
    pub day: i64,
    pub tab_withdrawals: Vec<u32>,
    pub copper_withdrawn: u64,
}

/// AzerothCore `GuildBankEventLogTypes` a log entry records.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildBankLogKindData {
    DepositItem,
    WithdrawItem,
    DepositMoney,
    WithdrawMoney,
    BuyTab,
    /// TrinityCore `GUILD_BANK_LOG_REPAIR_MONEY`.
    RepairMoney,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct GuildBankLogData {
    pub kind: GuildBankLogKindData,
    pub actor: String,
    pub item_id: u32,
    pub item_name: String,
    pub count: u32,
    pub copper: u64,
    /// Unix seconds.
    pub time: u64,
}

/// AzerothCore `GUILD_BANKLOG_MAX_RECORDS` (SharedDefines.h:3475), per tab and for money.
pub const GUILD_BANK_LOG_MAX: usize = 25;

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Default)]
pub struct GuildBankLogs {
    pub tabs: Vec<Vec<GuildBankLogData>>,
    pub money: Vec<GuildBankLogData>,
}

impl GuildBankLogs {
    /// Append to the tab log (`Some(tab)`) or the money log, oldest entries dropped.
    pub fn push(&mut self, tab: Option<u8>, entry: GuildBankLogData) {
        let log = match tab {
            Some(tab) => {
                let tab = usize::from(tab);
                if self.tabs.len() <= tab {
                    self.tabs.resize(tab + 1, Vec::new());
                }
                &mut self.tabs[tab]
            }
            None => &mut self.money,
        };
        log.push(entry);
        if log.len() > GUILD_BANK_LOG_MAX {
            log.remove(0);
        }
    }

    pub fn log(&self, tab: Option<u8>) -> &[GuildBankLogData] {
        match tab {
            Some(tab) => self.tabs.get(usize::from(tab)).map_or(&[], Vec::as_slice),
            None => &self.money,
        }
    }
}

fn read<K: redb::Key + 'static, T: bitcode::DecodeOwned + Default>(
    db: &Database,
    definition: TableDefinition<K, &[u8]>,
    key: K::SelfType<'_>,
) -> T {
    let txn = db.begin_read().expect("Failed to begin bank read");
    let table = txn.open_table(definition).expect("open bank table");
    table
        .get(key)
        .expect("read bank row")
        .map(|row| bitcode::decode(row.value()).expect("decode bank row"))
        .unwrap_or_default()
}

fn write<K: redb::Key + 'static, T: bitcode::Encode>(
    txn: &WriteTransaction,
    definition: TableDefinition<K, &[u8]>,
    key: K::SelfType<'_>,
    value: &T,
) {
    txn.open_table(definition)
        .expect("open bank table")
        .insert(key, bitcode::encode(value).as_slice())
        .expect("write bank row");
}

pub fn load_character_bank(db: &Database, character_id: u64) -> BankData {
    read(db, CHARACTER_BANKS, character_id)
}

pub fn load_account_bank(db: &Database, account_id: u64) -> BankData {
    read(db, ACCOUNT_BANKS, account_id)
}

pub fn load_guild(db: &Database, guild_id: u32) -> GuildData {
    let mut guild: GuildData = read(db, GUILDS, guild_id);
    // TC a352b1fa Guild::RankInfo::LoadFromDB: never lose Guild Master rights.
    // Monetary rights have separate authoritative booleans; do not change those.
    guild.ranks[0].rights |= default_social_rights(true);
    guild
}

pub fn load_guild_bank_usage(db: &Database, character_id: u64) -> GuildBankUsage {
    read(db, GUILD_BANK_USAGE, character_id)
}

pub fn load_guild_bank_logs(db: &Database, guild_id: u32) -> GuildBankLogs {
    read(db, GUILD_BANK_LOGS, guild_id)
}

/// Rank of a guild member; members without a stored rank hold the lowest rank.
pub fn load_guild_member_rank(db: &Database, guild: &GuildData, character_id: u64) -> u8 {
    let txn = db.begin_read().expect("Failed to begin rank read");
    let table = txn.open_table(GUILD_MEMBER_RANKS).expect("open ranks");
    table
        .get(character_id)
        .expect("read rank")
        .map_or(guild.lowest_rank(), |row| row.value())
}

pub fn save_guild_member_rank(db: &Database, character_id: u64, rank: u8) {
    let txn = db.begin_write().expect("Failed to begin rank write");
    txn.open_table(GUILD_MEMBER_RANKS)
        .expect("open ranks")
        .insert(character_id, rank)
        .expect("write rank");
    txn.commit().expect("Failed to commit rank");
}

pub fn save_guild(db: &Database, guild_id: u32, guild: &GuildData) {
    let txn = db.begin_write().expect("Failed to begin guild write");
    write(&txn, GUILDS, guild_id, guild);
    txn.commit().expect("Failed to commit guild");
}

/// Which bank a move writes.
pub enum BankOwner<'a> {
    Character(u64, &'a BankData),
    Account(u64, &'a BankData),
    Guild {
        guild_id: u32,
        guild: &'a GuildData,
        logs: &'a GuildBankLogs,
        usage: Option<&'a GuildBankUsage>,
    },
}

/// Write a bank and (when items or money moved to or from the player) the
/// player's inventory record in one transaction.
pub fn save_bank_move(
    db: &Database,
    character_id: u64,
    owner: BankOwner,
    inventory: Option<&InventoryRecord>,
) {
    save_bank_moves(db, character_id, [owner], inventory);
}

/// `save_bank_move` for several banks in one transaction.
pub fn save_bank_moves<'a>(
    db: &Database,
    character_id: u64,
    owners: impl IntoIterator<Item = BankOwner<'a>>,
    inventory: Option<&InventoryRecord>,
) {
    let txn = db.begin_write().expect("Failed to begin bank write");
    for owner in owners {
        write_bank_owner(&txn, character_id, owner);
    }
    if let Some(record) = inventory {
        write_inventory_record(&txn, character_id, record);
    }
    txn.commit().expect("Failed to commit bank write");
}

fn write_bank_owner(txn: &WriteTransaction, character_id: u64, owner: BankOwner) {
    match owner {
        BankOwner::Character(id, bank) => write(txn, CHARACTER_BANKS, id, bank),
        BankOwner::Account(id, bank) => write(txn, ACCOUNT_BANKS, id, bank),
        BankOwner::Guild {
            guild_id,
            guild,
            logs,
            usage,
        } => {
            write(txn, GUILDS, guild_id, guild);
            write(txn, GUILD_BANK_LOGS, guild_id, logs);
            if let Some(usage) = usage {
                write(txn, GUILD_BANK_USAGE, character_id, usage);
            }
        }
    }
}

/// `GuildRankData` before `withdraw_repair` (guild rank layout version 0).
#[derive(bitcode::Encode, bitcode::Decode)]
struct GuildRankDataV0 {
    name: String,
    withdraw_gold: bool,
    gold_per_day: u64,
    tabs: Vec<GuildTabRights>,
}

#[derive(bitcode::Encode, bitcode::Decode)]
struct GuildDataV0 {
    ranks: Vec<GuildRankDataV0>,
    money: u64,
    tabs: Vec<BankTabData>,
}

/// `SERVER_SETTINGS` key of the `GuildData` layout version.
pub(crate) const GUILD_LAYOUT_KEY: &str = "guild_layout_version";
/// 3: TC right values; 2 added compact social rights, 1 added `withdraw_repair`.
pub(crate) const GUILD_LAYOUT_VERSION: u32 = 3;

fn default_social_rights(officer: bool) -> u32 {
    use shared::protocol::*;
    if officer {
        GUILD_RIGHT_ALL & !(GUILD_RIGHT_WITHDRAW_GOLD | GUILD_RIGHT_WITHDRAW_REPAIR)
    } else {
        GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_CHAT_SPEAK
    }
}

/// Layout 1 before player-managed social permissions.
#[derive(bitcode::Encode, bitcode::Decode)]
struct GuildRankDataV1 {
    name: String,
    withdraw_gold: bool,
    withdraw_repair: bool,
    gold_per_day: u64,
    tabs: Vec<GuildTabRights>,
}

#[derive(bitcode::Encode, bitcode::Decode)]
struct GuildDataV1 {
    ranks: Vec<GuildRankDataV1>,
    money: u64,
    tabs: Vec<BankTabData>,
}

impl From<GuildDataV1> for GuildData {
    fn from(old: GuildDataV1) -> Self {
        Self {
            ranks: old
                .ranks
                .into_iter()
                .enumerate()
                .map(|(index, rank)| GuildRankData {
                    name: rank.name,
                    rights: default_social_rights(index <= 1),
                    withdraw_gold: rank.withdraw_gold,
                    withdraw_repair: rank.withdraw_repair,
                    gold_per_day: rank.gold_per_day,
                    tabs: rank.tabs,
                })
                .collect(),
            money: old.money,
            tabs: old.tabs,
        }
    }
}

/// Persist the ordered rank list and all affected member positions atomically.
/// Callers materialize implicit lowest-rank members before adding/removing ranks.
pub fn save_guild_ranks(
    db: &Database,
    guild_id: u32,
    guild: &GuildData,
    members: &[(u64, u8)],
    event: Option<shared::protocol::GuildEventKind>,
) {
    let txn = db.begin_write().expect("Failed to begin guild ranks write");
    write(&txn, GUILDS, guild_id, guild);
    {
        let mut table = txn.open_table(GUILD_MEMBER_RANKS).expect("open ranks");
        for &(character, rank) in members {
            table.insert(character, rank).expect("write member rank");
        }
    }
    if let Some(event) = event {
        crate::guild_event_log::record_in(&txn, guild_id, event);
    }
    txn.commit()
        .expect("Failed to commit guild ranks and lifecycle event");
}

impl From<GuildDataV0> for GuildData {
    /// Version 0 ranks get `GR_RIGHT_WITHDRAW_REPAIR` as TrinityCore's default ranks
    /// hold it (`_CreateDefaultGuildRanks`, Guild.cpp:3184-3188): `GR_RIGHT_ALL` for the
    /// Guild Master (rank 0) and the Officer (rank 1), chat rights only below.
    fn from(old: GuildDataV0) -> Self {
        let ranks = old
            .ranks
            .into_iter()
            .enumerate()
            .map(|(index, rank)| GuildRankData {
                name: rank.name,
                rights: default_social_rights(index <= 1),
                withdraw_gold: rank.withdraw_gold,
                withdraw_repair: index <= 1,
                gold_per_day: rank.gold_per_day,
                tabs: rank.tabs,
            })
            .collect();
        Self {
            ranks,
            money: old.money,
            tabs: old.tabs,
        }
    }
}

/// Translate each layout-2 compact bit independently, including mixed officer masks.
/// Monetary booleans/allowances and all other guild data are already authoritative.
fn migrate_compact_rights(mut guild: GuildData) -> GuildData {
    use shared::protocol::*;
    let rights = [
        GUILD_RIGHT_CHAT_LISTEN,
        GUILD_RIGHT_CHAT_SPEAK,
        GUILD_RIGHT_OFFICER_LISTEN,
        GUILD_RIGHT_OFFICER_SPEAK,
        GUILD_RIGHT_INVITE,
        GUILD_RIGHT_REMOVE,
        GUILD_RIGHT_PROMOTE,
        GUILD_RIGHT_DEMOTE,
        GUILD_RIGHT_EDIT_MOTD,
        GUILD_RIGHT_EDIT_INFO,
        GUILD_RIGHT_EDIT_OFFICER_NOTE,
        GUILD_RIGHT_VIEW_OFFICER_NOTE,
        GUILD_RIGHT_WITHDRAW_GOLD,
        GUILD_RIGHT_WITHDRAW_REPAIR,
    ];
    for rank in &mut guild.ranks {
        let old = rank.rights;
        rank.rights = rights
            .iter()
            .enumerate()
            .filter(|(bit, _)| old & (1 << bit) != 0)
            .fold(0, |mask, (_, right)| mask | right);
    }
    guild
}

/// Rewrite stored guilds from layout 0/1/2 to 3 atomically, once.
pub(crate) fn migrate_guild_ranks(db: &Database) {
    let txn = db
        .begin_write()
        .expect("Failed to begin guild layout migration");
    let version = super::layout_version(&txn, GUILD_LAYOUT_KEY);
    if version >= GUILD_LAYOUT_VERSION {
        return;
    }
    let migrated = {
        let mut table = txn
            .open_table(GUILDS)
            .expect("Failed to open guilds for migration");
        let rows: Vec<(u32, GuildData)> = table
            .iter()
            .expect("Failed to iterate guilds")
            .map(|row| {
                let (key, value) = row.expect("Failed to read guild row");
                let guild = match version {
                    0 => bitcode::decode::<GuildDataV0>(value.value()).map(GuildData::from),
                    1 => bitcode::decode::<GuildDataV1>(value.value()).map(GuildData::from),
                    2 => bitcode::decode::<GuildData>(value.value()).map(migrate_compact_rights),
                    _ => unreachable!("unsupported guild layout {version}"),
                }
                .unwrap_or_else(|error| {
                    panic!("Failed to migrate guild {}: {error:?}", key.value())
                });
                (key.value(), guild)
            })
            .collect();
        for (guild_id, guild) in &rows {
            table
                .insert(*guild_id, bitcode::encode(guild).as_slice())
                .expect("Failed to write migrated guild");
        }
        rows.len()
    };
    super::set_layout_version(&txn, GUILD_LAYOUT_KEY, GUILD_LAYOUT_VERSION);
    txn.commit()
        .expect("Failed to commit guild layout migration");
    bevy::log::info!("Migrated {migrated} guilds to layout version {GUILD_LAYOUT_VERSION}");
}

pub(crate) fn create_bank_tables(txn: &WriteTransaction) {
    let _ = txn.open_table(CHARACTER_BANKS);
    let _ = txn.open_table(ACCOUNT_BANKS);
    let _ = txn.open_table(GUILDS);
    let _ = txn.open_table(GUILD_MEMBER_RANKS);
    let _ = txn.open_table(GUILD_BANK_USAGE);
    let _ = txn.open_table(GUILD_BANK_LOGS);
}

#[cfg(test)]
#[path = "persistence_banks_tests.rs"]
mod tests;
