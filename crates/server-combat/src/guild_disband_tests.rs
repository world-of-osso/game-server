//! Real authenticated dispatch, durable deletion and online membership refresh.
use super::guild_ranks::snapshot;
use super::*;
use crate::persistence::banks::{load_guild, save_guild, save_guild_member_rank};
use crate::persistence::{create_tables, temp_database};
use crate::persistence_character::{create_character, set_character_guild_by_id};
use redb::ReadableDatabase;
use shared::protocol::*;

fn request() -> GuildRankRequest {
    serde_json::from_str("\"Disband\"").expect("guild disband must be represented")
}

fn members(db: &redb::Database) -> (u64, u64, u64) {
    let ids = ["Master", "Officer", "Member"].map(|name| {
        let (id, _) = create_character(db, 1, name, 1, 1).unwrap();
        set_character_guild_by_id(db, id, 7, "Disband Guild").unwrap();
        id
    });
    for (id, rank) in ids.into_iter().zip([0, 1, 4]) {
        save_guild_member_rank(db, id, rank);
    }
    (ids[0], ids[1], ids[2])
}

fn seed_metadata(db: &redb::Database, member: u64, former: u64, foreign: u64) {
    let transaction = db.begin_write().unwrap();
    seed_bank_usage(&transaction, &[(member, 7), (former, 7), (foreign, 8)]);
    seed_guild_identity(&transaction);
    seed_bank_log(&transaction);
    for (guild_id, member) in [(7, "Master"), (8, "Foreign")] {
        crate::guild_news::append_in(
            &transaction,
            guild_id,
            GuildNewsKind::ItemLooted {
                member: member.into(),
                item_id: 2589,
            },
            101,
            false,
        );
    }
    transaction.commit().unwrap();
}

fn seed_bank_usage(transaction: &redb::WriteTransaction, members: &[(u64, u32)]) {
    for &(id, guild_id) in members {
        let usage = crate::persistence::banks::GuildBankUsage {
            guild_id,
            day: 20_000,
            tab_withdrawals: vec![2],
            copper_withdrawn: 10_000,
        };
        transaction
            .open_table(crate::persistence::banks::GUILD_BANK_USAGE)
            .unwrap()
            .insert(id, bitcode::encode(&usage).as_slice())
            .unwrap();
    }
}

fn seed_guild_identity(transaction: &redb::WriteTransaction) {
    let design = shared::guild_creation::GuildTabardDesign {
        icon: 1,
        icon_color: 1,
        border: 1,
        border_color: 1,
        background_color: 1,
    };
    transaction
        .open_table(crate::persistence::guild_tabards::DESIGNS)
        .unwrap()
        .insert(7, serde_json::to_vec(&design).unwrap().as_slice())
        .unwrap();
    transaction
        .open_table(crate::persistence::charters::CREATED_AT)
        .unwrap()
        .insert(7, 1_728_600_000)
        .unwrap();
}

fn seed_bank_log(transaction: &redb::WriteTransaction) {
    let logs = crate::persistence::banks::GuildBankLogs {
        tabs: vec![],
        money: vec![crate::persistence::banks::GuildBankLogData {
            kind: crate::persistence::banks::GuildBankLogKindData::DepositMoney,
            actor: "Master".into(),
            item_id: 0,
            item_name: String::new(),
            count: 0,
            copper: 735_121,
            time: 100,
        }],
    };
    transaction
        .open_table(crate::persistence::banks::GUILD_BANK_LOGS)
        .unwrap()
        .insert(7, bitcode::encode(&logs).as_slice())
        .unwrap();
}

