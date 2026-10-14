//! The guild layout migration (version 0 -> 1: `GR_RIGHT_WITHDRAW_REPAIR`).
use super::*;
use crate::persistence::create_tables;
use redb::ReadableTable;

#[path = "guild_rights_migration_tests.rs"]
mod guild_rights_migration_tests;

/// `GuildBankLogKindData` before `RepairMoney`.
#[derive(bitcode::Encode)]
enum GuildBankLogKindV0 {
    DepositItem,
    WithdrawItem,
    DepositMoney,
    WithdrawMoney,
    BuyTab,
}

#[derive(bitcode::Encode)]
struct GuildBankLogDataV0 {
    kind: GuildBankLogKindV0,
    actor: String,
    item_id: u32,
    item_name: String,
    count: u32,
    copper: u64,
    time: u64,
}

#[derive(bitcode::Encode)]
struct GuildBankLogsV0 {
    tabs: Vec<Vec<GuildBankLogDataV0>>,
    money: Vec<GuildBankLogDataV0>,
}

fn money_entry_v0(kind: GuildBankLogKindV0, copper: u64) -> GuildBankLogDataV0 {
    GuildBankLogDataV0 {
        kind,
        actor: "Bankone".into(),
        item_id: 0,
        item_name: String::new(),
        count: 0,
        copper,
        time: 1_790_000_000,
    }
}

fn rank_v0(name: &str, withdraw_gold: bool, gold_per_day: u64) -> GuildRankDataV0 {
    GuildRankDataV0 {
        name: name.into(),
        withdraw_gold,
        gold_per_day,
        tabs: vec![
            GuildTabRights {
                view: true,
                deposit: false,
                withdrawals_per_day: 2,
            };
            shared::protocol::GUILD_BANK_MAX_TABS
        ],
    }
}

/// A database as the server wrote it before version 1: guild 7 has the default ranks,
/// the Member rank edited to withdraw 5g a day, 120g in the bank and one money log entry
/// of each kind.
fn version_0_database() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.redb");
    let db = Database::create(&path).unwrap();
    let guild = GuildDataV0 {
        ranks: vec![
            rank_v0("Guild Master", true, 0),
            rank_v0("Officer", true, 0),
            rank_v0("Veteran", false, 0),
            rank_v0("Member", true, 50_000),
            rank_v0("Initiate", false, 0),
        ],
        money: 1_200_000,
        tabs: Vec::new(),
    };
    let logs = GuildBankLogsV0 {
        tabs: Vec::new(),
        money: vec![
            money_entry_v0(GuildBankLogKindV0::DepositItem, 1),
            money_entry_v0(GuildBankLogKindV0::WithdrawItem, 2),
            money_entry_v0(GuildBankLogKindV0::DepositMoney, 2_200_000),
            money_entry_v0(GuildBankLogKindV0::WithdrawMoney, 3),
            money_entry_v0(GuildBankLogKindV0::BuyTab, 1_000_000),
        ],
    };
    let txn = db.begin_write().unwrap();
    write(&txn, GUILDS, 7, &guild);
    write(&txn, GUILD_BANK_LOGS, 7, &logs);
    crate::persistence_commit::commit(txn).unwrap();
    (dir, path)
}

fn repair_rights(guild: &GuildData) -> Vec<(&str, bool)> {
    guild
        .ranks
        .iter()
        .map(|rank| (rank.name.as_str(), rank.withdraw_repair))
        .collect()
}

#[test]
fn version_0_guilds_get_trinitycore_default_repair_rights_once() {
    let (_dir, path) = version_0_database();
    let db = Database::create(&path).unwrap();

    create_tables(&db);

    let guild = load_guild(&db, 7);
    assert_eq!(
        repair_rights(&guild),
        vec![
            ("Guild Master", true),
            ("Officer", true),
            ("Veteran", false),
            ("Member", false),
            ("Initiate", false),
        ]
    );
    // Everything else is kept.
    assert_eq!(guild.money, 1_200_000);
    assert!(guild.ranks[3].withdraw_gold);
    assert_eq!(guild.ranks[3].gold_per_day, 50_000);
    assert_eq!(guild.ranks[2].tabs[0].withdrawals_per_day, 2);
    // Logs written before the RepairMoney kind still read.
    let kinds: Vec<_> = load_guild_bank_logs(&db, 7)
        .money
        .iter()
        .map(|entry| (entry.kind, entry.copper))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (GuildBankLogKindData::DepositItem, 1),
            (GuildBankLogKindData::WithdrawItem, 2),
            (GuildBankLogKindData::DepositMoney, 2_200_000),
            (GuildBankLogKindData::WithdrawMoney, 3),
            (GuildBankLogKindData::BuyTab, 1_000_000)
        ]
    );

    // A later startup leaves version 1 rows alone, including an edited right.
    let mut edited = guild.clone();
    edited.ranks[3].withdraw_repair = true;
    save_guild(&db, 7, &edited);
    create_tables(&db);
    assert_eq!(load_guild(&db, 7), edited);
}

