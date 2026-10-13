use crate::auth::{Authenticated, InWorld};
use crate::persistence::{self, GameDatabase};
use bevy::prelude::*;
use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use shared::protocol::{AuthChannel, EnterWorldResponse, LoadTerrain, SelectCharacter};
use std::io::Write;
use std::time::{Duration, Instant};

fn admission_app(data: &std::path::Path) -> (tempfile::NamedTempFile, App, Entity) {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let (character_id, _) = persistence::create_character(&db, 1, "Loadproof", 1, 1).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.register_message::<SelectCharacter>();
    app.register_message::<EnterWorldResponse>();
    app.register_message::<LoadTerrain>();
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.insert_resource(GameDatabase { db });
    app.insert_resource(crate::maps::MapStore::continents());
    app.init_resource::<crate::maps::InstanceManager>();
    app.insert_resource(crate::ground::GroundMap::new(
        data,
        [(0, 999)].into_iter().collect(),
    ));
    app.add_systems(Update, crate::auth_character::process_select_character);
    app.finish();
    app.cleanup();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            MessageSender::<SelectCharacter>::default(),
            MessageReceiver::<SelectCharacter>::default(),
            MessageSender::<EnterWorldResponse>::default(),
            MessageReceiver::<EnterWorldResponse>::default(),
            MessageSender::<LoadTerrain>::default(),
            MessageReceiver::<LoadTerrain>::default(),
        ))
        .id();
    app.world_mut()
        .get_mut::<MessageSender<SelectCharacter>>(connection)
        .unwrap()
        .send::<AuthChannel>(SelectCharacter { character_id });
    (file, app, connection)
}

fn finish_admission(app: &mut App, connection: Entity) -> Vec<EnterWorldResponse> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        app.update();
        let replies: Vec<_> = app
            .world_mut()
            .get_mut::<MessageReceiver<EnterWorldResponse>>(connection)
            .unwrap()
            .receive()
            .collect();
        if !replies.is_empty() {
            return replies;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("admission worker failed to finish; deadline never admits unready geometry");
}

fn empty_wdt() -> Vec<u8> {
    let mut bytes = Vec::new();
    for (tag, payload) in [(b"MPHD", vec![0; 32]), (b"MAID", vec![0; 64 * 64 * 32])] {
        bytes.extend(tag.iter().rev());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend(payload);
    }
    bytes
}

fn release_fifo(path: &std::path::Path) -> bool {
    use std::os::unix::fs::OpenOptionsExt;
    const O_NONBLOCK: i32 = 2048;
    const ENXIO: i32 = 6; // FIFO has no reader yet.
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        match std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(path)
        {
            Ok(mut file) => {
                let bytes = empty_wdt();
                let mut written = 0;
                while written < bytes.len() {
                    match file.write(&bytes[written..]) {
                        Ok(count) => written += count,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(1))
                        }
                        Err(error) => panic!("release WDT reader: {error}"),
                    }
                }
                return true;
            }
            Err(error) if error.raw_os_error() == Some(ENXIO) => {
                std::thread::sleep(Duration::from_millis(1))
            }
            Err(error) => panic!("open WDT writer: {error}"),
        }
    }
    false
}

#[test]
fn admission_waits_for_blocked_geometry_io_without_blocking_updates() {
    let data = tempfile::tempdir().unwrap();
    std::fs::create_dir(data.path().join("terrain")).unwrap();
    let fifo = data.path().join("terrain/999.wdt");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let (_file, mut app, connection) = admission_app(data.path());
    app.update();
    app.update();
    let waiting = app.world().get::<InWorld>(connection).is_none();
    let early_replies = app
        .world_mut()
        .get_mut::<MessageReceiver<EnterWorldResponse>>(connection)
        .unwrap()
        .receive()
        .count();
    let start = Instant::now();
    for _ in 0..10 {
        app.update();
    }
    let elapsed = start.elapsed();
    let still_waiting = app.world().get::<InWorld>(connection).is_none();
    // Always release a live reader before assertions/World teardown, even if a
    // regression mistakenly admitted early; do not leave a blocked worker behind.
    let released = release_fifo(&fifo);
    assert!(
        waiting && still_waiting,
        "no admission while the WDT reader is blocked"
    );
    assert_eq!(early_replies, 0);
    assert!(
        elapsed < Duration::from_millis(50),
        "a blocked file read must not occupy the tick thread: {elapsed:?}"
    );
    assert!(released, "geometry loader must read the actual WDT");
    let replies = finish_admission(&mut app, connection);
    assert!(replies[0].success, "{:?}", replies[0]);
    assert!(app.world().get::<InWorld>(connection).is_some());
}

#[test]
fn admission_completes_when_the_map_cell_has_no_tile_data() {
    let data = tempfile::tempdir().unwrap();
    let (_file, mut app, connection) = admission_app(data.path());
    let replies = finish_admission(&mut app, connection);
    assert!(replies[0].success, "{:?}", replies[0]);
    assert!(app.world().get::<InWorld>(connection).is_some());
}

#[test]
fn admission_load_errors_fail_explicitly_instead_of_admitting_unready_geometry() {
    let data = tempfile::tempdir().unwrap();
    std::fs::create_dir(data.path().join("terrain")).unwrap();
    std::fs::write(data.path().join("terrain/999.wdt"), b"invalid WDT").unwrap();
    let (_file, mut app, connection) = admission_app(data.path());
    let replies = finish_admission(&mut app, connection);
    assert!(!replies[0].success);
    assert!(
        replies[0]
            .error
            .as_ref()
            .is_some_and(|error| error.contains("geometry"))
    );
    assert!(app.world().get::<InWorld>(connection).is_none());
}
