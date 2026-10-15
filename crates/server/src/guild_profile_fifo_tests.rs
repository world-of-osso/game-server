use super::*;
use lightyear::prelude::*;
use shared::protocol::*;
use std::time::{Duration, Instant};

struct Fixture {
    app: App,
    characters: [u64; 3],
    connections: [Entity; 3],
    directory: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("profile.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let names = ["Master", "Officer", "Member"];
    let characters = std::array::from_fn(|index| {
        let (id, _) =
            crate::persistence::create_character(&db, index as u64 + 1, names[index], 1, 1)
                .unwrap();
        crate::persistence::set_character_guild_by_id(&db, id, 7, "Profile Testers").unwrap();
        crate::persistence::banks::save_guild_member_rank(&db, id, [0, 1, 4][index]);
        id
    });
    apply_rank_request(
        &db,
        7,
        characters[0],
        GuildRankRequest::SetPermissions {
            rank: 1,
            rights: GUILD_RIGHT_EDIT_MOTD
                | GUILD_RIGHT_EDIT_INFO
                | GUILD_RIGHT_EDIT_OFFICER_NOTE
                | GUILD_RIGHT_VIEW_OFFICER_NOTE,
            gold_per_day: 0,
        },
    )
    .unwrap();
    save_guild_member_note(
        &db,
        characters[2],
        &GuildMemberNoteData {
            officer_note: "Before".into(),
        },
    );
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::ProtocolPlugin);
    app.insert_resource(GameDatabase::new(db));
    app.add_plugins(GuildPlugin);
    app.finish();
    app.cleanup();
    let players: [Entity; 3] = std::array::from_fn(|index| {
        app.world_mut()
            .spawn((
                Player {
                    name: names[index].into(),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                Level(1),
                GuildMembership {
                    guild_id: 7,
                    guild_name: "Profile Testers".into(),
                },
            ))
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
                MessageSender::<GuildStateUpdate>::default(),
                MessageReceiver::<GuildStateUpdate>::default(),
                MessageSender::<GuildRankRequest>::default(),
                MessageReceiver::<GuildRankRequest>::default(),
                MessageSender::<GuildRanksState>::default(),
                MessageReceiver::<GuildRanksState>::default(),
            ))
            .insert((
                MessageSender::<SetGuildMotd>::default(),
                MessageReceiver::<SetGuildMotd>::default(),
                MessageSender::<SetGuildInfo>::default(),
                MessageReceiver::<SetGuildInfo>::default(),
                MessageSender::<SetGuildOfficerNote>::default(),
                MessageReceiver::<SetGuildOfficerNote>::default(),
            ))
            .id()
    });
    let mut fixture = Fixture {
        app,
        characters,
        connections,
        directory,
    };
    collect_frames(&mut fixture, 6);
    fixture
}

fn collect_frames(f: &mut Fixture, frames: usize) -> [Vec<GuildStateUpdate>; 3] {
    let mut updates: [Vec<GuildStateUpdate>; 3] = std::array::from_fn(|_| Vec::new());
    for _ in 0..frames {
        f.app.update();
        for (index, connection) in f.connections.iter().enumerate() {
            updates[index].extend(
                f.app
                    .world_mut()
                    .get_mut::<MessageReceiver<GuildStateUpdate>>(*connection)
                    .unwrap()
                    .receive(),
            );
        }
    }
    updates
}

#[derive(Clone, Copy)]
enum Case {
    Motd,
    Info,
    Note,
    RevokedRights,
    RevokedView,
    DepartedActor,
    ReboundActor,
    StorageFailure,
}

#[test]
fn guild_profile_motd_waits_for_fifo_and_merges_the_latest_profile() {
    assert_fifo(Case::Motd);
}
#[test]
fn guild_profile_info_waits_for_fifo_and_merges_the_latest_profile() {
    assert_fifo(Case::Info);
}
#[test]
fn guild_profile_officer_note_waits_for_fifo_and_preserves_viewer_privacy() {
    assert_fifo(Case::Note);
}
#[test]
fn guild_profile_request_observes_prior_queued_rights_revocation() {
    assert_fifo(Case::RevokedRights);
}
#[test]
fn guild_profile_publication_uses_current_view_rights_after_a_later_commit() {
    assert_fifo(Case::RevokedView);
}
#[test]
fn guild_profile_departed_actor_gets_no_reply_but_other_members_refresh() {
    assert_fifo(Case::DepartedActor);
}
#[test]
fn guild_profile_rebound_account_gets_no_old_actor_reply() {
    assert_fifo(Case::ReboundActor);
}
#[test]
fn guild_profile_storage_failure_is_counted_and_reported_without_success() {
    assert_fifo(Case::StorageFailure);
}

