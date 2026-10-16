use super::*;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::merchant::{VendorCatalog, VendorGoods};
use lightyear::prelude::*;
use shared::protocol::*;
use std::time::{Duration, Instant};

struct Fixture {
    app: App,
    characters: [u64; 2],
    connections: [Entity; 2],
    guid: u64,
    directory: tempfile::TempDir,
}
fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let db = redb::Database::create(directory.path().join("charter.redb")).unwrap();
    persistence::create_tables(&db);
    let characters: [u64; 2] = ["Owner", "Signer"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let id = persistence::create_character(&db, index as u64 + 1, name, 1, 1)
                .unwrap()
                .0;
            let mut record = persistence::inventory_store::load_inventory_record(&db, id).unwrap();
            record.inventory.gold = 5000;
            persistence::inventory_store::save_inventory_record(&db, id, &record);
            id
        })
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let mut templates = ItemTemplates::default();
    templates.retail.insert(
        GUILD_CHARTER_ITEM,
        ItemTemplate {
            max_stack: 1,
            bonding: 1,
            allowable_class: -1,
            allowable_race: u64::MAX,
            ..Default::default()
        },
    );
    let mut catalog = VendorCatalog::default();
    catalog.goods.insert(
        GUILD_CHARTER_ITEM,
        VendorGoods {
            name: "Guild Charter".into(),
            quality: 1,
            buy_price: 0,
            sell_price: 0,
            buy_count: 1,
            max_count: 1,
        },
    );
    let guid =
        store::purchase(&db, &templates, &catalog, characters[0], "First Guild", 100).unwrap();
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
    let connections = std::array::from_fn(|index| {
        let player = app.world_mut().spawn_empty().id();
        app.world_mut()
            .spawn((
                Authenticated {
                    account_id: index as u64 + 1,
                },
                InWorld {
                    character_id: characters[index],
                },
                PlayerLink(player),
                RemoteId(PeerId::Local(index as u64 + 1)),
                Connected,
                lightyear::connection::host::HostClient { buffer: Vec::new() },
                MessageSender::<GuildCharterRequest>::default(),
                MessageReceiver::<GuildCharterRequest>::default(),
                MessageSender::<GuildCharterReply>::default(),
                MessageReceiver::<GuildCharterReply>::default(),
            ))
            .id()
    });
    let mut f = Fixture {
        app,
        characters,
        connections,
        guid,
        directory,
    };
    collect_frames(&mut f, 6);
    f
}
fn collect_frames(f: &mut Fixture, frames: usize) -> [Vec<GuildCharterReply>; 2] {
    let mut replies = [Vec::new(), Vec::new()];
    for _ in 0..frames {
        f.app.update();
        for index in 0..2 {
            replies[index].extend(
                f.app
                    .world_mut()
                    .get_mut::<MessageReceiver<GuildCharterReply>>(f.connections[index])
                    .unwrap()
                    .receive(),
            );
        }
    }
    replies
}
#[derive(Clone, Copy)]
enum Case {
    Sign,
    Rename,
    Invited,
    Guilded,
    Double,
    Destroyed,
    Departed,
    Rebound,
    Failure,
}
#[test]
fn charter_sign_waits_for_fifo_before_signer_and_owner_petitions() {
    assert_mutation(Case::Sign);
}
#[test]
fn charter_rename_waits_for_fifo_before_owner_petition() {
    assert_mutation(Case::Rename);
}
#[test]
fn charter_sign_honors_an_earlier_queued_guild_invitation() {
    assert_mutation(Case::Invited);
}
#[test]
fn charter_sign_honors_an_earlier_queued_guild_membership() {
    assert_mutation(Case::Guilded);
}
#[test]
fn charter_sign_duplicate_packets_keep_one_account_signature() {
    assert_mutation(Case::Double);
}
#[test]
fn charter_rename_rechecks_physical_ownership_after_queued_destruction() {
    assert_mutation(Case::Destroyed);
}
#[test]
fn charter_departed_signer_has_no_reply_but_owner_still_refreshes() {
    assert_mutation(Case::Departed);
}
#[test]
fn charter_rebound_signer_gets_no_old_reply_but_owner_refreshes() {
    assert_mutation(Case::Rebound);
}
#[test]
fn charter_storage_failure_is_counted_and_reported_without_success() {
    assert_mutation(Case::Failure);
}
fn enqueue_prior(f: &Fixture, case: Case) {
    let [owner, signer] = f.characters;
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("charter mutation prefix", move |db| {
            match case {
                Case::Invited => {
                    persistence::set_pending_guild_invite(db, signer, 7, "Other Guild", "Master")
                }
                Case::Guilded => {
                    persistence::set_character_guild_by_id(db, signer, 7, "Other Guild")?
                }
                Case::Destroyed => {
                    let mut record =
                        persistence::inventory_store::load_inventory_record(db, owner).unwrap();
                    record.inventory.items.clear();
                    persistence::inventory_store::save_inventory_record(db, owner, &record);
                }
                _ => {}
            }
            let txn = db.begin_write().map_err(|error| error.to_string())?;
            txn.abort().map_err(|error| error.to_string())
        });
}
fn assert_mutation(case: Case) {
    let mut f = fixture();
    if matches!(case, Case::Failure) {
        let txn = f
            .app
            .world()
            .resource::<GameDatabase>()
            .db
            .begin_write()
            .unwrap();
        txn.open_table(store::STATE)
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
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    let rename = matches!(case, Case::Rename | Case::Destroyed);
    let source = usize::from(!rename);
    let request = if rename {
        GuildCharterRequest::Rename {
            charter_guid: f.guid,
            guild_name: "  Durable Guild  ".into(),
        }
    } else {
        GuildCharterRequest::Sign {
            charter_guid: f.guid,
        }
    };
    for _ in 0..if matches!(case, Case::Double) { 2 } else { 1 } {
        f.app
            .world_mut()
            .get_mut::<MessageSender<GuildCharterRequest>>(f.connections[source])
            .unwrap()
            .send::<GuildChannel>(request.clone());
    }
    let started = Instant::now();
    let before = collect_frames(&mut f, 3);
    let elapsed = started.elapsed();
    if matches!(case, Case::Departed) {
        f.app
            .world_mut()
            .entity_mut(f.connections[source])
            .remove::<InWorld>();
    }
    if matches!(case, Case::Rebound) {
        f.app
            .world_mut()
            .entity_mut(f.connections[source])
            .insert(Authenticated { account_id: 99 });
    }
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "charter mutation tick waited: {elapsed:?}"
    );
    assert!(
        before.iter().all(Vec::is_empty),
        "petition preceded FIFO receipt"
    );
    let (done, wait) = std::sync::mpsc::channel();
    f.app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("charter mutation barrier", move |_| {
            done.send(()).map_err(|error| error.to_string())
        });
    wait.recv().unwrap();
    let after = collect_frames(&mut f, 6);
    assert_outcome(&f, case, source, &after);
    let path = f.directory.path().join("charter.redb");
    let guid = f.guid;
    let owner = f.characters[0];
    drop(f.app);
    let db = redb::Database::create(path).unwrap();
    if matches!(case, Case::Failure) {
        return;
    }
    if matches!(case, Case::Destroyed) {
        assert!(store::owned_charter_guid(&db, owner).is_none());
        return;
    }
    let petition = store::snapshot(&db, guid, owner, 1).unwrap();
    let signed = !matches!(case, Case::Rename | Case::Invited | Case::Guilded);
    assert_eq!(
        petition.signatures,
        if signed {
            vec!["Signer"]
        } else {
            Vec::<&str>::new()
        }
    );
    assert_eq!(
        petition.guild_name,
        if rename {
            "Durable Guild"
        } else {
            "First Guild"
        }
    );
    assert_eq!(persistence::load_inventory(&db, owner).items.len(), 1);
    assert_eq!(persistence::load_inventory(&db, owner).gold, 4000);
}
fn assert_outcome(f: &Fixture, case: Case, source: usize, replies: &[Vec<GuildCharterReply>; 2]) {
    let departed = matches!(case, Case::Departed | Case::Rebound);
    let failed = matches!(
        case,
        Case::Invited | Case::Guilded | Case::Destroyed | Case::Failure
    );
    assert_eq!(
        replies[source].len(),
        if departed {
            0
        } else if matches!(case, Case::Double) {
            2
        } else {
            1
        }
    );
    if !departed {
        match &replies[source][0] {
            GuildCharterReply::Failed { message } => {
                assert!(failed);
                assert!(!message.is_empty());
                if matches!(case, Case::Invited) {
                    assert_eq!(message, "You have already been invited into a guild.");
                }
                if matches!(case, Case::Guilded) {
                    assert_eq!(message, "You are already in a guild.");
                }
            }
            GuildCharterReply::Petition(petition) => {
                assert!(!failed);
                if source == 0 {
                    assert_eq!(petition.guild_name, "Durable Guild");
                } else {
                    assert_eq!(petition.signatures, vec!["Signer"]);
                    assert!(!petition.can_sign);
                }
            }
            other => panic!("unexpected charter response: {other:?}"),
        }
    }
    if matches!(case, Case::Double) {
        assert!(
            matches!(&replies[source][1], GuildCharterReply::Failed { message } if message == "You have already signed that guild charter.")
        );
    }
    if source == 1 {
        assert_eq!(replies[0].len(), usize::from(!failed));
        if !failed {
            assert!(
                matches!(&replies[0][0], GuildCharterReply::Petition(petition) if petition.signatures == vec!["Signer"] && petition.is_owner)
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
        u64::from(matches!(case, Case::Failure))
    );
}
