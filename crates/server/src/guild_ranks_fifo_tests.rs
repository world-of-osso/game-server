use super::*;
use crate::networking::PlayerLink;
use lightyear::prelude::*;
use shared::components::GuildMembership;
use std::time::{Duration, Instant};

struct Fixture {
    app: App,
    characters: [u64; 3],
    players: [Entity; 3],
    connections: [Entity; 3],
    directory: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("ranks.redb")).unwrap();
    persistence::create_tables(&db);
    let names = ["Master", "Officer", "Member"];
    let characters = std::array::from_fn(|index| {
        let (character, _) =
            persistence::create_character(&db, index as u64 + 1, names[index], 1, 1).unwrap();
        persistence::set_character_guild_by_id(&db, character, 7, "Bank Testers").unwrap();
        crate::persistence::banks::save_guild_member_rank(&db, character, [0, 1, 4][index]);
        character
    });
    apply_rank_request(
        &db,
        7,
        characters[0],
        GuildRankRequest::SetPermissions {
            rank: 1,
            rights: GUILD_RIGHT_EDIT_PUBLIC_NOTE,
            gold_per_day: 0,
        },
    )
    .unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.init_resource::<ChangedGuilds>();
    install_persistence(&mut app);
    app.add_systems(Update, process_rank_requests);
    app.finish();
    app.cleanup();
    app.insert_resource(GameDatabase::new(db));
    let players = std::array::from_fn(|_| {
        app.world_mut()
            .spawn(GuildMembership {
                guild_id: 7,
                guild_name: "Bank Testers".into(),
            })
            .id()
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
                MessageSender::<GuildRankRequest>::default(),
                MessageReceiver::<GuildRankRequest>::default(),
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

fn collect_frames(f: &mut Fixture, frames: usize) -> [Vec<GuildRanksState>; 3] {
    let mut replies: [Vec<GuildRanksState>; 3] = std::array::from_fn(|_| Vec::new());
    for _ in 0..frames {
        f.app.update();
        for (index, connection) in f.connections.iter().enumerate() {
            replies[index].extend(
                f.app
                    .world_mut()
                    .get_mut::<MessageReceiver<GuildRanksState>>(*connection)
                    .unwrap()
                    .receive(),
            );
        }
    }
    replies
}

fn send(f: &mut Fixture, index: usize, request: GuildRankRequest) {
    f.app
        .world_mut()
        .get_mut::<MessageSender<GuildRankRequest>>(f.connections[index])
        .unwrap()
        .send::<GuildChannel>(request);
}

#[derive(Clone, Copy)]
enum Case {
    PublicNote,
    RevokedPermission,
    DepartedActor,
    RemoveMember,
    OrderedQuery,
}

#[test]
fn guild_rank_public_note_waits_for_fifo_before_reply_and_change_publication() {
    assert_fifo(Case::PublicNote);
}
#[test]
fn guild_rank_request_reads_an_earlier_queued_permission_revocation() {
    assert_fifo(Case::RevokedPermission);
}
#[test]
fn guild_rank_departed_actor_receives_no_reply_but_current_members_refresh() {
    assert_fifo(Case::DepartedActor);
}
#[test]
fn guild_rank_member_removal_defers_component_cleanup_until_durability() {
    assert_fifo(Case::RemoveMember);
}
#[test]
fn guild_rank_query_reads_the_preceding_queued_edit() {
    assert_fifo(Case::OrderedQuery);
}

fn assert_fifo(case: Case) {
    let mut f = fixture();
    let held = f
        .app
        .world()
        .resource::<GameDatabase>()
        .db
        .begin_write()
        .unwrap();
    if matches!(case, Case::RevokedPermission) {
        let master = f.characters[0];
        f.app.world().resource::<GameDatabase>().writer.enqueue(
            "prior guild permission revocation",
            move |db| {
                apply_rank_request(
                    db,
                    7,
                    master,
                    GuildRankRequest::SetPermissions {
                        rank: 1,
                        rights: 0,
                        gold_per_day: 0,
                    },
                )
                .map_err(|error| format!("{error:?}"))
            },
        );
    }
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    let actor = if matches!(case, Case::RevokedPermission) {
        1
    } else {
        0
    };
    let request = match case {
        Case::RemoveMember => GuildRankRequest::RemoveMember {
            character_name: "Member".into(),
        },
        Case::OrderedQuery => GuildRankRequest::SetPermissions {
            rank: 1,
            rights: 0,
            gold_per_day: 0,
        },
        _ => GuildRankRequest::SetMemberNote {
            character_name: "Member".into(),
            note: "Alchemy 100".into(),
            officer: false,
        },
    };
    send(&mut f, actor, request);
    if matches!(case, Case::OrderedQuery) {
        send(&mut f, actor, GuildRankRequest::Query);
    }
    let started = Instant::now();
    let premature = collect_frames(&mut f, 3);
    let elapsed = started.elapsed();
    let premature_changes = f.app.world().resource::<ChangedGuilds>().0.clone();
    let premature_removal = f.app.world().get::<GuildMembership>(f.players[2]).is_none();
    if matches!(case, Case::DepartedActor) {
        f.app
            .world_mut()
            .entity_mut(f.connections[actor])
            .remove::<InWorld>();
    }
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "guild rank tick waited: {elapsed:?}"
    );
    assert!(
        premature.iter().all(Vec::is_empty),
        "guild reply preceded commit"
    );
    assert!(premature_changes.is_empty(), "guild change preceded commit");
    assert!(
        !premature_removal,
        "member component was removed before commit"
    );
    let (done, wait) = std::sync::mpsc::channel();
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("guild rank fixture barrier", move |_| {
            done.send(()).map_err(|error| error.to_string())
        });
    wait.recv().unwrap();
    let replies = collect_frames(&mut f, 6);
    let db = &f.app.world().resource::<GameDatabase>().db;
    match case {
        Case::RevokedPermission => {
            assert_eq!(
                replies[actor].last().unwrap().error,
                Some(GuildRankError::Permissions)
            );
            assert_eq!(load_guild(db, 7).ranks[1].rights, 0);
            assert_eq!(
                crate::persistence_character::load_guild_public_note(db, f.characters[2]),
                ""
            );
            assert!(f.app.world().resource::<ChangedGuilds>().0.is_empty());
        }
        Case::RemoveMember => {
            assert!(persistence::load_character_guild(db, f.characters[2]).is_none());
            assert!(f.app.world().get::<GuildMembership>(f.players[2]).is_none());
            assert!(replies[2].last().unwrap().members.is_empty());
        }
        Case::OrderedQuery => {
            assert!(replies[actor].len() >= 2);
            assert!(
                replies[actor]
                    .iter()
                    .all(|reply| reply.error.is_none() && reply.ranks[1].rights == 0)
            );
        }
        Case::PublicNote | Case::DepartedActor => {
            assert_eq!(
                crate::persistence_character::load_guild_public_note(db, f.characters[2]),
                "Alchemy 100"
            );
            assert_eq!(replies[2].last().unwrap().error, None);
            if matches!(case, Case::DepartedActor) {
                assert!(replies[actor].is_empty());
            }
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
        0
    );
    let member = f.characters[2];
    drop(f.app);
    let db = redb::Database::open(f.directory.path().join("ranks.redb")).unwrap();
    match case {
        Case::RemoveMember => assert!(persistence::load_character_guild(&db, member).is_none()),
        Case::PublicNote | Case::DepartedActor => assert_eq!(
            crate::persistence_character::load_guild_public_note(&db, member),
            "Alchemy 100"
        ),
        _ => assert_eq!(load_guild(&db, 7).ranks[1].rights, 0),
    }
}
