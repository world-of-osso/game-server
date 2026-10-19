//! Voluntary ownership crosses the existing request, durable rank and HostClient boundaries.
use super::*;
use crate::bank_test_harness;
use crate::persistence::banks::{
    GUILD_MEMBER_RANKS, load_guild, load_guild_member_rank, save_guild_member_rank,
};
use crate::persistence::{create_tables, temp_database};
use crate::persistence_character::{create_character, set_character_guild_by_id};
use shared::protocol::*;

fn transfer_request(name: &str) -> GuildRankRequest {
    serde_json::from_value(serde_json::json!({"TransferLeadership": {"character_name": name}}))
        .expect("the authoritative voluntary transfer request must be represented")
}

fn members(db: &redb::Database) -> (u64, u64, u64) {
    let ids = ["Master", "Officer", "Member"].map(|name| {
        let (id, _) = create_character(db, 1, name, 1, 1).unwrap();
        set_character_guild_by_id(db, id, 7, "Transfer Guild").unwrap();
        id
    });
    for (id, rank) in ids.into_iter().zip([0, 1, 4]) {
        save_guild_member_rank(db, id, rank);
    }
    (ids[0], ids[1], ids[2])
}

fn rank(db: &redb::Database, character: u64) -> u8 {
    load_guild_member_rank(db, &load_guild(db, 7), character)
}

#[test]
#[ignore = "guild leadership transfer RED before implementation"]
fn guild_leadership_transfer_moves_authority_and_reopens_before_former_master_leaves() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("leadership.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (master, officer, member) = members(&db);
    let before = load_guild(&db, 7);
    let old_events = crate::guild_event_log::query(&db, master).events;
    assert_eq!(
        apply_rank_request(&db, 7, master, GuildRankRequest::Leave),
        Err(GuildRankError::LeaderCannotLeave)
    );
    // The target has no connection: source membership, not online visibility, authorizes it.
    apply_rank_request(&db, 7, master, transfer_request("member"))
        .expect("actual current master transfers to a stored offline member");
    assert_eq!(
        (rank(&db, master), rank(&db, officer), rank(&db, member)),
        (4, 1, 0)
    );
    assert_eq!(
        load_guild(&db, 7),
        before,
        "rank definitions and bank state unchanged"
    );
    assert_eq!(
        crate::guild_event_log::query(&db, master).events,
        old_events
    );
    drop(db);
    let db = redb::Database::open(&path).unwrap();
    assert_eq!(
        (rank(&db, master), rank(&db, officer), rank(&db, member)),
        (4, 1, 0)
    );
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::Rename {
                rank: 1,
                name: "Core".into()
            }
        ),
        Err(GuildRankError::Permissions)
    );
    apply_rank_request(
        &db,
        7,
        member,
        GuildRankRequest::Rename {
            rank: 1,
            name: "Core".into(),
        },
    )
    .expect("new master immediately owns settings authority after reopen");
    assert_eq!(load_guild(&db, 7).ranks[1].name, "Core");
    assert_eq!(
        apply_rank_request(&db, 7, member, GuildRankRequest::Leave),
        Err(GuildRankError::LeaderCannotLeave)
    );
    apply_rank_request(&db, 7, master, GuildRankRequest::Leave).unwrap();
    assert!(crate::persistence::load_character_guild(&db, master).is_none());
    assert_eq!(rank(&db, member), 0);
}

#[test]
#[ignore = "guild leadership transfer RED before implementation"]
fn guild_leadership_transfer_refuses_nonleader_foreign_missing_and_policy_self_without_writes() {
    let db = temp_database();
    let (master, officer, _) = members(&db);
    let (foreign, _) = create_character(&db, 1, "Foreign", 1, 1).unwrap();
    set_character_guild_by_id(&db, foreign, 8, "Other Guild").unwrap();
    save_guild_member_rank(&db, foreign, 0);
    let (visitor, _) = create_character(&db, 1, "Visitor", 1, 1).unwrap();
    let before = guild_rank_snapshot(&db, master, None);
    let foreign_before = guild_rank_snapshot(&db, foreign, None);
    for (actor, target, error) in [
        (officer, "Member", GuildRankError::Permissions),
        (master, "Foreign", GuildRankError::MemberNotFound),
        (master, "Absent", GuildRankError::MemberNotFound),
        (master, "", GuildRankError::MemberNotFound),
        (master, "mAsTeR", GuildRankError::Permissions),
        (foreign, "Member", GuildRankError::NotInGuild),
        (visitor, "Member", GuildRankError::NotInGuild),
    ] {
        assert_eq!(
            apply_rank_request(&db, 7, actor, transfer_request(target)),
            Err(error)
        );
        assert_eq!(guild_rank_snapshot(&db, master, None), before);
        assert_eq!(guild_rank_snapshot(&db, foreign, None), foreign_before);
    }
}

