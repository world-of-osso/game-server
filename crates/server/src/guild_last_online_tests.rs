//! The production Connected-removal observer must persist actual logout authority.
use super::*;
use crate::persistence::{ClientId, create_tables};
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use shared::components::{CharacterAppearance, Health, Level, Player, Position};
use shared::protocol::ProtocolPlugin;
use std::time::Duration;

#[derive(Resource, Default)]
struct Replies(Vec<GuildStateUpdate>);

fn collect(
    mut receivers: Query<&mut MessageReceiver<GuildStateUpdate>>,
    mut replies: ResMut<Replies>,
) {
    for mut receiver in &mut receivers {
        replies.0.extend(receiver.receive());
    }
}

fn server(path: &std::path::Path) -> App {
    let db = redb::Database::create(path).unwrap();
    create_tables(&db);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    let catalog = rusqlite::Connection::open_in_memory().unwrap();
    catalog.execute_batch("CREATE TABLE area_table(ID INTEGER,ParentAreaID INTEGER,ContinentID INTEGER,Flags_0 INTEGER,FactionGroupMask INTEGER,Flags_1 INTEGER,AreaName_lang TEXT);").unwrap();
    app.insert_resource(crate::area_table::load_area_table(&catalog).unwrap());
    app.insert_resource(crate::ground::GroundMap::with_area_samples(&[]));
    app.insert_resource(GameDatabase::new(db));
    app.init_resource::<Replies>();
    app.add_plugins(GuildPlugin);
    crate::networking::install_disconnect_fixture(&mut app);
    app.add_systems(Update, collect.after(process_query_requests));
    app.finish();
    app.cleanup();
    app.update();
    app
}

fn stored_member(app: &App, name: &str, guild_id: u32) -> u64 {
    let db = &app.world().resource::<GameDatabase>().db;
    let (id, _) = crate::persistence_character::create_character(db, 1, name, 1, 1).unwrap();
    crate::persistence_character::set_character_guild_by_id(db, id, guild_id, "Presence Guild")
        .unwrap();
    id
}

fn connect(app: &mut App, id: u64, name: &str) -> (Entity, Entity) {
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(id),
            Level(32),
            Position {
                x: 321.0,
                y: 63.0,
                z: 43.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            GuildMembership {
                guild_id: 7,
                guild_name: "Presence Guild".into(),
            },
            crate::maps::InMap::continent(0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld { character_id: id },
            PlayerLink(player),
            MessageSender::<QueryGuild>::default(),
            MessageReceiver::<QueryGuild>::default(),
            MessageSender::<GuildStateUpdate>::default(),
            MessageReceiver::<GuildStateUpdate>::default(),
            MessageSender::<shared::protocol::ProtocolFingerprint>::default(),
            MessageReceiver::<shared::protocol::ProtocolFingerprint>::default(),
        ))
        .id();
    app.update();
    (connection, player)
}

fn query(app: &mut App, connection: Entity) -> GuildSnapshot {
    app.world_mut().resource_mut::<Replies>().0.clear();
    app.world_mut()
        .get_mut::<MessageSender<QueryGuild>>(connection)
        .unwrap()
        .send::<GuildChannel>(QueryGuild);
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<Replies>()
        .0
        .pop()
        .and_then(|reply| reply.guild)
        .expect("authenticated real GuildChannel reply")
}

fn member(snapshot: &GuildSnapshot, name: &str) -> serde_json::Value {
    serde_json::to_value(
        snapshot
            .members
            .iter()
            .find(|member| member.character_name == name)
            .unwrap(),
    )
    .unwrap()
}

fn disconnect(app: &mut App, connection: Entity, player: Entity) -> (u64, u64) {
    let before = crate::rest::unix_secs();
    // Invoke the actual production observer; do not write a synthetic logout row.
    app.world_mut()
        .entity_mut(connection)
        .insert(lightyear::prelude::server::ClientOf)
        .remove::<Connected>();
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world().get_entity(player).is_err(),
        "real disconnect despawn completed"
    );
    (before, crate::rest::unix_secs())
}

#[test]
fn guild_last_online_real_disconnect_reopen_preserves_distinct_times_and_unknown_history() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("presence.redb");
    let mut app = server(&path);
    let viewer = stored_member(&app, "Viewer", 7);
    let cara = stored_member(&app, "Cara", 7);
    let dora = stored_member(&app, "Dora", 7);
    let legacy = stored_member(&app, "Legacy", 7);
    stored_member(&app, "Foreign", 8);
    // Rest rows are not logout provenance: admin set-rested also writes this timestamp.
    crate::persistence_progression::save_rest(
        &app.world().resource::<GameDatabase>().db,
        legacy,
        &crate::persistence_progression::RestData {
            rested_xp: 5,
            logout_unix_secs: 123,
            logout_resting: false,
        },
    );
    let (viewer_connection, _) = connect(&mut app, viewer, "Viewer");
    let (cara_connection, cara_player) = connect(&mut app, cara, "Cara");
    let (dora_connection, dora_player) = connect(&mut app, dora, "Dora");
    let starting = query(&mut app, viewer_connection);
    assert!(member(&starting, "Legacy")["last_logout_unix_secs"].is_null());
    let cara_window = disconnect(&mut app, cara_connection, cara_player);
    while crate::rest::unix_secs() <= cara_window.1 {
        std::thread::sleep(Duration::from_millis(25));
    }
    let dora_window = disconnect(&mut app, dora_connection, dora_player);
    drop(app);
    // A new app opens the same closed database, as a fresh server does.
    let mut app = server(&path);
    assert_eq!(
        crate::persistence::load_character(&app.world().resource::<GameDatabase>().db, cara)
            .unwrap()
            .x,
        321.0,
        "production live-character save ran before the logout authority check"
    );
    let (viewer_connection, _) = connect(&mut app, viewer, "Viewer");
    let snapshot = query(&mut app, viewer_connection);
    assert_eq!(snapshot.members.len(), 4, "no foreign-guild disclosure");
    for (name, window) in [("Cara", cara_window), ("Dora", dora_window)] {
        let row = member(&snapshot, name);
        assert_eq!(row["is_online"], false);
        let observed = row["last_logout_unix_secs"]
            .as_u64()
            .expect("real disconnect timestamp survives server/database reopen");
        assert!(
            (window.0..=window.1).contains(&observed),
            "recorded at actual disconnect, never query/startup time"
        );
    }
    assert!(
        member(&snapshot, "Cara")["last_logout_unix_secs"]
            .as_u64()
            .unwrap()
            < member(&snapshot, "Dora")["last_logout_unix_secs"]
                .as_u64()
                .unwrap()
    );
    assert!(
        member(&snapshot, "Legacy")["last_logout_unix_secs"].is_null(),
        "unknown history never copied from a rest/admin timestamp"
    );
    assert_eq!(
        query(&mut app, viewer_connection),
        snapshot,
        "querying does not restamp history"
    );
    connect(&mut app, cara, "Cara");
    let online = member(&query(&mut app, viewer_connection), "Cara");
    assert_eq!(online["is_online"], true);
    assert!(
        online["last_logout_unix_secs"].is_null(),
        "online response does not expose stale offline recency"
    );
}
