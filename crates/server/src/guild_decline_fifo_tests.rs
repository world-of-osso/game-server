use super::*;
use crate::networking::PlayerLink;
use lightyear::prelude::*;
use std::time::{Duration, Instant};

struct Fixture {
    app: App,
    character: u64,
    sibling: u64,
    connection: Entity,
    directory: tempfile::TempDir,
}

fn fixture(pending: bool) -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("decline.redb")).unwrap();
    persistence::create_tables(&db);
    let character = persistence::create_character(&db, 2, "Invitee", 1, 1)
        .unwrap()
        .0;
    let sibling = persistence::create_character(&db, 2, "Sibling", 1, 1)
        .unwrap()
        .0;
    if pending {
        persistence::set_pending_guild_invite(&db, character, 7, "Raid Team", "Inviter");
    }
    persistence::set_pending_guild_invite(&db, sibling, 9, "Other Guild", "Other");
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.insert_resource(GameDatabase::new(db));
    app.add_plugins(crate::guild::GuildPlugin);
    app.finish();
    app.cleanup();
    let player = app.world_mut().spawn_empty().id();
    let connection = app
        .world_mut()
        .spawn((
            Authenticated { account_id: 2 },
            InWorld {
                character_id: character,
            },
            PlayerLink(player),
            RemoteId(PeerId::Local(2)),
            Connected,
            lightyear::connection::host::HostClient { buffer: Vec::new() },
            MessageSender::<GuildDeclineInviteRequest>::default(),
            MessageReceiver::<GuildDeclineInviteRequest>::default(),
            MessageSender::<GuildInvitation>::default(),
            MessageReceiver::<GuildInvitation>::default(),
        ))
        .id();
    let mut f = Fixture {
        app,
        character,
        sibling,
        connection,
        directory,
    };
    let initial = collect_frames(&mut f, 6);
    assert_eq!(initial.len(), usize::from(pending));
    f
}

fn collect_frames(f: &mut Fixture, frames: usize) -> Vec<GuildInvitation> {
    let mut invites = Vec::new();
    for _ in 0..frames {
        f.app.update();
        invites.extend(
            f.app
                .world_mut()
                .get_mut::<MessageReceiver<GuildInvitation>>(f.connection)
                .unwrap()
                .receive(),
        );
    }
    invites
}

#[derive(Clone, Copy)]
enum Case {
    Pending,
    Missing,
    PrefixReplacement,
    PrefixCreation,
    AcceptAfterDecline,
    Relog,
    StorageFailure,
}

#[test]
fn guild_decline_pending_invite_waits_for_fifo_without_blocking_ticks() {
    assert_decline(Case::Pending);
}
#[test]
fn guild_decline_missing_invite_does_not_wait_for_a_writer_on_tick() {
    assert_decline(Case::Missing);
}
#[test]
fn guild_decline_removes_the_earlier_queued_replacement_at_its_fifo_position() {
    assert_decline(Case::PrefixReplacement);
}
#[test]
fn guild_decline_consumes_an_earlier_queued_invite_when_the_live_row_is_missing() {
    assert_decline(Case::PrefixCreation);
}
#[test]
fn guild_decline_is_observed_by_a_following_queued_accept() {
    assert_decline(Case::AcceptAfterDecline);
}
#[test]
fn guild_decline_keeps_its_original_character_when_the_connection_relogs() {
    assert_decline(Case::Relog);
}
#[test]
fn guild_decline_decode_failure_is_counted_without_killing_the_writer() {
    assert_decline(Case::StorageFailure);
}

fn assert_decline(case: Case) {
    let mut f = fixture(!matches!(case, Case::Missing | Case::PrefixCreation));
    if matches!(case, Case::StorageFailure) {
        let txn = f
            .app
            .world()
            .resource::<GameDatabase>()
            .db
            .begin_write()
            .unwrap();
        txn.open_table(persistence::PENDING_GUILD_INVITES)
            .unwrap()
            .insert(f.character, b"invalid invite".as_slice())
            .unwrap();
        crate::persistence_commit::commit(txn).unwrap();
    }
    let held = f
        .app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    if matches!(case, Case::PrefixReplacement | Case::PrefixCreation) {
        let character = f.character;
        f.app.world().resource::<GameDatabase>().writer.enqueue(
            "prior invite replacement",
            move |db| {
                persistence::set_pending_guild_invite(db, character, 8, "New Guild", "New Inviter");
                Ok(())
            },
        );
    }
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    f.app
        .world_mut()
        .get_mut::<MessageSender<GuildDeclineInviteRequest>>(f.connection)
        .unwrap()
        .send::<AuthChannel>(GuildDeclineInviteRequest);
    let started = Instant::now();
    collect_frames(&mut f, 3);
    let elapsed = started.elapsed();
    let pending_before = if matches!(case, Case::StorageFailure) {
        false
    } else {
        persistence::load_pending_guild_invite(
            &f.app.world().resource::<GameDatabase>().db,
            f.character,
        )
        .is_some()
    };
    if matches!(case, Case::Relog) {
        let replacement = f.app.world_mut().spawn_empty().id();
        f.app
            .world_mut()
            .entity_mut(f.connection)
            .remove::<InWorld>()
            .insert((
                InWorld {
                    character_id: f.sibling,
                },
                PlayerLink(replacement),
            ));
    }
    let accepted = if matches!(case, Case::AcceptAfterDecline) {
        let character = f.character;
        Some(
            f.app
                .world()
                .resource::<GameDatabase>()
                .writer
                .enqueue("following invite accept", move |db| {
                    Ok(persistence::accept_pending_guild_invite(db, character).map(|_| ()))
                }),
        )
    } else {
        None
    };
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "guild decline tick waited: {elapsed:?}"
    );
    assert_eq!(
        pending_before,
        !matches!(
            case,
            Case::Missing | Case::PrefixCreation | Case::StorageFailure
        )
    );
    let (done, wait) = std::sync::mpsc::channel();
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("guild decline fixture barrier", move |_| {
            done.send(()).map_err(|error| error.to_string())
        });
    wait.recv().unwrap();
    let invites = collect_frames(&mut f, 6);
    if matches!(case, Case::Relog) {
        assert_eq!(invites.last().unwrap().guild_id, 9);
    }
    if let Some(accepted) = accepted {
        assert_eq!(
            accepted.result(),
            Some(Ok(Err("No pending guild invite".into())))
        );
    }
    assert_eq!(
        f.app
            .world()
            .resource::<GameDatabase>()
            .writer
            .metrics
            .failed
            .load(std::sync::atomic::Ordering::Relaxed),
        u64::from(matches!(case, Case::StorageFailure))
    );
    if !matches!(case, Case::StorageFailure) {
        let db = &f.app.world().resource::<GameDatabase>().db;
        assert!(persistence::load_pending_guild_invite(db, f.character).is_none());
        assert_eq!(
            persistence::load_pending_guild_invite(db, f.sibling)
                .unwrap()
                .guild_id,
            9
        );
        assert!(persistence::load_character_guild(db, f.character).is_none());
    }
    let character = f.character;
    drop(f.app);
    let db = redb::Database::open(f.directory.path().join("decline.redb")).unwrap();
    if !matches!(case, Case::StorageFailure) {
        assert!(persistence::load_pending_guild_invite(&db, character).is_none());
    }
}