#[test]
#[ignore = "guild leadership transfer RED before implementation"]
fn guild_leadership_transfer_host_replies_to_both_members_and_rechecks_auth_and_new_authority() {
    let mut h = bank_test_harness::harness().unwrap();
    let master = h.character_id;
    let member = h.create_alt("Cara");
    for (id, rank) in [(master, 0), (member, 4)] {
        set_character_guild_by_id(h.db(), id, 7, "Transfer Guild").unwrap();
        save_guild_member_rank(h.db(), id, rank);
    }
    let peer = h.connect_character(member, "Cara");
    h.take::<GuildRanksState>();
    h.take_for::<GuildRanksState>(peer);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<crate::auth::Authenticated>();
    h.send::<GuildRankRequest, GuildChannel>(transfer_request("Cara"));
    assert!(
        h.take::<GuildRanksState>().is_empty(),
        "unauthenticated request stays unread"
    );
    assert_eq!((rank(h.db(), master), rank(h.db(), member)), (0, 4));
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    h.send::<GuildRankRequest, GuildChannel>(transfer_request("Cara"));
    let old_master = h
        .take::<GuildRanksState>()
        .pop()
        .expect("old master refreshed");
    let new_master = h
        .take_for::<GuildRanksState>(peer)
        .pop()
        .expect("new master refreshed");
    assert_eq!(old_master.own_rank, 4);
    assert_eq!(new_master.own_rank, 0);
    assert!(old_master.error.is_none() && new_master.error.is_none());
    assert_eq!(old_master.members, new_master.members);
    assert_eq!(
        old_master
            .members
            .iter()
            .filter(|member| member.rank == 0)
            .count(),
        1
    );
    h.send::<GuildRankRequest, GuildChannel>(GuildRankRequest::Rename {
        rank: 1,
        name: "Forbidden".into(),
    });
    assert_eq!(
        h.take::<GuildRanksState>().pop().unwrap().error,
        Some(GuildRankError::Permissions)
    );
    h.send_from::<GuildRankRequest, GuildChannel>(
        peer,
        GuildRankRequest::Rename {
            rank: 1,
            name: "Core".into(),
        },
    );
    assert!(
        h.take_for::<GuildRanksState>(peer)
            .pop()
            .unwrap()
            .error
            .is_none()
    );
    assert_eq!(load_guild(h.db(), 7).ranks[1].name, "Core");
}

#[test]
#[ignore = "guild leadership transfer RED before implementation"]
fn guild_leadership_transfer_readers_never_observe_zero_or_two_masters_during_real_commits() {
    use redb::{ReadableDatabase, ReadableTable};
    use std::sync::{Arc, Barrier};
    let db = Arc::new(temp_database());
    let (master, _, member) = members(&db);
    let to_member = transfer_request("Member");
    let to_master = transfer_request("Master");
    let barrier = Arc::new(Barrier::new(2));
    let writer_db = Arc::clone(&db);
    let writer_barrier = Arc::clone(&barrier);
    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        for index in 0..31 {
            let (actor, request) = if index % 2 == 0 {
                (master, to_member.clone())
            } else {
                (member, to_master.clone())
            };
            apply_rank_request(&writer_db, 7, actor, request).unwrap();
        }
    });
    barrier.wait();
    for _ in 0..1000 {
        let transaction = db.begin_read().unwrap();
        let ranks = transaction.open_table(GUILD_MEMBER_RANKS).unwrap();
        let pair = (
            ranks.get(master).unwrap().unwrap().value(),
            ranks.get(member).unwrap().unwrap().value(),
        );
        assert!(
            pair == (0, 4) || pair == (4, 0),
            "observed partial leadership commit: {pair:?}"
        );
    }
    writer.join().unwrap();
    assert_eq!((rank(&db, master), rank(&db, member)), (4, 0));
}