/// Run against a copy of a real database: `GAME_SERVER_MIGRATION_DB=<path to game.redb>
/// cargo test -p game-server -- --ignored canonical_game_database_migrates`. The file at
/// that path is copied first and never opened.
#[test]
#[ignore = "needs GAME_SERVER_MIGRATION_DB"]
fn canonical_game_database_migrates() {
    let source = std::env::var_os("GAME_SERVER_MIGRATION_DB").expect("GAME_SERVER_MIGRATION_DB");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.redb");
    std::fs::copy(&source, &path).unwrap();
    let db = Database::create(&path).unwrap();
    let before: Vec<(u32, GuildDataV0)> = {
        let txn = db.begin_read().unwrap();
        let table = txn.open_table(GUILDS).unwrap();
        table
            .iter()
            .unwrap()
            .map(|row| {
                let (key, value) = row.unwrap();
                (key.value(), bitcode::decode(value.value()).unwrap())
            })
            .collect()
    };

    create_tables(&db);

    for (guild_id, old) in &before {
        let guild = load_guild(&db, *guild_id);
        let logs = load_guild_bank_logs(&db, *guild_id);
        eprintln!(
            "guild {guild_id}: money {} ranks {:?}, {} money log entries",
            guild.money,
            repair_rights(&guild),
            logs.money.len()
        );
        assert_eq!(guild.money, old.money);
        assert_eq!(guild.tabs, old.tabs);
        assert_eq!(guild.ranks.len(), old.ranks.len());
        for (index, (rank, old)) in guild.ranks.iter().zip(&old.ranks).enumerate() {
            assert_eq!(rank.name, old.name);
            assert_eq!(rank.withdraw_gold, old.withdraw_gold);
            assert_eq!(rank.gold_per_day, old.gold_per_day);
            assert_eq!(rank.tabs, old.tabs);
            assert_eq!(rank.withdraw_repair, index <= 1);
        }
    }
    let txn = db.begin_read().unwrap();
    let count = |rows: redb::Range<'_, u64, _>| rows.count();
    let members = count(
        txn.open_table(crate::persistence::CHARACTER_GUILDS)
            .unwrap()
            .iter()
            .unwrap(),
    );
    let ranked = txn
        .open_table(GUILD_MEMBER_RANKS)
        .unwrap()
        .iter()
        .unwrap()
        .count();
    let logs = txn.open_table(GUILD_BANK_LOGS).unwrap();
    let mut entries = 0;
    for row in logs.iter().unwrap() {
        let (guild_id, _) = row.unwrap();
        let logs = load_guild_bank_logs(&db, guild_id.value());
        entries += logs.money.len() + logs.tabs.iter().map(Vec::len).sum::<usize>();
    }
    drop(txn);
    eprintln!(
        "{} stored guilds migrated; {members} guild members, {ranked} stored ranks, {entries} log entries read",
        before.len()
    );
    let txn = db.begin_write().unwrap();
    assert_eq!(
        crate::persistence::layout_version(&txn, GUILD_LAYOUT_KEY),
        GUILD_LAYOUT_VERSION
    );
}

#[test]
fn guild_ranks_layout_one_preserves_bank_allowances_on_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guild.redb");
    {
        let db = Database::create(&path).unwrap();
        let txn = db.begin_write().unwrap();
        let old = GuildDataV1 {
            ranks: vec![
                GuildRankDataV1 {
                    name: "Master".into(),
                    withdraw_gold: true,
                    withdraw_repair: true,
                    gold_per_day: 0,
                    tabs: vec![],
                },
                GuildRankDataV1 {
                    name: "Officer".into(),
                    withdraw_gold: false,
                    withdraw_repair: true,
                    gold_per_day: 12_345,
                    tabs: vec![GuildTabRights {
                        view: true,
                        deposit: false,
                        withdrawals_per_day: 3,
                    }],
                },
            ],
            money: 90_000,
            tabs: vec![],
        };
        write(&txn, GUILDS, 7, &old);
        super::super::set_layout_version(&txn, GUILD_LAYOUT_KEY, 1);
        crate::persistence_commit::commit(txn).unwrap();
    }
    let db = Database::create(&path).unwrap();
    create_tables(&db);
    let guild = load_guild(&db, 7);
    assert_eq!(guild.money, 90_000);
    assert_eq!(guild.ranks[1].gold_per_day, 12_345);
    assert!(!guild.ranks[1].withdraw_gold);
    assert!(guild.ranks[1].withdraw_repair);
    assert_eq!(guild.ranks[1].tabs[0].withdrawals_per_day, 3);
    assert_ne!(
        guild.ranks[1].rights & shared::protocol::GUILD_RIGHT_INVITE,
        0
    );
    create_tables(&db);
    assert_eq!(load_guild(&db, 7), guild);
}
