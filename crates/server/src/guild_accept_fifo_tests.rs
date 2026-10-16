use super::*;
use crate::auth::{Authenticated, InWorld};
use redb::ReadableDatabase;
use shared::protocol::*;
use std::time::{Duration, Instant};

struct Fixture {
    app: App,
    characters: [u64; 2],
    players: [Entity; 2],
    connections: [Entity; 2],
    directory: tempfile::TempDir,
}

fn fixture(pending: bool) -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("accept.redb")).unwrap();
    persistence::create_tables(&db);
    let characters = ["Invitee", "Master"]
        .map(|name| persistence::create_character(&db, 2, name, 1, 1).unwrap().0);
    persistence::set_character_guild_by_id(&db, characters[1], 7, "Raid Team").unwrap();
    persistence::banks::save_guild_member_rank(&db, characters[1], 0);
    if pending {
        persistence::set_pending_guild_invite(&db, characters[0], 7, "Raid Team", "Master");
    }
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.insert_resource(GameDatabase::new(db));
    app.add_plugins(crate::guild::GuildPlugin);
    app.add_systems(Update, process_guild_accepts);
    app.finish();
    app.cleanup();
    let players = characters.map(|id| app.world_mut().spawn(ClientId(id)).id());
    let connections = std::array::from_fn(|index| {
        app.world_mut()
            .spawn((
                Authenticated { account_id: 2 },
                InWorld {
                    character_id: characters[index],
                },
                PlayerLink(players[index]),
                RemoteId(PeerId::Local(index as u64 + 1)),
                Connected,
                lightyear::connection::host::HostClient { buffer: Vec::new() },
                MessageSender::<GuildAcceptInviteRequest>::default(),
                MessageReceiver::<GuildAcceptInviteRequest>::default(),
                MessageSender::<GuildAcceptInviteResponse>::default(),
                MessageReceiver::<GuildAcceptInviteResponse>::default(),
                MessageSender::<GuildInvitation>::default(),
                MessageReceiver::<GuildInvitation>::default(),
                MessageSender::<GuildRanksState>::default(),
                MessageReceiver::<GuildRanksState>::default(),
            ))
            .id()
    });
    let mut fixture = Fixture {
        app,
        characters,
        players,
        connections,
        directory,
    };
    collect_frames(&mut fixture, 6);
    fixture
}

#[derive(Default)]
struct Replies {
    accepts: Vec<GuildAcceptInviteResponse>,
    ranks: [Vec<GuildRanksState>; 2],
}

fn collect_frames(f: &mut Fixture, frames: usize) -> Replies {
    let mut replies = Replies::default();
    for _ in 0..frames {
        f.app.update();
        replies.accepts.extend(
            f.app
                .world_mut()
                .get_mut::<MessageReceiver<GuildAcceptInviteResponse>>(f.connections[0])
                .unwrap()
                .receive(),
        );
        for index in 0..2 {
            replies.ranks[index].extend(
                f.app
                    .world_mut()
                    .get_mut::<MessageReceiver<GuildRanksState>>(f.connections[index])
                    .unwrap()
                    .receive(),
            );
        }
    }
    replies
}

#[derive(Clone, Copy)]
enum Case {
    Pending,
    Missing,
    PriorCreation,
    PriorReplacement,
    PriorDecline,
    PriorMembership,
    Double,
    Departed,
    Rebound,
    Relog,
    StorageFailure,
    CleanupFailure,
}

#[test]
fn guild_accept_waits_for_fifo_without_publishing_membership_on_tick() {
    assert_accept(Case::Pending);
}
#[test]
fn guild_accept_missing_invite_is_ordered_without_blocking_ticks() {
    assert_accept(Case::Missing);
}
#[test]
fn guild_accept_observes_an_earlier_queued_invite_creation() {
    assert_accept(Case::PriorCreation);
}
#[test]
fn guild_accept_uses_the_earlier_queued_replacement() {
    assert_accept(Case::PriorReplacement);
}
#[test]
fn guild_accept_observes_an_earlier_queued_decline() {
    assert_accept(Case::PriorDecline);
}
#[test]
fn guild_accept_cannot_replace_earlier_queued_membership() {
    assert_accept(Case::PriorMembership);
}
#[test]
fn guild_accept_duplicate_packets_join_once_and_refuse_the_second() {
    assert_accept(Case::Double);
}
#[test]
fn guild_accept_departed_origin_receives_no_reply_but_members_refresh() {
    assert_accept(Case::Departed);
}
#[test]
fn guild_accept_rebound_account_receives_no_old_reply_or_component() {
    assert_accept(Case::Rebound);
}
#[test]
fn guild_accept_relogged_character_receives_no_old_reply_or_component() {
    assert_accept(Case::Relog);
}
#[test]
fn guild_accept_storage_failure_is_counted_without_success_or_membership() {
    assert_accept(Case::StorageFailure);
}

