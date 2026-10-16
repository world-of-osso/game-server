use super::*;
use crate::auth::{Authenticated, InWorld};
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
    let db = redb::Database::create(directory.path().join("invite.redb")).unwrap();
    persistence::create_tables(&db);
    let names = ["Master", "Invitee"];
    let characters = std::array::from_fn(|index| {
        persistence::create_character(&db, index as u64 + 1, names[index], 1, 1)
            .unwrap()
            .0
    });
    persistence::set_character_guild_by_id(&db, characters[0], 7, "Raid Team").unwrap();
    persistence::banks::save_guild_member_rank(&db, characters[0], 0);
    if pending {
        persistence::set_pending_guild_invite(&db, characters[1], 8, "Old Guild", "Old Inviter");
    }
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.insert_resource(GameDatabase::new(db));
    app.add_plugins(crate::guild::GuildPlugin);
    app.add_systems(Update, process_guild_invites);
    app.finish();
    app.cleanup();
    let players = std::array::from_fn(|index| {
        app.world_mut()
            .spawn((
                ClientId(characters[index]),
                Player {
                    name: names[index].into(),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
            ))
            .id()
    });
    app.world_mut()
        .entity_mut(players[0])
        .insert(GuildMembership {
            guild_id: 7,
            guild_name: "Raid Team".into(),
        });
    let connections = std::array::from_fn(|index| {
        app.world_mut()
            .spawn((
                Authenticated {
                    account_id: index as u64 + 1,
                },
                InWorld {
                    character_id: characters[index],
                },
                PlayerLink(players[index]),
                RemoteId(PeerId::Local(index as u64 + 1)),
                Connected,
                lightyear::connection::host::HostClient { buffer: Vec::new() },
                MessageSender::<GuildInviteRequest>::default(),
                MessageReceiver::<GuildInviteRequest>::default(),
                MessageSender::<GuildInviteResponse>::default(),
                MessageReceiver::<GuildInviteResponse>::default(),
                MessageSender::<GuildInvitation>::default(),
                MessageReceiver::<GuildInvitation>::default(),
            ))
            .id()
    });
    let mut f = Fixture {
        app,
        characters,
        players,
        connections,
        directory,
    };
    collect_frames(&mut f, 6);
    f
}

#[derive(Default)]
struct Replies {
    owner: Vec<GuildInviteResponse>,
    target: Vec<GuildInvitation>,
}
fn collect_frames(f: &mut Fixture, frames: usize) -> Replies {
    let mut replies = Replies::default();
    for _ in 0..frames {
        f.app.update();
        replies.owner.extend(
            f.app
                .world_mut()
                .get_mut::<MessageReceiver<GuildInviteResponse>>(f.connections[0])
                .unwrap()
                .receive(),
        );
        replies.target.extend(
            f.app
                .world_mut()
                .get_mut::<MessageReceiver<GuildInvitation>>(f.connections[1])
                .unwrap()
                .receive(),
        );
    }
    replies
}

#[derive(Clone, Copy)]
enum Case {
    Invite,
    RevokedRights,
    GuildedTarget,
    PendingTarget,
    PriorDecline,
    Double,
    Departed,
    Rebound,
    TargetRelog,
    Failure,
}
#[test]
fn guild_invite_success_and_target_notification_wait_for_fifo() {
    assert_invite(Case::Invite);
}
#[test]
fn guild_invite_reads_earlier_queued_rights_revocation() {
    assert_invite(Case::RevokedRights);
}
#[test]
fn guild_invite_reads_earlier_queued_target_membership() {
    assert_invite(Case::GuildedTarget);
}
#[test]
fn guild_invite_reads_earlier_queued_pending_invite() {
    assert_invite(Case::PendingTarget);
}
#[test]
fn guild_invite_observes_prior_decline_before_reinviting() {
    assert_invite(Case::PriorDecline);
}
#[test]
fn guild_invite_duplicate_packets_persist_only_one_invitation() {
    assert_invite(Case::Double);
}
#[test]
fn guild_invite_departed_source_does_not_prevent_target_notification() {
    assert_invite(Case::Departed);
}
#[test]
fn guild_invite_rebound_source_gets_no_old_response() {
    assert_invite(Case::Rebound);
}
#[test]
fn guild_invite_relogged_target_gets_no_other_characters_invitation() {
    assert_invite(Case::TargetRelog);
}
#[test]
fn guild_invite_storage_failure_is_counted_without_success_or_notification() {
    assert_invite(Case::Failure);
}

