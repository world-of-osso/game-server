//! Bank storage: the character bank (by character), the Warband bank (by account),
//! guilds (ranks, bank tabs, money), guild member ranks, daily guild bank usage and
//! guild bank logs. Every bank move writes the bank and the owner's inventory in one
//! transaction (`save_bank_move`).
use redb::{Database, ReadableDatabase, TableDefinition, WriteTransaction};

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
    /// AzerothCore `GR_RIGHT_WITHDRAW_GOLD`.
    pub withdraw_gold: bool,
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
    /// AzerothCore `Guild::_CreateDefaultGuildRanks`: Guild Master and Officer get
    /// `GR_RIGHT_ALL` (gold withdrawal included), the others chat rights only; no
    /// rank but the Guild Master has bank tab rights or a gold allowance.
    fn default() -> Self {
        let rank = |name: &str, withdraw_gold: bool| GuildRankData {
            name: name.into(),
            withdraw_gold,
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
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq)]
pub struct GuildBankLogData {
    pub kind: GuildBankLogKindData,
    pub actor: String,
    pub item_id: u32,
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
    read(db, GUILDS, guild_id)
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
    let txn = db.begin_write().expect("Failed to begin bank write");
    match owner {
        BankOwner::Character(id, bank) => write(&txn, CHARACTER_BANKS, id, bank),
        BankOwner::Account(id, bank) => write(&txn, ACCOUNT_BANKS, id, bank),
        BankOwner::Guild {
            guild_id,
            guild,
            logs,
            usage,
        } => {
            write(&txn, GUILDS, guild_id, guild);
            write(&txn, GUILD_BANK_LOGS, guild_id, logs);
            if let Some(usage) = usage {
                write(&txn, GUILD_BANK_USAGE, character_id, usage);
            }
        }
    }
    if let Some(record) = inventory {
        write_inventory_record(&txn, character_id, record);
    }
    txn.commit().expect("Failed to commit bank write");
}

pub(crate) fn create_bank_tables(txn: &WriteTransaction) {
    let _ = txn.open_table(CHARACTER_BANKS);
    let _ = txn.open_table(ACCOUNT_BANKS);
    let _ = txn.open_table(GUILDS);
    let _ = txn.open_table(GUILD_MEMBER_RANKS);
    let _ = txn.open_table(GUILD_BANK_USAGE);
    let _ = txn.open_table(GUILD_BANK_LOGS);
}