#[test]
fn guild_accept_cleanup_failure_rolls_back_membership_rank_and_invite_consumption() {
    assert_accept(Case::CleanupFailure);
}

fn enqueue_prior(f: &Fixture, case: Case) {
    let character = f.characters[0];
    let writer = &f.app.world().resource::<GameDatabase>().writer;
    match case {
        Case::PriorCreation | Case::PriorReplacement => {
            writer.enqueue("prior invite replacement", move |db| {
                persistence::set_pending_guild_invite(db, character, 8, "New Guild", "Other");
                persistence::banks::save_guild_member_rank(db, character, 1);
                Ok(())
            });
        }
        Case::PriorDecline => {
            writer.enqueue("prior decline", move |db| {
                persistence::take_pending_guild_invite(db, character);
                Ok(())
            });
        }
        Case::PriorMembership => {
            writer.enqueue("prior membership", move |db| {
                persistence::set_character_guild_by_id(db, character, 9, "Existing")
            });
        }
        _ => {}
    }
}

fn wait_for_writer(f: &Fixture) {
    let (done, wait) = std::sync::mpsc::channel();
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("accept fixture barrier", move |_| {
            done.send(()).map_err(|error| error.to_string())
        });
    wait.recv().unwrap();
}

fn assert_accept(case: Case) {
    let mut f = fixture(!matches!(case, Case::Missing | Case::PriorCreation));
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
            .insert(f.characters[0], b"invalid invite".as_slice())
            .unwrap();
        crate::persistence_commit::commit(txn).unwrap();
    }
    if matches!(case, Case::CleanupFailure) {
        let txn = f
            .app
            .world()
            .resource::<GameDatabase>()
            .db
            .begin_write()
            .unwrap();
        txn.open_table(persistence::charters::STATE)
            .unwrap()
            .insert(0u8, b"invalid charter state".as_slice())
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
    enqueue_prior(&f, case);
    // Every case has a blocked prefix, including read-only domain refusals.
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("accept held prefix", |db| {
            let write = db.begin_write().map_err(|error| error.to_string())?;
            write.abort().map_err(|error| error.to_string())
        });
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    for _ in 0..if matches!(case, Case::Double) { 2 } else { 1 } {
        f.app
            .world_mut()
            .get_mut::<MessageSender<GuildAcceptInviteRequest>>(f.connections[0])
            .unwrap()
            .send::<AuthChannel>(GuildAcceptInviteRequest {});
    }
    let started = Instant::now();
    let before = collect_frames(&mut f, 3);
    let elapsed = started.elapsed();
    let membership_before = f.app.world().get::<GuildMembership>(f.players[0]).cloned();
    if matches!(case, Case::Departed) {
        f.app
            .world_mut()
            .entity_mut(f.connections[0])
            .remove::<InWorld>();
    }
    if matches!(case, Case::Rebound) {
        f.app
            .world_mut()
            .entity_mut(f.connections[0])
            .insert(Authenticated { account_id: 99 });
    }
    if matches!(case, Case::Relog) {
        f.app.world_mut().entity_mut(f.connections[0]).insert((
            InWorld {
                character_id: f.characters[1],
            },
            PlayerLink(f.players[1]),
        ));
    }
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "guild accept tick waited: {elapsed:?}"
    );
    assert!(
        before.accepts.is_empty(),
        "accept reply preceded its FIFO receipt"
    );
    assert!(before.ranks.iter().all(Vec::is_empty));
    assert!(membership_before.is_none());
    assert!(
        f.app
            .world()
            .resource::<crate::guild_bank::ChangedGuilds>()
            .0
            .is_empty()
    );
    wait_for_writer(&f);
    let after = collect_frames(&mut f, 6);
    assert_outcome(&f, case, &after);
    let path = f.directory.path().join("accept.redb");
    let character = f.characters[0];
    let expected =
        persistence::load_character_guild(&f.app.world().resource::<GameDatabase>().db, character);
    drop(f.app);
    let reopened = redb::Database::create(path).unwrap();
    assert_eq!(
        persistence::load_character_guild(&reopened, character),
        expected
    );
}