fn enqueue_prior(f: &Fixture, case: Case) {
    let [actor, target] = f.characters;
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("guild invite prefix", move |db| {
            match case {
                Case::RevokedRights => persistence::banks::save_guild_member_rank(db, actor, 4),
                Case::GuildedTarget => {
                    persistence::set_character_guild_by_id(db, target, 9, "Existing")?
                }
                Case::PendingTarget => {
                    persistence::set_pending_guild_invite(db, target, 8, "Other Guild", "Other")
                }
                Case::PriorDecline => {
                    persistence::take_pending_guild_invite(db, target);
                }
                _ => {}
            }
            let txn = db.begin_write().map_err(|error| error.to_string())?;
            txn.abort().map_err(|error| error.to_string())
        });
}

fn assert_invite(case: Case) {
    let mut f = fixture(matches!(case, Case::PriorDecline));
    if matches!(case, Case::Failure) {
        let txn = f
            .app
            .world()
            .resource::<GameDatabase>()
            .db
            .begin_write()
            .unwrap();
        txn.open_table(persistence::banks::GUILDS)
            .unwrap()
            .insert(7u32, b"invalid guild".as_slice())
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
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    for _ in 0..if matches!(case, Case::Double) { 2 } else { 1 } {
        f.app
            .world_mut()
            .get_mut::<MessageSender<GuildInviteRequest>>(f.connections[0])
            .unwrap()
            .send::<AuthChannel>(GuildInviteRequest {
                target_character_name: "Invitee".into(),
            });
    }
    let started = Instant::now();
    let before = collect_frames(&mut f, 3);
    let elapsed = started.elapsed();
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
    if matches!(case, Case::TargetRelog) {
        f.app.world_mut().entity_mut(f.connections[1]).insert((
            InWorld {
                character_id: f.characters[0],
            },
            PlayerLink(f.players[0]),
            Authenticated { account_id: 1 },
        ));
    }
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "guild invite tick waited: {elapsed:?}"
    );
    assert!(
        before.owner.is_empty(),
        "invite response preceded FIFO receipt"
    );
    assert!(
        before.target.is_empty(),
        "target invitation preceded durability"
    );
    let (done, wait) = std::sync::mpsc::channel();
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("invite fixture barrier", move |_| {
            done.send(()).map_err(|error| error.to_string())
        });
    wait.recv().unwrap();
    let after = collect_frames(&mut f, 6);
    let refused = matches!(
        case,
        Case::RevokedRights | Case::GuildedTarget | Case::PendingTarget | Case::Failure
    );
    let departed = matches!(case, Case::Departed | Case::Rebound);
    assert_eq!(
        after.owner.len(),
        if departed {
            0
        } else if matches!(case, Case::Double) {
            2
        } else {
            1
        }
    );
    if !departed {
        assert_eq!(after.owner[0].success, !refused);
        match case {
            Case::RevokedRights => assert_eq!(
                after.owner[0].error.as_deref(),
                Some(GuildRankError::Permissions.message())
            ),
            Case::GuildedTarget => assert_eq!(
                after.owner[0].error.as_deref(),
                Some("Invitee is already in a guild.")
            ),
            Case::PendingTarget => assert_eq!(
                after.owner[0].error.as_deref(),
                Some("Invitee has already been invited to a guild.")
            ),
            Case::Failure => assert!(
                after.owner[0]
                    .error
                    .as_deref()
                    .is_some_and(|error| !error.is_empty())
            ),
            Case::Double => {
                assert!(!after.owner[1].success);
                assert_eq!(
                    after.owner[1].error.as_deref(),
                    Some("Invitee has already been invited to a guild.")
                );
            }
            _ => {}
        }
    }
    assert_eq!(
        after.target.len(),
        usize::from(!refused && !matches!(case, Case::TargetRelog))
    );
    if let Some(invite) = after.target.first() {
        assert_eq!(invite.guild_id, 7);
        assert_eq!(invite.guild_name, "Raid Team");
        assert_eq!(invite.inviter_name, "Master");
    }
    assert_eq!(
        f.app
            .world()
            .resource::<GameDatabase>()
            .writer
            .metrics
            .failed
            .load(std::sync::atomic::Ordering::Relaxed),
        u64::from(matches!(case, Case::Failure))
    );
    let expected_guild = if matches!(case, Case::PendingTarget) {
        Some(8)
    } else if refused {
        None
    } else {
        Some(7)
    };
    let db = &f.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_pending_guild_invite(db, f.characters[1]).map(|invite| invite.guild_id),
        expected_guild
    );
    let path = f.directory.path().join("invite.redb");
    let target = f.characters[1];
    drop(f.app);
    let db = redb::Database::create(path).unwrap();
    assert_eq!(
        persistence::load_pending_guild_invite(&db, target).map(|invite| invite.guild_id),
        expected_guild
    );
}
