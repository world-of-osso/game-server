//! Mainline guild news; current membership scopes every query.
use redb::Database;
use shared::protocol::{GuildNewsError, GuildNewsState};

pub(crate) fn query(db: &Database, actor: u64) -> GuildNewsState {
    let membership = crate::persistence::load_character_guild(db, actor);
    GuildNewsState {
        entries: vec![],
        server_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("guild news clock precedes Unix epoch")
            .as_secs(),
        error: membership.is_none().then_some(GuildNewsError::NotInGuild),
    }
}