#[test]
fn guild_disband_refuses_unguilded_nonmaster_and_foreign_without_mutation() {
    let db = temp_database();
    let (master, officer, member) = members(&db);
    for actor in [officer, member] {
        assert_eq!(
            apply_rank_request(&db, 7, actor, request()),
            Err(GuildRankError::Permissions)
        );
    }
    assert_eq!(
        apply_rank_request(&db, 8, master, request()),
        Err(GuildRankError::NotInGuild)
    );
    assert_eq!(
        apply_rank_request(&db, 7, u64::MAX, request()),
        Err(GuildRankError::NotInGuild)
    );
    assert_eq!(
        crate::persistence_character::list_guild_members(&db, 7).len(),
        3
    );
}

#[test]
fn guild_disband_deletes_offline_members_bank_and_profile_atomically_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("disband.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (master, officer, member) = members(&db);
    let (foreign, _) = create_character(&db, 2, "Foreign", 1, 1).unwrap();
    set_character_guild_by_id(&db, foreign, 8, "Other Guild").unwrap();
    let mut bank = load_guild(&db, 7);
    bank.money = 735_121;
    bank.tabs
        .push(crate::bank_storage::BankTabData::new("Supplies".into()));
    save_guild(&db, 7, &bank);
    let mut foreign_bank = load_guild(&db, 8);
    foreign_bank.money = 43_210;
    save_guild(&db, 8, &foreign_bank);
    let (former, _) = create_character(&db, 2, "Former", 1, 1).unwrap();
    set_character_guild_by_id(&db, former, 7, "Disband Guild").unwrap();
    save_guild_member_rank(&db, former, 4);
    seed_metadata(&db, member, former, foreign);
    apply_rank_request(&db, 7, former, GuildRankRequest::Leave).unwrap();
    let inventory_before = crate::persistence::load_inventory(&db, master);
    crate::persistence_character::save_guild_profile(
        &db,
        7,
        &crate::persistence_types::GuildProfileData {
            motd: "Raid tonight".into(),
            info_text: "A real guild profile".into(),
        },
    );
    crate::persistence_character::save_guild_member_note(
        &db,
        officer,
        &crate::persistence_types::GuildMemberNoteData {
            officer_note: "Private officer".into(),
        },
    );
    crate::persistence_character::save_guild_public_note(&db, member, "Public member");
    let (invited, _) = create_character(&db, 2, "Invited", 1, 1).unwrap();
    let (other_invited, _) = create_character(&db, 2, "OtherInvite", 1, 1).unwrap();
    crate::persistence_character::set_pending_guild_invite(
        &db,
        invited,
        7,
        "Disband Guild",
        "Master",
    );
    crate::persistence_character::set_pending_guild_invite(
        &db,
        other_invited,
        8,
        "Other Guild",
        "Foreign",
    );
    apply_rank_request(&db, 7, master, request()).unwrap();
    drop(db);
    let db = redb::Database::open(&path).unwrap();
    for id in [master, officer, member] {
        assert!(crate::persistence::load_character_guild(&db, id).is_none());
        assert!(snapshot(&db, id, None).ranks.is_empty());
    }
    assert_eq!(
        crate::persistence::load_character_guild(&db, foreign)
            .unwrap()
            .guild_id,
        8
    );
    assert!(crate::persistence_character::load_pending_guild_invite(&db, invited).is_none());
    assert_eq!(
        crate::persistence_character::load_pending_guild_invite(&db, other_invited)
            .unwrap()
            .guild_id,
        8
    );
    assert!(
        crate::persistence_character::load_guild_member_note(&db, officer)
            .officer_note
            .is_empty()
    );
    assert!(crate::persistence_character::load_guild_public_note(&db, member).is_empty());
    assert_eq!(load_guild(&db, 8), foreign_bank);
    assert_eq!(
        crate::persistence::load_inventory(&db, master),
        inventory_before
    );
    let read = db.begin_read().unwrap();
    for definition in [
        crate::persistence::banks::GUILD_BANK_LOGS,
        crate::persistence::guild_tabards::DESIGNS,
    ] {
        assert!(
            read.open_table(definition)
                .unwrap()
                .get(7)
                .unwrap()
                .is_none()
        );
    }
    for definition in [crate::guild_event_log::EVENTS, crate::guild_news::NEWS] {
        let table = read.open_table(definition).unwrap();
        assert!(table.get(7).unwrap().is_none());
        assert!(table.get(8).unwrap().is_some());
    }
    assert!(
        read.open_table(crate::persistence::charters::CREATED_AT)
            .unwrap()
            .get(7)
            .unwrap()
            .is_none()
    );
    let usage = read
        .open_table(crate::persistence::banks::GUILD_BANK_USAGE)
        .unwrap();
    assert!(usage.get(member).unwrap().is_none());
    assert!(usage.get(former).unwrap().is_none());
    assert!(usage.get(foreign).unwrap().is_some());
    assert!(
        read.open_table(crate::persistence::banks::GUILDS)
            .unwrap()
            .get(7)
            .unwrap()
            .is_none()
    );
    assert!(
        read.open_table(crate::persistence::GUILD_PROFILES)
            .unwrap()
            .get(7)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        apply_rank_request(&db, 7, master, request()),
        Err(GuildRankError::NotInGuild)
    );
}

