//! Real guild lifecycle history, distinct from bank logs and guild news.
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use shared::protocol::{
    GUILD_EVENT_LOG_MAX_RECORDS, GuildEvent, GuildEventKind, GuildEventLog, GuildRankError,
};

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

pub(crate) fn character_name_in(txn: &WriteTransaction, character: u64) -> String {
    let table = txn
        .open_table(crate::persistence::CHARACTERS)
        .expect("open event character records");
    let record = table
        .get(character)
        .expect("read event character")
        .expect("guild event requires a real character");
    crate::persistence_types::CharacterData::decode_compatible(record.value())
        .expect("decode event character")
        .name
}

pub(crate) fn record_in(txn: &WriteTransaction, guild: u32, kind: GuildEventKind) {
    append_in(
        txn,
        guild,
        GuildEvent {
            timestamp: unix_now(),
            kind,
        },
    );
}

pub(crate) fn append_in(txn: &WriteTransaction, guild: u32, event: GuildEvent) {
    let mut table = txn.open_table(EVENTS).expect("open guild event history");
    let mut events: Vec<GuildEvent> = table
        .get(guild)
        .expect("read guild event history for append")
        .map(|record| serde_json::from_str(record.value()).expect("decode guild event history"))
        .unwrap_or_default();
    events.push(event);
    if events.len() > GUILD_EVENT_LOG_MAX_RECORDS {
        events.drain(..events.len() - GUILD_EVENT_LOG_MAX_RECORDS);
    }
    let encoded = serde_json::to_string(&events).expect("encode guild event history");
    table
        .insert(guild, encoded.as_str())
        .expect("write guild event history");
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("guild event clock precedes Unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_lifecycle_retains_hundred_tied_events_across_reopen_and_rollback() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("retained-events.redb");
        let db = Database::create(&path).unwrap();
        crate::persistence::create_tables(&db);
        let event = |index| GuildEvent {
            timestamp: 100,
            kind: GuildEventKind::Promote {
                actor: "Master".into(),
                member: "Visitor".into(),
                rank: format!("Rank{index}"),
            },
        };
        let txn = db.begin_write().unwrap();
        for index in 0..103 {
            append_in(&txn, 7, event(index));
        }
        txn.commit().unwrap();
        let expected = (3..103).map(event).collect::<Vec<_>>();
        assert_eq!(load_events(&db, 7), expected);
        assert!(load_events(&db, 9).is_empty());
        let txn = db.begin_write().unwrap();
        append_in(&txn, 7, event(103));
        txn.abort().unwrap();
        assert_eq!(
            load_events(&db, 7),
            expected,
            "Aborted mutation cannot leak a log row"
        );
        drop(db);
        let db = Database::create(&path).unwrap();
        crate::persistence::create_tables(&db);
        assert_eq!(load_events(&db, 7), expected);
    }
}
