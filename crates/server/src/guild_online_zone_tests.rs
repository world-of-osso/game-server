//! Real host-client QueryGuild delivery over authoritative terrain-area samples.
use super::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use shared::components::{CharacterAppearance, Position};
use shared::protocol::ProtocolPlugin;
use std::time::Duration;

fn catalog() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE area_table(ID INTEGER,ParentAreaID INTEGER,ContinentID INTEGER,
            Flags_0 INTEGER,FactionGroupMask INTEGER,Flags_1 INTEGER,AreaName_lang TEXT);
         INSERT INTO area_table VALUES (12,0,0,0,0,0,'Elwynn Forest'),
            (87,12,0,1073741824,0,0,'Goldshire'),(40,0,0,0,0,0,'Westfall');",
    )
    .unwrap();
    conn
}

fn spawn_member(app: &mut App, name: &str, guild_id: u32, position: Vec3) -> (Entity, u64) {
    let db = &app.world().resource::<GameDatabase>().db;
    let (id, _) = crate::persistence_character::create_character(db, 1, name, 1, 1).unwrap();
    crate::persistence_character::set_character_guild_by_id(db, id, guild_id, "Zone Guild")
        .unwrap();
    let actor = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(32),
            GuildMembership {
                guild_id,
                guild_name: "Zone Guild".into(),
            },
            Position {
                x: position.x,
                y: position.y,
                z: position.z,
            },
            crate::maps::InMap {
                map_id: 0,
                instance_id: 0,
            },
        ))
        .id();
    (actor, id)
}

fn query(app: &mut App, connection: Entity) -> GuildSnapshot {
    app.world_mut()
        .get_mut::<MessageSender<QueryGuild>>(connection)
        .unwrap()
        .send::<GuildChannel>(QueryGuild);
    for _ in 0..5 {
        app.update();
    }
    app.world_mut()
        .get_mut::<MessageReceiver<GuildStateUpdate>>(connection)
        .unwrap()
        .receive()
        .filter_map(|update| update.guild)
        .last()
        .expect("actual guild response")
}

#[test]
#[ignore = "behavioral RED: native guild query does not carry authoritative online zones"]
fn guild_online_zone_native_query_uses_parent_area_refreshes_and_preserves_privacy() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    let catalog = catalog();
    app.insert_resource(crate::area_table::load_area_table(&catalog).unwrap());
    let goldshire = Vec3::new(-9465.0, 63.0, 43.0);
    let westfall = Vec3::new(-10542.0, 40.0, -1069.0);
    app.insert_resource(crate::ground::GroundMap::with_area_samples(&[
        (0, goldshire, 87),
        (0, westfall, 40),
    ]));
    app.insert_resource(GameDatabase::new(crate::persistence::temp_database()));
    app.add_systems(Update, process_query_requests);
    app.finish();
    app.cleanup();
    app.update();
    let (cara, cara_id) = spawn_member(&mut app, "Cara", 7, goldshire);
    let (dora, _) = spawn_member(&mut app, "Dora", 7, westfall);
    let (_outsider, _) = spawn_member(&mut app, "Fae", 8, westfall);
    let (offline, _) = spawn_member(&mut app, "Eli", 7, westfall);
    app.world_mut().entity_mut(offline).remove::<Player>();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            PlayerLink(cara),
            Authenticated { account_id: 1 },
            InWorld {
                character_id: cara_id,
            },
            MessageSender::<QueryGuild>::default(),
            MessageReceiver::<QueryGuild>::default(),
            MessageSender::<GuildStateUpdate>::default(),
            MessageReceiver::<GuildStateUpdate>::default(),
        ))
        .id();
    app.update();
    let snapshot = query(&mut app, connection);
    assert_eq!(snapshot.members.len(), 3, "no other-guild disclosure");
    let member = |snapshot: &GuildSnapshot, name: &str| {
        serde_json::to_value(
            snapshot
                .members
                .iter()
                .find(|member| member.character_name == name)
                .unwrap(),
        )
        .unwrap()
    };
    assert_eq!(member(&snapshot, "Cara")["online_zone"], "Elwynn Forest");
    assert_eq!(member(&snapshot, "Dora")["online_zone"], "Westfall");
    assert!(member(&snapshot, "Eli")["online_zone"].is_null());
    *app.world_mut().get_mut::<Position>(dora).unwrap() = Position {
        x: goldshire.x,
        y: goldshire.y,
        z: goldshire.z,
    };
    assert_eq!(
        member(&query(&mut app, connection), "Dora")["online_zone"],
        "Elwynn Forest"
    );
    app.world_mut().entity_mut(dora).remove::<Position>();
    assert!(member(&query(&mut app, connection), "Dora")["online_zone"].is_null());
}