fn assert_outcome(f: &Fixture, case: Case, replies: &Replies) {
    let db = &f.app.world().resource::<GameDatabase>().db;
    let refused = matches!(
        case,
        Case::Missing
            | Case::PriorDecline
            | Case::PriorMembership
            | Case::StorageFailure
            | Case::CleanupFailure
    );
    let departed = matches!(case, Case::Departed | Case::Rebound | Case::Relog);
    assert_eq!(
        replies.accepts.len(),
        if departed {
            0
        } else if matches!(case, Case::Double) {
            2
        } else {
            1
        }
    );
    if !departed {
        assert_eq!(replies.accepts[0].success, !refused);
        if matches!(case, Case::Double) {
            assert_eq!(
                replies.accepts[1].error.as_deref(),
                Some("You are already in a guild.")
            );
        }
    }
    assert_eq!(
        f.app
            .world()
            .resource::<GameDatabase>()
            .writer
            .metrics
            .failed
            .load(std::sync::atomic::Ordering::Relaxed),
        u64::from(matches!(case, Case::StorageFailure | Case::CleanupFailure))
    );
    if refused {
        let error = replies.accepts[0].error.as_deref().unwrap();
        match case {
            Case::Missing | Case::PriorDecline => assert_eq!(error, "No pending guild invite"),
            Case::PriorMembership => assert_eq!(error, "You are already in a guild."),
            Case::StorageFailure => assert!(error.starts_with("decode pending invite:")),
            Case::CleanupFailure => assert!(error.starts_with("decode guild charter state:")),
            _ => unreachable!(),
        }
        let txn = db.begin_read().unwrap();
        let invites = txn.open_table(persistence::PENDING_GUILD_INVITES).unwrap();
        let pending = invites
            .get(f.characters[0])
            .unwrap()
            .map(|row| row.value().to_vec());
        assert_eq!(
            pending.is_some(),
            matches!(
                case,
                Case::StorageFailure | Case::CleanupFailure | Case::PriorMembership
            )
        );
        if matches!(case, Case::StorageFailure) {
            assert_eq!(pending.as_deref(), Some(b"invalid invite".as_slice()));
        }
        if matches!(case, Case::CleanupFailure) {
            assert_eq!(
                persistence::load_pending_guild_invite(db, f.characters[0])
                    .unwrap()
                    .guild_id,
                7
            );
        }
        let ranks = txn
            .open_table(persistence::banks::GUILD_MEMBER_RANKS)
            .unwrap();
        assert!(ranks.get(f.characters[0]).unwrap().is_none());
        assert!(replies.ranks.iter().all(Vec::is_empty));
        assert!(f.app.world().get::<GuildMembership>(f.players[0]).is_none());
        assert_eq!(
            persistence::load_character_guild(db, f.characters[0]).map(|g| g.guild_id),
            if matches!(case, Case::PriorMembership) {
                Some(9)
            } else {
                None
            }
        );
        return;
    }
    let guild_id = if matches!(case, Case::PriorCreation | Case::PriorReplacement) {
        8
    } else {
        7
    };
    assert_eq!(
        persistence::load_character_guild(db, f.characters[0])
            .unwrap()
            .guild_id,
        guild_id
    );
    assert!(persistence::load_pending_guild_invite(db, f.characters[0]).is_none());
    assert_eq!(
        persistence::banks::load_guild_member_rank(
            db,
            &persistence::banks::load_guild(db, guild_id),
            f.characters[0]
        ),
        4
    );
    assert_eq!(
        f.app
            .world()
            .get::<GuildMembership>(f.players[0])
            .map(|g| g.guild_id),
        if departed { None } else { Some(guild_id) }
    );
    assert!(
        f.app
            .world()
            .resource::<crate::guild_bank::ChangedGuilds>()
            .0
            .contains(&guild_id)
    );
    if guild_id == 7 {
        assert!(
            !replies.ranks[1].is_empty(),
            "current guild member missed durable join"
        );
    }
    assert_eq!(replies.ranks[0].is_empty(), departed);
}
