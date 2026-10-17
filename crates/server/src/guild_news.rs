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

pub(crate) fn apply_request(
    db: &Database,
    actor: u64,
    request: shared::protocol::GuildNewsRequest,
) -> GuildNewsState {
    let error = match request {
        shared::protocol::GuildNewsRequest::Query => None,
        shared::protocol::GuildNewsRequest::SetSticky { id, sticky } => {
            set_sticky(db, actor, id, sticky).err()
        }
    };
    let mut reply = query(db, actor);
    if error.is_some() {
        reply.error = error;
    }
    reply
}

fn set_sticky(db: &Database, actor: u64, id: u64, sticky: bool) -> Result<(), GuildNewsError> {
    let guild = crate::persistence::load_character_guild(db, actor)
        .ok_or(GuildNewsError::NotInGuild)?
        .guild_id;
    if !crate::guild::character_has_right(db, actor, guild, shared::protocol::GUILD_RIGHT_EDIT_MOTD)
    {
        return Err(GuildNewsError::Permissions);
    }
    let txn = db.begin_write().expect("begin guild news sticky write");
    let mut history = read_history_in(&txn, guild);
    let entry = history
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or(GuildNewsError::EntryNotFound)?;
    entry.sticky = sticky;
    write_history_in(&txn, guild, &history);
    txn.commit().expect("commit guild news sticky write");
    Ok(())
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
    let mut history = read_history_in(txn, guild);
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
    write_history_in(txn, guild, &history);
}

fn read_history_in(txn: &WriteTransaction, guild: u32) -> History {
    let table = txn.open_table(NEWS).expect("open guild news for write");
    table
        .get(guild)
        .expect("read guild news for write")
        .map(|record| serde_json::from_str(record.value()).expect("decode guild news"))
        .unwrap_or_default()
}

fn write_history_in(txn: &WriteTransaction, guild: u32, history: &History) {
    let encoded = serde_json::to_string(history).expect("encode guild news");
    txn.open_table(NEWS)
        .expect("open guild news for write")
        .insert(guild, encoded.as_str())
        .expect("write guild news");
}

pub(crate) enum ItemNewsSource {
    Looted,
    Crafted,
    Purchased,
}

/// TC Player.cpp23904: committed vendor purchase uses the template's base level.
pub(crate) fn record_purchase(
    db: &Database,
    character: u64,
    item_id: u32,
    template: &crate::item_templates::ItemTemplate,
) {
    record_item(
        db,
        character,
        item_id,
        template,
        None,
        ItemNewsSource::Purchased,
    );
}

/// Call only after the real item mutation commits, never for inventory moves.
pub(crate) fn record_item(
    db: &Database,
    character: u64,
    item_id: u32,
    template: &crate::item_templates::ItemTemplate,
    scaling: Option<&shared::item_scaling::ItemScalingTables>,
    source: ItemNewsSource,
) {
    const EPIC: u8 = 4;
    const MIN_NEWS_ITEM_LEVEL: u16 = 353; // TC Guild.h260.
    if template.scaling.quality < EPIC {
        return;
    }
    let Some(membership) = crate::persistence::load_character_guild(db, character) else {
        return;
    };
    if template.scaling.quality == EPIC {
        let level = match source {
            ItemNewsSource::Purchased => template.scaling.item_level,
            _ => scaling
                .expect("guild loot/craft news requires installed item scaling")
                .item_level(&template.scaling),
        };
        if level < MIN_NEWS_ITEM_LEVEL {
            return;
        }
    }
    persist_item(db, character, membership.guild_id, item_id, source);
}

fn persist_item(db: &Database, character: u64, guild: u32, item_id: u32, source: ItemNewsSource) {
    let txn = db.begin_write().expect("begin actual guild item news");
    let member = crate::guild_event_log::character_name_in(&txn, character);
    let kind = match source {
        ItemNewsSource::Looted => GuildNewsKind::ItemLooted { member, item_id },
        ItemNewsSource::Crafted => GuildNewsKind::ItemCrafted { member, item_id },
        ItemNewsSource::Purchased => GuildNewsKind::ItemPurchased { member, item_id },
    };
    append_in(&txn, guild, kind, crate::guild_bank::unix_now(), false);
    txn.commit().expect("commit actual guild item news");
}

