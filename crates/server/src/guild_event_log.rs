//! Real guild lifecycle history, distinct from bank logs and guild news.
use redb::{Database, ReadableDatabase, TableDefinition};
use shared::protocol::{GuildEvent, GuildEventLog, GuildRankError};

pub(crate) const EVENTS: TableDefinition<u32, &str> = TableDefinition::new("guild_event_log");

pub(crate) fn query(db: &Database, actor: u64) -> GuildEventLog {
    let membership = crate::persistence::load_character_guild(db, actor);
    GuildEventLog {
        events: membership
            .as_ref()
            .map(|guild| load_events(db, guild.guild_id))
            .unwrap_or_default(),
        server_time: unix_now(),
        error: membership.is_none().then_some(GuildRankError::NotInGuild),
    }
}

pub(crate) fn load_events(db: &Database, guild_id: u32) -> Vec<GuildEvent> {
    let read = db.begin_read().expect("begin guild event history read");
    let table = read.open_table(EVENTS).expect("open guild event history");
    table
        .get(guild_id)
        .expect("read guild event history")
        .map(|record| serde_json::from_str(record.value()).expect("decode guild event history"))
        .unwrap_or_default()
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("guild event clock precedes Unix epoch")
        .as_secs()
}
