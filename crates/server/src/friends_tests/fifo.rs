use super::*;
use lightyear::connection::host::HostClient;
use std::time::{Duration, Instant};

#[test]
fn friends_mutations_wait_for_fifo_and_preserve_fresh_rows() {
    let db = persistence::temp_database();
    persistence::create_character(&db, 2, "Alice", 1, 8).unwrap();
    persistence::create_character(&db, 3, "Bob", 1, 8).unwrap();
    persistence::save_friends(
        &db,
        42,
        &PlayerFriendsState {
            entries: vec![friend("Bob")],
        },
    );
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<QueryFriends>();
    app.register_message::<AddFriend>();
    app.register_message::<RemoveFriend>();
    app.register_message::<SetPresenceStatus>();
    app.register_message::<FriendsStateUpdate>();
    app.insert_resource(GameDatabase::new(db))
        .add_plugins(FriendsPlugin);
    app.finish();
    app.cleanup();
    let player = app
        .world_mut()
        .spawn((
            test_player("Fifofriend"),
            Level(10),
            PlayerFriendsState::default(),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            Connected,
            RemoteId(PeerId::Local(1)),
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld { character_id: 42 },
            PlayerLink(player),
            MessageSender::<AddFriend>::default(),
            MessageReceiver::<AddFriend>::default(),
            MessageSender::<RemoveFriend>::default(),
            MessageReceiver::<RemoveFriend>::default(),
            MessageSender::<FriendsStateUpdate>::default(),
            MessageReceiver::<FriendsStateUpdate>::default(),
        ))
        .id();
    app.update();
    app.update();
    drain_replies(&mut app, connection);
    let held = app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    app.world_mut()
        .get_mut::<MessageSender<AddFriend>>(connection)
        .unwrap()
        .send::<FriendsChannel>(AddFriend {
            name: "Alice".into(),
        });
    let started = Instant::now();
    app.update();
    app.update();
    app.world_mut()
        .get_mut::<MessageSender<RemoveFriend>>(connection)
        .unwrap()
        .send::<FriendsChannel>(RemoveFriend {
            name: "Alice".into(),
        });
    app.update();
    app.update();
    let elapsed = started.elapsed();
    let premature = drain_replies(&mut app, connection);
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "friends tick waited: {elapsed:?}"
    );
    assert!(premature.is_empty(), "friends reply precedes commit");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut replies = Vec::new();
    while replies
        .iter()
        .filter(|reply: &&FriendsStateUpdate| {
            reply.message.as_deref() == Some("friend added: Alice")
                || reply.message.as_deref() == Some("friend removed: Alice")
        })
        .count()
        < 2
    {
        app.update();
        replies.extend(drain_replies(&mut app, connection));
        assert!(Instant::now() < deadline, "missing durable friend replies");
        std::thread::sleep(Duration::from_millis(1));
    }
    let added = replies
        .iter()
        .find(|reply| reply.message.as_deref() == Some("friend added: Alice"))
        .unwrap();
    assert!(added.error.is_none());
    assert_eq!(
        added
            .snapshot
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Alice", "Bob"]
    );
    let removed = replies
        .iter()
        .find(|reply| reply.message.as_deref() == Some("friend removed: Alice"))
        .unwrap();
    assert!(removed.error.is_none());
    assert_eq!(
        removed
            .snapshot
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Bob"]
    );
    assert_eq!(
        persistence::load_friends(&app.world().resource::<GameDatabase>().db, 42).entries,
        vec![friend("Bob")]
    );
    assert_eq!(
        app.world()
            .get::<PlayerFriendsState>(player)
            .unwrap()
            .entries,
        vec![friend("Bob")]
    );
}

fn drain_replies(app: &mut App, connection: Entity) -> Vec<FriendsStateUpdate> {
    app.world_mut()
        .get_mut::<MessageReceiver<FriendsStateUpdate>>(connection)
        .unwrap()
        .receive()
        .collect()
}