/// Called inside the transaction saving newly earned character/account achievements.
pub(crate) fn record_achievements_in(
    txn: &WriteTransaction,
    character: u64,
    timestamp: i64,
    achievements: impl Iterator<Item = (GuildNewsKind, bool)>,
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
    for (mut kind, sticky) in achievements {
        match &mut kind {
            GuildNewsKind::PlayerAchievement { member: actor, .. } => *actor = member.clone(),
            _ => panic!("account achievement news must describe a player achievement"),
        }
        append_in(txn, guild, kind, timestamp, sticky);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::GuildNewsRequest;

    /// Offline prerequisites only: no news, inventory, guild or earned-state writes.
    #[test]
    #[ignore = "private5891 runtime setup; requires GUILD_NEWS_PRIVATE_DB"]
    fn guild_news_private_lillehoff_prerequisites() {
        let path = std::env::var("GUILD_NEWS_PRIVATE_DB").expect("explicit private redb");
        assert!(path.starts_with("/home/osso/.worktrees/logs/guild-news-"));
        let db = Database::open(path).expect("open stopped private server database");
        let ids = ["Gcnewslead", "Gcnewsbuyer", "Gcnewsother"].map(|name| {
            crate::persistence_character::find_character_id_by_name(&db, name)
                .expect("existing private test character")
        });
        assert_eq!(ids.len(), 3, "only the three private news characters");
        for id in ids {
            let mut reputation = shared::reputation::CharacterReputation::default();
            reputation.set_rep(
                1119,
                shared::reputation::threshold_for_standing(shared::reputation::Standing::Exalted),
            );
            crate::persistence::save_reputation(&db, id, &reputation);
            crate::maps::persistence::save_character_map(&db, id, 571);
            assert!(
                query(&db, id).entries.is_empty(),
                "setup never fabricates news"
            );
        }
    }

    #[derive(bevy::prelude::Resource, Default)]
    struct Inbox(Vec<GuildNewsState>);

    fn collect_news(
        mut receivers: bevy::prelude::Query<
            &mut lightyear::prelude::MessageReceiver<GuildNewsState>,
        >,
        mut inbox: bevy::prelude::ResMut<Inbox>,
    ) {
        for mut receiver in &mut receivers {
            inbox.0.extend(receiver.receive());
        }
    }

    #[test]
    fn guild_news_authenticated_host_query_refusal_and_committed_live_update() {
        use bevy::prelude::*;
        use lightyear::prelude::*;
        use shared::protocol::*;
        let db = crate::persistence::temp_database();
        let (id, _) = crate::persistence::create_character(&db, 1, "Member", 1, 1).unwrap();
        crate::persistence_character::set_character_guild_by_id(&db, id, 7, "News Guild").unwrap();
        crate::persistence::banks::save_guild_member_rank(&db, id, 4);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: std::time::Duration::from_millis(50),
        });
        app.add_plugins(ProtocolPlugin);
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.init_resource::<Inbox>();
        app.add_plugins(crate::guild::GuildPlugin);
        app.add_systems(First, collect_news);
        app.finish();
        app.cleanup();
        let connection = app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(1)),
                Connected,
                lightyear::connection::host::HostClient { buffer: Vec::new() },
                crate::auth::InWorld { character_id: id },
                MessageSender::<GuildNewsRequest>::default(),
                MessageReceiver::<GuildNewsRequest>::default(),
                MessageSender::<GuildNewsState>::default(),
                MessageReceiver::<GuildNewsState>::default(),
            ))
            .id();
        app.world_mut()
            .get_mut::<MessageSender<GuildNewsRequest>>(connection)
            .unwrap()
            .send::<GuildChannel>(GuildNewsRequest::Query);
        for _ in 0..4 {
            app.update();
        }
        assert!(
            app.world().resource::<Inbox>().0.is_empty(),
            "unauthenticated requests stay unread"
        );
        app.world_mut()
            .entity_mut(connection)
            .insert(crate::auth::Authenticated { account_id: 1 });
        app.world_mut()
            .get_mut::<MessageSender<GuildNewsRequest>>(connection)
            .unwrap()
            .send::<GuildChannel>(GuildNewsRequest::Query);
        for _ in 0..4 {
            app.update();
        }
        let replies = std::mem::take(&mut app.world_mut().resource_mut::<Inbox>().0);
        assert_eq!(replies.len(), 1, "authenticated query must answer");
        assert!(replies[0].entries.is_empty());
        let db = &app
            .world()
            .resource::<crate::persistence::GameDatabase>()
            .db;
        let template =
            crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap();
        record_purchase(
            db,
            id,
            50335,
            template
                .get(shared::item_data::ItemDefinitionSource::Retail, 50335)
                .unwrap(),
        );
        app.world_mut()
            .resource_mut::<crate::gold::GoldUpdateQueue>()
            .record_items(id);
        for _ in 0..4 {
            app.update();
        }
        let replies = std::mem::take(&mut app.world_mut().resource_mut::<Inbox>().0);
        assert_eq!(
            replies.len(),
            1,
            "committed acquisition broadcasts without another query"
        );
        let news_id = replies[0].entries[0].id;
        app.world_mut()
            .get_mut::<MessageSender<GuildNewsRequest>>(connection)
            .unwrap()
            .send::<GuildChannel>(GuildNewsRequest::SetSticky {
                id: news_id,
                sticky: true,
            });
        for _ in 0..4 {
            app.update();
        }
        let replies = std::mem::take(&mut app.world_mut().resource_mut::<Inbox>().0);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].error, Some(GuildNewsError::Permissions));
        assert!(!replies[0].entries[0].sticky);
    }

    #[test]
    fn guild_news_retention_sticky_permissions_and_abort_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("news-controls.redb");
        let db = Database::create(&path).unwrap();
        crate::persistence::create_tables(&db);
        let mut ids = Vec::new();
        for (name, guild, rank) in [("Master", 7, 0), ("Member", 7, 4), ("Outsider", 9, 0)] {
            let (id, _) = crate::persistence::create_character(&db, 1, name, 1, 1).unwrap();
            crate::persistence_character::set_character_guild_by_id(&db, id, guild, name).unwrap();
            crate::persistence::banks::save_guild_member_rank(&db, id, rank);
            ids.push(id);
        }
        let [master, member, outsider] = ids.try_into().unwrap();
        let (unguilded, _) =
            crate::persistence::create_character(&db, 2, "Unguilded", 1, 1).unwrap();
        let event = || GuildNewsKind::ItemLooted {
            member: "Member".into(),
            item_id: 61931,
        };
        let txn = db.begin_write().unwrap();
        for _ in 0..251 {
            append_in(&txn, 7, event(), 100, false);
        }
        txn.commit().unwrap();
        let expected = query(&db, member).entries;
        assert_eq!(expected.len(), 250);
        assert_eq!(
            (expected[0].id, expected[249].id),
            (2, 251),
            "Tied-time insertion order and stable identities"
        );
        let txn = db.begin_write().unwrap();
        append_in(&txn, 7, event(), 101, false);
        txn.abort().unwrap();
        assert_eq!(query(&db, member).entries, expected);
        let request = GuildNewsRequest::SetSticky {
            id: 2,
            sticky: true,
        };
        assert_eq!(
            apply_request(&db, member, request.clone()).error,
            Some(GuildNewsError::Permissions)
        );
        assert_eq!(
            query(&db, member).entries,
            expected,
            "Refusal cannot mutate history"
        );
        assert_eq!(
            apply_request(&db, outsider, request.clone()).error,
            Some(GuildNewsError::EntryNotFound)
        );
        let refused = apply_request(&db, unguilded, request.clone());
        assert_eq!(refused.error, Some(GuildNewsError::NotInGuild));
        assert!(refused.entries.is_empty());
        let changed = apply_request(&db, master, request);
        assert_eq!(changed.error, None);
        assert!(changed.entries[0].sticky);
        assert!(changed.entries[1..].iter().all(|entry| !entry.sticky));
        assert_eq!(
            changed
                .entries
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            expected.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            "Sticky changes flags, not stored order or identity"
        );
        assert_eq!(
            apply_request(
                &db,
                master,
                GuildNewsRequest::SetSticky {
                    id: 1,
                    sticky: true
                }
            )
            .error,
            Some(GuildNewsError::EntryNotFound)
        );
        drop(db);
        let db = Database::create(&path).unwrap();
        crate::persistence::create_tables(&db);
        assert_eq!(query(&db, member).entries, changed.entries);
        assert_eq!(
            apply_request(
                &db,
                master,
                GuildNewsRequest::SetSticky {
                    id: 2,
                    sticky: false
                }
            )
            .entries,
            expected
        );
    }
}
