use super::*;
use lightyear::connection::host::HostClient;
use std::time::{Duration, Instant};

#[test]
fn password_login_completion_rechecks_bans_and_password_rotation() {
    use redb::ReadableDatabase;
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (account, _) =
        persistence_accounts::register_account(&db, "fb_perf_revoke", "fbtest").unwrap();
    let prepare = || {
        let persistence_accounts::PreparedLogin::Password(prepared) =
            persistence_accounts::prepare_login(&db, None, "fb_perf_revoke").unwrap()
        else {
            panic!("password preparation")
        };
        prepared.verify("fbtest").unwrap()
    };
    let verified = prepare();
    persistence_accounts::ban_account(&db, "fb_perf_revoke", "fixture ban").unwrap();
    assert!(
        persistence_accounts::complete_password_login(&db, verified)
            .unwrap_err()
            .contains("banned")
    );
    persistence_accounts::unban_account(&db, "fb_perf_revoke").unwrap();
    let verified = prepare();
    let (other, _) =
        persistence_accounts::register_account(&db, "fb_perf_rotated", "newtest").unwrap();
    let hash = db
        .begin_read()
        .unwrap()
        .open_table(persistence::PASSWORDS)
        .unwrap()
        .get(other)
        .unwrap()
        .unwrap()
        .value()
        .to_vec();
    let tx = db.begin_write().unwrap();
    tx.open_table(persistence::PASSWORDS)
        .unwrap()
        .insert(account, hash.as_slice())
        .unwrap();
    tx.commit().unwrap();
    assert!(
        persistence_accounts::complete_password_login(&db, verified)
            .unwrap_err()
            .contains("Password changed")
    );
}

#[test]
fn password_login_waits_for_worker_before_authentication_and_roster_reply() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (account, _) =
        persistence_accounts::register_account(&db, "fb_perf_auth", "fbtest").unwrap();
    let (character, _) = persistence::create_character(&db, account, "Authproof", 1, 1).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<LoginRequest>();
    app.register_message::<LoginResponse>();
    app.register_message::<ForcedDisconnect>();
    app.init_resource::<ConnectedAccounts>();
    app.insert_resource(GameDatabase { db });
    app.add_systems(Update, process_login_requests);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            Connected,
            RemoteId(PeerId::Local(account)),
            HostClient { buffer: Vec::new() },
            PendingAuth,
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
            username: "fb_perf_auth".into(),
            password: "fbtest".into(),
        });
    app.update();
    app.update();
    assert!(
        app.world().get::<Authenticated>(connection).is_none(),
        "password verification cannot authenticate inline on the tick thread"
    );
    let start = Instant::now();
    let reply = loop {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "password worker failed to finish"
        );
        app.update();
        if let Some(reply) = app
            .world_mut()
            .get_mut::<MessageReceiver<LoginResponse>>(connection)
            .unwrap()
            .receive()
            .next()
        {
            break reply;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    assert!(reply.success, "{:?}", reply.error);
    assert_eq!(reply.characters[0].character_id, character);
    assert_eq!(
        app.world()
            .get::<Authenticated>(connection)
            .unwrap()
            .account_id,
        account
    );
    assert_eq!(
        persistence_accounts::find_account_by_token(
            &app.world().resource::<GameDatabase>().db,
            &reply.token
        ),
        Some(account)
    );
}
