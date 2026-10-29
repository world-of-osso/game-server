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
fn password_verification_does_not_block_server_updates() {
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::{Algorithm, Argon2, Params, Version};

    let file = tempfile::NamedTempFile::new().unwrap();
    let raw = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&raw);
    let (account, _) =
        persistence_accounts::register_account(&raw, "coldpassword", "fixture-password").unwrap();
    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19_456, 60, 1, None).unwrap(),
    );
    let salt = SaltString::encode_b64(b"login-stall-test").unwrap();
    let hash = argon
        .hash_password(b"fixture-password", &salt)
        .unwrap()
        .to_string();
    let txn = raw.begin_write().unwrap();
    {
        let mut passwords = txn.open_table(persistence::PASSWORDS).unwrap();
        passwords.insert(account, hash.as_bytes()).unwrap();
    }
    txn.commit().unwrap();
    let start = Instant::now();
    persistence_accounts::validate_login_credentials(
        &raw,
        None,
        "coldpassword",
        "fixture-password",
    )
    .unwrap();
    let verification_time = start.elapsed();
    let (mut app, connection) = password_login_app(GameDatabase::new(raw));
    let start = Instant::now();
    app.update();
    app.update();
    let update_time = start.elapsed();
    assert!(
        update_time < verification_time / 2,
        "login update {update_time:?} blocked for credential verification {verification_time:?}"
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.world().resource::<LoginReplies>().0.is_empty() {
        assert!(Instant::now() < deadline, "login never completed");
        app.update();
        std::thread::yield_now();
    }
    assert!(app.world().resource::<LoginReplies>().0[0].success);
    assert_eq!(
        app.world()
            .get::<Authenticated>(connection)
            .unwrap()
            .account_id,
        account
    );
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

#[test]
fn abandoned_password_token_receipt_cannot_authenticate_another_connection() {
    for disconnect in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let raw = redb::Database::create(file.path()).unwrap();
        persistence::create_tables(&raw);
        persistence_accounts::register_account(&raw, "coldpassword", "fixture-password").unwrap();
        let db = GameDatabase::new(raw);
        let held = db.db.begin_write().unwrap();
        let (release, blocked) = std::sync::mpsc::channel();
        let holder = std::thread::spawn(move || {
            let signal = blocked.recv_timeout(Duration::from_secs(5));
            held.abort().unwrap();
            signal.expect("explicit writer release");
        });
        let (mut app, old) = password_login_app(db);
        app.update();
        app.update();
        assert!(!holder.is_finished());
        if disconnect {
            app.world_mut().entity_mut(old).remove::<Connected>();
        } else {
            app.world_mut().despawn(old);
        }
        let replacement = app
            .world_mut()
            .spawn((
                PendingAuth,
                MessageReceiver::<LoginRequest>::default(),
                MessageSender::<LoginResponse>::default(),
            ))
            .id();
        release.send(()).unwrap();
        holder.join().unwrap();
        crate::test_data::drain_persistence(app.world().resource::<GameDatabase>());
        for _ in 0..4 {
            app.update();
        }
        assert!(app.world().get::<Authenticated>(replacement).is_none());
        if disconnect {
            assert!(app.world().get::<Authenticated>(old).is_none());
        }
        assert!(app.world().resource::<ConnectedAccounts>().0.is_empty());
        assert!(app.world().resource::<LoginReplies>().0.is_empty());
    }
}

#[test]
fn password_token_receipt_reports_a_preceding_account_ban_without_authenticating() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let raw = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&raw);
    persistence_accounts::register_account(&raw, "coldpassword", "fixture-password").unwrap();
    let db = GameDatabase::new(raw);
    let held = db.db.begin_write().unwrap();
    let (release, blocked) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let signal = blocked.recv_timeout(Duration::from_secs(5));
        held.abort().unwrap();
        signal.expect("explicit writer release");
    });
    db.writer.enqueue("fixture account ban", |raw| {
        persistence_accounts::ban_account(raw, "coldpassword", "fixture ban").map(|_| ())
    });
    let (mut app, connection) = password_login_app(db);
    app.update();
    app.update();
    assert!(!holder.is_finished());
    assert!(app.world().resource::<LoginReplies>().0.is_empty());
    release.send(()).unwrap();
    holder.join().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.world().resource::<LoginReplies>().0.is_empty() {
        assert!(Instant::now() < deadline);
        app.update();
        std::thread::yield_now();
    }
    let replies = &app.world().resource::<LoginReplies>().0;
    assert_eq!(replies.len(), 1);
    assert!(!replies[0].success);
    assert_eq!(
        replies[0].error.as_deref(),
        Some("Account banned: fixture ban")
    );
    assert!(replies[0].token.is_empty());
    assert!(app.world().get::<Authenticated>(connection).is_none());
    assert!(app.world().resource::<ConnectedAccounts>().0.is_empty());
}
