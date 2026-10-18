use super::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Connected, PeerId, RemoteId};
use std::time::{Duration, Instant};

#[derive(Resource, Default)]
struct LoginReplies(Vec<LoginResponse>);

fn collect_login_replies(
    mut receivers: Query<&mut MessageReceiver<LoginResponse>>,
    mut replies: ResMut<LoginReplies>,
) {
    for mut receiver in &mut receivers {
        replies.0.extend(receiver.receive());
    }
}

fn password_login_app(db: GameDatabase) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<LoginRequest>();
    app.register_message::<LoginResponse>();
    app.insert_resource(db);
    app.init_resource::<ConnectedAccounts>();
    app.init_resource::<LoginReplies>();
    app.add_systems(Update, process_login_requests);
    app.add_systems(Last, collect_login_replies);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            PendingAuth,
            PendingAuthTimer(0.0),
            MessageSender::<LoginRequest>::default(),
            MessageReceiver::<LoginRequest>::default(),
            MessageSender::<LoginResponse>::default(),
            MessageReceiver::<LoginResponse>::default(),
        ))
        .id();
    app.world_mut()
        .get_mut::<MessageSender<LoginRequest>>(connection)
        .unwrap()
        .send::<AuthChannel>(LoginRequest {
            token: None,
            username: "coldpassword".into(),
            password: "fixture-password".into(),
        });
    (app, connection)
}

#[test]
fn password_login_returns_while_writer_is_held_and_publishes_only_a_durable_token() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let raw = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&raw);
    let (account, _) =
        persistence_accounts::register_account(&raw, "coldpassword", "fixture-password").unwrap();
    let db = GameDatabase::new(raw);
    let held = db.db.begin_write().unwrap();
    let (release, blocked) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let explicitly_released = blocked.recv_timeout(Duration::from_secs(5)).is_ok();
        held.abort().unwrap();
        explicitly_released
    });
    let (mut app, connection) = password_login_app(db);
    app.update();
    app.update();
    assert!(
        !holder.is_finished(),
        "password login blocked on token persistence"
    );
    assert!(app.world().get::<Authenticated>(connection).is_none());
    assert!(app.world().resource::<ConnectedAccounts>().0.is_empty());
    assert!(app.world().resource::<LoginReplies>().0.is_empty());
    release.send(()).unwrap();
    assert!(
        holder.join().unwrap(),
        "writer watchdog is not a success condition"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.world().resource::<LoginReplies>().0.is_empty() {
        assert!(
            Instant::now() < deadline,
            "committed token was never published"
        );
        app.update();
        std::thread::yield_now();
    }
    let replies = &app.world().resource::<LoginReplies>().0;
    assert_eq!(replies.len(), 1);
    assert!(replies[0].success, "{:?}", replies[0].error);
    let token = replies[0].token.clone();
    assert_eq!(
        app.world()
            .get::<Authenticated>(connection)
            .unwrap()
            .account_id,
        account
    );
    drop(app);
    let reopened = redb::Database::open(file.path()).unwrap();
    let authenticated =
        persistence_accounts::login_with_credentials(&reopened, Some(&token), "", "").unwrap();
    assert_eq!(authenticated, (account, token));
}
