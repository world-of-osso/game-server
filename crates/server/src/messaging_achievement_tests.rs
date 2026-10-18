use super::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};
use crate::auth::{Authenticated, InWorld};
use lightyear::connection::host::HostClient;
use shared::components::{CharacterAppearance, Level, Npc};

#[path = "messaging_mount_special_tests.rs"]
mod mount_special;
#[path = "messaging_targeted_emote_tests.rs"]
mod targeted;

fn emote_app() -> (tempfile::NamedTempFile, App, Entity, Entity, u64) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 41, "Danceproof", 1, 1).unwrap();
    let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: std::time::Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    app.add_plugins(AchievementPlugin);
    app.insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    app.insert_resource(GameDatabase::new(db));
    app.init_resource::<ChatHistoryBuffer>();
    app.init_resource::<GroupRegistry>();
    app.init_resource::<crate::friends::AutoAfkScheduler>();
    app.add_systems(Update, process_chat_messages);
    app.finish();
    app.cleanup();
    app.update();
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Danceproof".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(1),
            ClientId(character),
            crate::maps::InMap::continent(0),
            PlayerAchievementState::default(),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            PlayerLink(player),
            Authenticated { account_id: 41 },
            RemoteId(PeerId::Netcode(41)),
            Connected,
            HostClient { buffer: Vec::new() },
            MessageReceiver::<ChatMessage>::default(),
            MessageReceiver::<EmoteIntent>::default(),
            MessageSender::<ChatMessage>::default(),
            MessageSender::<EmoteEvent>::default(),
            MessageReceiver::<EmoteEvent>::default(),
            MessageSender::<EmoteIntent>::default(),
        ))
        .id();
    (file, app, player, connection, character)
}

#[test]
fn achievements_criteria_native_dance_chat_and_intent_persist_without_idle_replay() {
    let (file, mut app, player, connection, character) = emote_app();
    let dance = || ChatMessage {
        sender: "spoof".into(),
        content: "/dance".into(),
        channel: ChatType::Say,
    };
    app.world_mut()
        .get_mut::<MessageSender<ChatMessage>>(connection)
        .unwrap()
        .send::<ChatChannel>(dance());
    for _ in 0..3 {
        app.update();
    }
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .contains_key(&19515)
    );
    app.world_mut().entity_mut(connection).insert(InWorld {
        character_id: character,
    });
    app.update();
    app.world_mut()
        .get_mut::<MessageSender<ChatMessage>>(connection)
        .unwrap()
        .send::<ChatChannel>(dance());
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&19515),
        Some(&1)
    );
    app.world_mut()
        .get_mut::<MessageSender<EmoteIntent>>(connection)
        .unwrap()
        .send::<ChatChannel>(EmoteIntent {
            emote: EmoteKind::Dance,
            target_entity: None,
        });
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&19515),
        Some(&2)
    );
    app.world_mut()
        .get_mut::<MessageSender<ChatMessage>>(connection)
        .unwrap()
        .send::<ChatChannel>(ChatMessage {
            content: "/kneel".into(),
            ..dance()
        });
    for _ in 0..3 {
        app.update();
    }
    let state = app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&19515), Some(&2));
    assert!(
        !state.criteria_progress.contains_key(&3238),
        "Kneel59 cannot impersonate Laugh60"
    );
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, character)
            .criteria_progress
            .get(&19515),
        Some(&2)
    );
}
