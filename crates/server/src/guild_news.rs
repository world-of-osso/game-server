//! Mainline guild news, independent of lifecycle and bank history.
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use shared::protocol::{
    GUILD_NEWS_MAX_RECORDS, GuildNewsEntry, GuildNewsError, GuildNewsKind, GuildNewsState,
};

pub(crate) const NEWS: TableDefinition<u32, &str> = TableDefinition::new("guild_news");

#[derive(Default, Serialize, Deserialize)]
struct History {
    next_id: u64,
    entries: Vec<GuildNewsEntry>,
}

pub(crate) fn query(db: &Database, actor: u64) -> GuildNewsState {
    let membership = crate::persistence::load_character_guild(db, actor);
    let entries = membership.map(|guild| load_entries(db, guild.guild_id));
    GuildNewsState {
        error: entries.is_none().then_some(GuildNewsError::NotInGuild),
        entries: entries.unwrap_or_default(),
        server_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("guild news clock precedes Unix epoch")
            .as_secs(),
    }
}

fn load_entries(db: &Database, guild: u32) -> Vec<GuildNewsEntry> {
    let txn = db.begin_read().expect("begin guild news read");
    let table = txn.open_table(NEWS).expect("open guild news");
    table
        .get(guild)
        .expect("read guild news")
        .map(|record| {
            serde_json::from_str::<History>(record.value())
                .expect("decode guild news")
                .entries
        })
        .unwrap_or_default()
}

pub(crate) fn append_in(
    txn: &WriteTransaction,
    guild: u32,
    kind: GuildNewsKind,
    timestamp: u64,
    sticky: bool,
) {
    let mut table = txn.open_table(NEWS).expect("open guild news for append");
    let mut history: History = table
        .get(guild)
        .expect("read guild news for append")
        .map(|record| serde_json::from_str(record.value()).expect("decode guild news"))
        .unwrap_or_default();
    history.next_id = history
        .next_id
        .checked_add(1)
        .expect("guild news ID exhausted");
    history.entries.push(GuildNewsEntry {
        id: history.next_id,
        timestamp,
        sticky,
        kind,
    });
    if history.entries.len() > GUILD_NEWS_MAX_RECORDS {
        history
            .entries
            .drain(..history.entries.len() - GUILD_NEWS_MAX_RECORDS);
    }
    let encoded = serde_json::to_string(&history).expect("encode guild news");
    table
        .insert(guild, encoded.as_str())
        .expect("write guild news");
}

/// Called inside the transaction saving newly earned character/account achievements.
pub(crate) fn record_achievements_in(
    txn: &WriteTransaction,
    character: u64,
    timestamp: i64,
    achievements: impl Iterator<Item = u32>,
    catalog: &crate::achievement::AchievementCatalog,
) {
    let table = txn
        .open_table(crate::persistence::CHARACTER_GUILDS)
        .expect("open news membership");
    let Some(record) = table.get(character).expect("read news membership") else {
        return;
    };
    let guild = bitcode::decode::<crate::persistence_types::GuildMembershipData>(record.value())
        .expect("decode news membership")
        .guild_id;
    let member = crate::guild_event_log::character_name_in(txn, character);
    let timestamp = u64::try_from(timestamp).expect("guild news timestamp precedes Unix epoch");
    for id in achievements {
        if let Some((kind, sticky)) = catalog.build_guild_news_achievement(id, &member) {
            append_in(txn, guild, kind, timestamp, sticky);
        }
    }
}