fn assert_fifo(case: Case) {
    let mut f = fixture();
    if matches!(case, Case::StorageFailure) {
        let txn = f
            .app
            .world()
            .resource::<GameDatabase>()
            .db
            .begin_write()
            .unwrap();
        txn.open_table(crate::persistence::GUILD_PROFILES)
            .unwrap()
            .insert(7, b"invalid profile".as_slice())
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
    if !matches!(case, Case::StorageFailure) {
        let master = f.characters[0];
        f.app.world().resource::<GameDatabase>().writer.enqueue(
            "prior guild profile and authority",
            move |db| {
                let mut profile = load_guild_profile(db, 7);
                profile.motd = "Prior motd".into();
                profile.info_text = "Prior info".into();
                save_guild_profile(db, 7, &profile);
                if matches!(case, Case::RevokedRights) {
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
                    .map_err(|error| format!("{error:?}"))?;
                }
                Ok(())
            },
        );
    }
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    send_request(&mut f, case);
    let started = Instant::now();
    let premature = collect_frames(&mut f, 3);
    let elapsed = started.elapsed();
    match case {
        Case::DepartedActor => {
            f.app
                .world_mut()
                .entity_mut(f.connections[1])
                .remove::<InWorld>();
        }
        Case::ReboundActor => {
            f.app
                .world_mut()
                .get_mut::<Authenticated>(f.connections[1])
                .unwrap()
                .account_id = 99;
        }
        _ => {}
    }
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "guild profile tick waited: {elapsed:?}"
    );
    assert!(
        premature.iter().all(Vec::is_empty),
        "guild profile reply preceded durability"
    );
    let (done, wait) = std::sync::mpsc::channel();
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("guild profile fixture barrier", move |_| {
            done.send(()).map_err(|error| error.to_string())
        });
    wait.recv().unwrap();
    let updates = collect_frames(&mut f, 6);
    assert_outcome(&f, case, &updates);
    let member = f.characters[2];
    drop(f.app);
    let db = redb::Database::open(f.directory.path().join("profile.redb")).unwrap();
    assert_saved(&db, member, case);
}

fn send_request(f: &mut Fixture, case: Case) {
    match case {
        Case::Motd | Case::RevokedRights | Case::StorageFailure => f
            .app
            .world_mut()
            .get_mut::<MessageSender<SetGuildMotd>>(f.connections[1])
            .unwrap()
            .send::<GuildChannel>(SetGuildMotd {
                text: "  Raid at eight  ".into(),
            }),
        Case::Info => f
            .app
            .world_mut()
            .get_mut::<MessageSender<SetGuildInfo>>(f.connections[1])
            .unwrap()
            .send::<GuildChannel>(SetGuildInfo {
                text: "  Raid notes  ".into(),
            }),
        _ => f
            .app
            .world_mut()
            .get_mut::<MessageSender<SetGuildOfficerNote>>(f.connections[1])
            .unwrap()
            .send::<GuildChannel>(SetGuildOfficerNote {
                character_name: "Member".into(),
                note: "  Raid healer  ".into(),
            }),
    }
    if matches!(case, Case::RevokedView) {
        f.app
            .world_mut()
            .get_mut::<MessageSender<GuildRankRequest>>(f.connections[0])
            .unwrap()
            .send::<GuildChannel>(GuildRankRequest::SetPermissions {
                rank: 1,
                rights: GUILD_RIGHT_EDIT_OFFICER_NOTE,
                gold_per_day: 0,
            });
    }
}

fn assert_outcome(f: &Fixture, case: Case, updates: &[Vec<GuildStateUpdate>; 3]) {
    let failed = f
        .app
        .world()
        .resource::<GameDatabase>()
        .writer
        .metrics
        .failed
        .load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(failed, u64::from(matches!(case, Case::StorageFailure)));
    if matches!(case, Case::StorageFailure) {
        assert!(updates[0].is_empty() && updates[2].is_empty());
        let error = updates[1].last().unwrap();
        assert!(
            error
                .error
                .as_deref()
                .unwrap()
                .to_ascii_lowercase()
                .contains("decode")
        );
        assert!(error.message.is_none());
        return;
    }
    let db = &f.app.world().resource::<GameDatabase>().db;
    assert_saved(db, f.characters[2], case);
    if matches!(case, Case::RevokedRights) {
        assert_eq!(
            updates[1].last().unwrap().error.as_deref(),
            Some(GuildRankError::Permissions.message())
        );
        assert!(updates[0].is_empty() && updates[2].is_empty());
        return;
    }
    for index in [0, 2] {
        let update = updates[index].last().unwrap();
        assert_eq!(update.error, None);
        assert!(update.message.is_some());
        let snapshot = update.guild.as_ref().unwrap();
        let note = &snapshot
            .members
            .iter()
            .find(|member| member.character_name == "Member")
            .unwrap()
            .officer_note;
        let expected = if index == 0 && !matches!(case, Case::Motd | Case::Info) {
            "  Raid healer  "
        } else if index == 0 {
            "Before"
        } else {
            ""
        };
        assert_eq!(note, expected);
    }
    if matches!(case, Case::DepartedActor | Case::ReboundActor) {
        assert!(updates[1].is_empty());
    } else if matches!(case, Case::RevokedView) {
        let profile = updates[1].last().unwrap().guild.as_ref().unwrap();
        assert!(
            profile
                .members
                .iter()
                .find(|member| member.character_name == "Member")
                .unwrap()
                .officer_note
                .is_empty()
        );
    }
}

fn assert_saved(db: &redb::Database, member: u64, case: Case) {
    let note = load_guild_member_note(db, member).officer_note;
    if matches!(case, Case::StorageFailure) {
        assert_eq!(note, "Before");
        return;
    }
    let profile = load_guild_profile(db, 7);
    assert_eq!(
        profile.motd,
        if matches!(case, Case::Motd) {
            "Raid at eight"
        } else {
            "Prior motd"
        }
    );
    assert_eq!(
        profile.info_text,
        if matches!(case, Case::Info) {
            "Raid notes"
        } else {
            "Prior info"
        }
    );
    assert_eq!(
        note,
        if matches!(case, Case::Motd | Case::Info | Case::RevokedRights) {
            "Before"
        } else {
            "  Raid healer  "
        }
    );
}