#[test]
fn guild_disband_requires_an_authenticated_connection() {
    let mut h = crate::bank_test_harness::harness().unwrap();
    let master = h.character_id;
    set_character_guild_by_id(h.db(), master, 7, "Disband Guild").unwrap();
    save_guild_member_rank(h.db(), master, 0);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<crate::auth::Authenticated>();
    h.send::<GuildRankRequest, GuildChannel>(request());
    assert_eq!(
        crate::persistence::load_character_guild(h.db(), master)
            .unwrap()
            .guild_id,
        7
    );
}

#[test]
fn guild_disband_authenticated_request_refreshes_both_online_clients() {
    let mut h = crate::bank_test_harness::harness().unwrap();
    let master = h.character_id;
    let member = h.create_alt("Cara");
    for (id, rank) in [(master, 0), (member, 4)] {
        set_character_guild_by_id(h.db(), id, 7, "Disband Guild").unwrap();
        save_guild_member_rank(h.db(), id, rank);
    }
    let peer = h.connect_character(member, "Cara");
    let peer_player = h
        .app
        .world()
        .get::<crate::networking::PlayerLink>(peer)
        .unwrap()
        .0;
    for player in [h.player, peer_player] {
        h.app.world_mut().entity_mut(player).insert((
            shared::components::GuildMembership {
                guild_id: 7,
                guild_name: "Disband Guild".into(),
            },
            shared::guild_creation::GuildTabardDesign {
                icon: 1,
                icon_color: 1,
                border: 1,
                border_color: 1,
                background_color: 1,
            },
        ));
    }
    h.take::<GuildRanksState>();
    h.take_for::<GuildRanksState>(peer);
    h.send_from::<GuildRankRequest, GuildChannel>(peer, request());
    assert_eq!(
        h.take_for::<GuildRanksState>(peer).pop().unwrap().error,
        Some(GuildRankError::Permissions)
    );
    assert_eq!(
        crate::persistence_character::list_guild_members(h.db(), 7).len(),
        2
    );
    h.send::<GuildRankRequest, GuildChannel>(request());
    for state in [
        h.take::<GuildRanksState>().pop().unwrap(),
        h.take_for::<GuildRanksState>(peer).pop().unwrap(),
    ] {
        assert!(state.ranks.is_empty() && state.members.is_empty());
        assert!(state.error.is_none());
    }
    for player in [h.player, peer_player] {
        assert!(
            h.app
                .world()
                .get::<shared::components::GuildMembership>(player)
                .is_none()
        );
        assert!(
            h.app
                .world()
                .get::<shared::guild_creation::GuildTabardDesign>(player)
                .is_none()
        );
    }
    assert!(crate::persistence::load_character_guild(h.db(), member).is_none());
}
