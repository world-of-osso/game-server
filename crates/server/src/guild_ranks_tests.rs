use super::*;
use crate::guild::guild_ranks::snapshot;
use crate::persistence::banks::{load_guild, load_guild_member_rank, save_guild_member_rank};
use crate::persistence::{create_tables, temp_database};
use crate::persistence_character::{create_character, set_character_guild_by_id};
use shared::protocol::*;

fn members(db: &redb::Database) -> (u64, u64, u64) {
    let ids = ["Master", "Officer", "Member"].map(|name| {
        let (id, _) = create_character(db, 1, name, 1, 1).unwrap();
        set_character_guild_by_id(db, id, 7, "Bank Testers").unwrap();
        id
    });
    for (id, rank) in ids.into_iter().zip([0, 1, 4]) {
        save_guild_member_rank(db, id, rank);
    }
    (ids[0], ids[1], ids[2])
}

fn direct_member_rank_request(name: &str, rank: u8) -> GuildRankRequest {
    serde_json::from_value(serde_json::json!({"SetMemberRank":{"character_name":name,"rank":rank}}))
        .unwrap()
}

#[test]
fn guild_member_rank_assignment_jumps_once_without_fabricated_step_logs_and_reopens() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("direct-rank.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (master, officer, member) = members(&db);
    let before = snapshot(&db, master, None);
    let old_events = crate::guild_event_log::query(&db, master).events;
    assert_eq!(load_guild_member_rank(&db, &load_guild(&db, 7), member), 4);
    apply_rank_request(&db, 7, master, direct_member_rank_request("Member", 1))
        .expect("GM direct multi-rank assignment");
    let guild = load_guild(&db, 7);
    assert_eq!(load_guild_member_rank(&db, &guild, member), 1);
    assert_eq!(load_guild_member_rank(&db, &guild, master), 0);
    assert_eq!(load_guild_member_rank(&db, &guild, officer), 1);
    assert_eq!(snapshot(&db, master, None).ranks, before.ranks);
    assert_eq!(
        crate::guild_event_log::query(&db, master).events,
        old_events
    );
    drop(db);
    let reopened = redb::Database::open(&path).unwrap();
    assert_eq!(
        load_guild_member_rank(&reopened, &load_guild(&reopened, 7), member),
        1
    );
    assert_eq!(
        crate::guild_event_log::query(&reopened, master).events,
        old_events
    );
}

#[test]
fn guild_member_rank_assignment_rechecks_direction_hierarchy_self_and_membership() {
    for (rights, current, target, destination, expected_error) in [
        (GUILD_RIGHT_PROMOTE, 4, "Member", 2, None),
        (GUILD_RIGHT_DEMOTE, 2, "Member", 4, None),
        (
            GUILD_RIGHT_PROMOTE,
            2,
            "Member",
            4,
            Some(GuildRankError::Permissions),
        ),
        (
            GUILD_RIGHT_DEMOTE,
            4,
            "Member",
            2,
            Some(GuildRankError::Permissions),
        ),
        (
            GUILD_RIGHT_PROMOTE,
            4,
            "Member",
            1,
            Some(GuildRankError::RankTooHigh),
        ),
        (
            GUILD_RIGHT_PROMOTE,
            4,
            "Member",
            0,
            Some(GuildRankError::RankTooHigh),
        ),
        (
            GUILD_RIGHT_PROMOTE | GUILD_RIGHT_DEMOTE,
            4,
            "Master",
            3,
            Some(GuildRankError::RankTooHigh),
        ),
        (
            GUILD_RIGHT_DEMOTE,
            4,
            "Officer",
            3,
            Some(GuildRankError::InvalidName),
        ),
        (
            GUILD_RIGHT_DEMOTE,
            4,
            "Member",
            255,
            Some(GuildRankError::RankTooLow),
        ),
        (
            GUILD_RIGHT_PROMOTE,
            4,
            "Outsider",
            2,
            Some(GuildRankError::MemberNotFound),
        ),
        (0, 4, "Member", 2, Some(GuildRankError::Permissions)),
        (GUILD_RIGHT_PROMOTE, 4, "Member", 4, None),
        (
            GUILD_RIGHT_DEMOTE,
            4,
            "Member",
            4,
            Some(GuildRankError::Permissions),
        ),
    ] {
        let db = temp_database();
        let (master, officer, member) = members(&db);
        let (outsider, _) = create_character(&db, 2, "Outsider", 1, 1).unwrap();
        set_character_guild_by_id(&db, outsider, 9, "Other Guild").unwrap();
        save_guild_member_rank(&db, member, current);
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::SetPermissions {
                rank: 1,
                rights,
                gold_per_day: 0,
            },
        )
        .unwrap();
        let before = snapshot(&db, master, None);
        let old_events = crate::guild_event_log::query(&db, master).events;
        let result = apply_rank_request(
            &db,
            7,
            officer,
            direct_member_rank_request(target, destination),
        );
        assert_eq!(
            result.err(),
            expected_error,
            "rights={rights} target={target} current={current} destination={destination}"
        );
        if expected_error.is_some() {
            assert_eq!(snapshot(&db, master, None), before);
        } else {
            assert_eq!(
                load_guild_member_rank(&db, &load_guild(&db, 7), member),
                destination
            );
        }
        assert_eq!(
            crate::guild_event_log::query(&db, master).events,
            old_events
        );
    }
    let db = temp_database();
    let (master, _, _) = members(&db);
    let (unguilded, _) = create_character(&db, 2, "Unguilded", 1, 1).unwrap();
    let before = snapshot(&db, master, None);
    assert_eq!(
        apply_rank_request(&db, 7, unguilded, direct_member_rank_request("Member", 1)),
        Err(GuildRankError::NotInGuild)
    );
    assert_eq!(
        apply_rank_request(&db, 7, master, direct_member_rank_request("Master", 1)),
        Err(GuildRankError::InvalidName)
    );
    assert_eq!(snapshot(&db, master, None), before);
}

#[test]
fn guild_lifecycle_log_records_real_actions_only_and_survives_restart() {
    use crate::guild_event_log::query;
    use crate::persistence_character::{accept_pending_guild_invite, set_pending_guild_invite};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("guild-events.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (master, _, member) = members(&db);
    let (visitor, _) = create_character(&db, 2, "Visitor", 1, 1).unwrap();
    let (other, _) = create_character(&db, 3, "Outsider", 1, 1).unwrap();
    set_character_guild_by_id(&db, other, 9, "Other Guild").unwrap();
    assert!(
        query(&db, master).events.is_empty(),
        "Never reconstruct old history"
    );
    assert_eq!(query(&db, visitor).error, Some(GuildRankError::NotInGuild));
    set_pending_guild_invite(&db, visitor, 7, "Bank Testers", "Master");
    accept_pending_guild_invite(&db, visitor).unwrap();
    assert!(accept_pending_guild_invite(&db, visitor).is_err());
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            member,
            GuildRankRequest::Promote {
                character_name: "Master".into()
            }
        ),
        Err(GuildRankError::Permissions)
    );
    for request in [
        GuildRankRequest::Promote {
            character_name: "Visitor".into(),
        },
        GuildRankRequest::Demote {
            character_name: "Visitor".into(),
        },
        GuildRankRequest::RemoveMember {
            character_name: "Visitor".into(),
        },
        GuildRankRequest::Rename {
            rank: 3,
            name: "Renamed Member".into(),
        },
        GuildRankRequest::Rename {
            rank: 4,
            name: "New Initiate".into(),
        },
    ] {
        apply_rank_request(&db, 7, master, request).unwrap();
    }
    apply_rank_request(&db, 7, member, GuildRankRequest::Leave).unwrap();
    let expected = vec![
        GuildEventKind::Invite {
            actor: "Master".into(),
            member: "Visitor".into(),
        },
        GuildEventKind::Join {
            member: "Visitor".into(),
        },
        GuildEventKind::Promote {
            actor: "Master".into(),
            member: "Visitor".into(),
            rank: "Member".into(),
        },
        GuildEventKind::Demote {
            actor: "Master".into(),
            member: "Visitor".into(),
            rank: "Initiate".into(),
        },
        GuildEventKind::Remove {
            actor: "Master".into(),
            member: "Visitor".into(),
        },
        GuildEventKind::Quit {
            member: "Member".into(),
        },
    ];
    let before = query(&db, master);
    assert_eq!(before.error, None);
    assert_eq!(
        before
            .events
            .iter()
            .map(|event| event.kind.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(
        before
            .events
            .iter()
            .all(|event| event.timestamp > 0 && event.timestamp <= before.server_time)
    );
    assert!(
        query(&db, other).events.is_empty(),
        "Cannot query another guild's history"
    );
    assert_eq!(query(&db, member).error, Some(GuildRankError::NotInGuild));
    assert!(query(&db, member).events.is_empty());
    assert!(crate::persistence::load_character_guild(&db, visitor).is_none());
    assert!(crate::persistence::load_character_guild(&db, member).is_none());
    drop(db);
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    assert_eq!(query(&db, master).events, before.events);
    assert!(crate::persistence::load_character_guild(&db, visitor).is_none());
}

#[test]
fn guild_member_notes_are_filtered_per_authenticated_snapshot_recipient() {
    let db = temp_database();
    let (master, _, member) = members(&db);
    crate::persistence_character::save_guild_public_note(&db, member, "Alchemy");
    crate::persistence_character::save_guild_member_note(
        &db,
        member,
        &crate::persistence_types::GuildMemberNoteData {
            officer_note: "Raid healer".into(),
        },
    );
    let mut world = World::new();
    let mut queries = bevy::ecs::system::SystemState::<OnlinePlayers>::new(&mut world);
    let online = queries.get(&world).expect("read guild online query");
    let guild = shared::components::GuildMembership {
        guild_id: 7,
        guild_name: "Bank Testers".into(),
    };
    for (viewer, private) in [(master, "Raid healer"), (member, "")] {
        let snapshot = visible_guild_snapshot(&db, &guild, &online, viewer);
        let target = snapshot
            .members
            .iter()
            .find(|m| m.character_name == "Member")
            .unwrap();
        assert_eq!(target.public_note, "Alchemy");
        assert_eq!(target.officer_note, private);
        assert_eq!(target.is_self, viewer == member);
        assert_eq!(snapshot.members.iter().filter(|m| m.is_self).count(), 1);
    }
}

#[test]
fn guild_member_notes_preserve_legacy_officer_data_and_public_notes_across_restart() {
    use crate::persistence_character::{
        load_guild_member_note, load_guild_public_note, save_guild_member_note,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (master, officer, member) = members(&db);
    save_guild_member_note(
        &db,
        member,
        &crate::persistence_types::GuildMemberNoteData {
            officer_note: "Raid healer".into(),
        },
    );
    let write = |actor, name: &str, note: &str, officer| {
        apply_rank_request(
            &db,
            7,
            actor,
            GuildRankRequest::SetMemberNote {
                character_name: name.into(),
                note: note.into(),
                officer,
            },
        )
    };
    for rights in [GUILD_RIGHT_CHAT_LISTEN, GUILD_RIGHT_EDIT_PUBLIC_NOTE] {
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::SetPermissions {
                rank: 1,
                rights,
                gold_per_day: 0,
            },
        )
        .unwrap();
        let result = write(officer, "Member", "Officer assigned alchemist", false);
        if rights == GUILD_RIGHT_EDIT_PUBLIC_NOTE {
            result.unwrap();
            assert_eq!(
                load_guild_public_note(&db, member),
                "Officer assigned alchemist"
            );
        } else {
            assert_eq!(result, Err(GuildRankError::Permissions));
        }
        assert_eq!(
            write(officer, "Member", "private without right", true),
            Err(GuildRankError::Permissions)
        );
    }
    // Mainline permits self-public editing without the general public edit right.
    write(member, "Member", "Alchemy 100", false).unwrap();
    assert_eq!(
        write(member, "Officer", "forbidden", false),
        Err(GuildRankError::Permissions)
    );
    assert_eq!(
        write(member, "Member", "forbidden", true),
        Err(GuildRankError::Permissions)
    );
    assert_eq!(
        write(master, "Member", &"界".repeat(32), false),
        Err(GuildRankError::NoteTooLong)
    );
    assert_eq!(
        write(master, "Member", "line\nbreak", false),
        Err(GuildRankError::NoteTooLong)
    );
    write(master, "Member", &"界".repeat(31), false).unwrap();
    assert_eq!(
        load_guild_member_note(&db, member).officer_note,
        "Raid healer"
    );
    write(master, "Officer", "Bank organizer", true).unwrap();
    let outsider = create_character(&db, 2, "Outsider", 1, 1).unwrap().0;
    set_character_guild_by_id(&db, outsider, 8, "Other Guild").unwrap();
    assert_eq!(
        write(master, "Outsider", "forbidden", false),
        Err(GuildRankError::MemberNotFound)
    );
    assert_eq!(
        write(outsider, "Member", "forbidden", false),
        Err(GuildRankError::NotInGuild)
    );
    drop(db);
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    assert_eq!(load_guild_public_note(&db, member), "界".repeat(31));
    assert_eq!(
        load_guild_member_note(&db, member).officer_note,
        "Raid healer"
    );
    assert_eq!(
        load_guild_member_note(&db, officer).officer_note,
        "Bank organizer"
    );
    let guild = shared::components::GuildMembership {
        guild_id: 7,
        guild_name: "Bank Testers".into(),
    };
    assert!(
        update_officer_note(
            &db,
            &guild,
            SetGuildOfficerNote {
                character_name: "Officer".into(),
                note: "x".repeat(32),
            }
        )
        .is_err()
    );
    update_officer_note(
        &db,
        &guild,
        SetGuildOfficerNote {
            character_name: "Officer".into(),
            note: "  Raid healer  ".into(),
        },
    )
    .unwrap();
    assert_eq!(
        load_guild_member_note(&db, officer).officer_note,
        "  Raid healer  "
    );
    apply_rank_request(&db, 7, member, GuildRankRequest::Leave).unwrap();
    set_character_guild_by_id(&db, member, 7, "Bank Testers").unwrap();
    assert_eq!(load_guild_public_note(&db, member), "");
    assert_eq!(load_guild_member_note(&db, member).officer_note, "");
}

#[test]
fn guild_lifecycle_leave_and_remove_enforce_hierarchy_and_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lifecycle.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (master, officer, member) = members(&db);
    assert_eq!(
        apply_rank_request(&db, 7, master, GuildRankRequest::Leave),
        Err(GuildRankError::LeaderCannotLeave)
    );
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            member,
            GuildRankRequest::RemoveMember {
                character_name: "Officer".into()
            }
        ),
        Err(GuildRankError::Permissions)
    );
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            officer,
            GuildRankRequest::RemoveMember {
                character_name: "Master".into()
            }
        ),
        Err(GuildRankError::RankTooHigh)
    );
    save_guild_member_rank(&db, member, 3);
    apply_rank_request(
        &db,
        7,
        officer,
        GuildRankRequest::RemoveMember {
            character_name: "Member".into(),
        },
    )
    .unwrap();
    assert!(crate::persistence::load_character_guild(&db, member).is_none());
    apply_rank_request(&db, 7, officer, GuildRankRequest::Leave).unwrap();
    assert!(crate::persistence::load_character_guild(&db, officer).is_none());
    drop(db);
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    assert!(crate::persistence::load_character_guild(&db, member).is_none());
    assert!(crate::persistence::load_character_guild(&db, officer).is_none());
    assert_eq!(
        crate::persistence_character::list_guild_members(&db, 7).len(),
        1
    );
    // Rejoining must not retain the removed member's previous promotion.
    set_character_guild_by_id(&db, member, 7, "Bank Testers").unwrap();
    assert_eq!(load_guild_member_rank(&db, &load_guild(&db, 7), member), 4);
}

#[test]
fn guild_ranks_officer_chat_toggle_does_not_grant_or_revoke_other_permissions() {
    let db = temp_database();
    let (master, officer, _) = members(&db);
    let independent = GUILD_RIGHT_EDIT_MOTD
        | GUILD_RIGHT_EDIT_INFO
        | GUILD_RIGHT_VIEW_OFFICER_NOTE
        | GUILD_RIGHT_EDIT_OFFICER_NOTE;
    let chat = GUILD_RIGHT_OFFICER_LISTEN | GUILD_RIGHT_OFFICER_SPEAK;
    for rights in [
        independent | GUILD_RIGHT_OFFICER_LISTEN,
        independent | chat,
        independent,
    ] {
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::SetPermissions {
                rank: 1,
                rights,
                gold_per_day: 123_456,
            },
        )
        .unwrap();
        let state = crate::guild::guild_rank_snapshot(&db, officer, None);
        assert_eq!(state.ranks[1].rights, rights);
        assert_eq!(state.ranks[1].gold_per_day, 123_456);
        for right in [
            GUILD_RIGHT_EDIT_MOTD,
            GUILD_RIGHT_EDIT_INFO,
            GUILD_RIGHT_VIEW_OFFICER_NOTE,
            GUILD_RIGHT_EDIT_OFFICER_NOTE,
        ] {
            assert!(character_has_right(&db, officer, 7, right));
        }
        assert_eq!(
            character_has_right(&db, officer, 7, GUILD_RIGHT_OFFICER_LISTEN),
            rights & GUILD_RIGHT_OFFICER_LISTEN != 0
        );
        assert_eq!(
            character_has_right(&db, officer, 7, GUILD_RIGHT_OFFICER_SPEAK),
            rights & GUILD_RIGHT_OFFICER_SPEAK != 0
        );
    }
}

#[test]
fn guild_ranks_master_creates_and_configures_persisted_rank() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guild.redb");
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    let (master, officer, member) = members(&db);
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            officer,
            GuildRankRequest::Add {
                name: "Bank Reader".into()
            }
        ),
        Err(GuildRankError::Permissions)
    );
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::Add {
            name: "Bank Reader".into(),
        },
    )
    .unwrap();
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::SetPermissions {
            rank: 5,
            rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_CHAT_SPEAK | GUILD_RIGHT_WITHDRAW_GOLD,
            gold_per_day: 50_000,
        },
    )
    .unwrap();
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::SetTab {
            rank: 5,
            tab: 0,
            view: true,
            deposit: false,
            withdrawals_per_day: 0,
        },
    )
    .unwrap();
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::Demote {
            character_name: "Member".into(),
        },
    )
    .unwrap();
    let guild = load_guild(&db, 7);
    assert_eq!(guild.ranks[5].name, "Bank Reader");
    assert!(guild.ranks[5].tabs[0].view);
    assert!(!guild.ranks[5].tabs[0].deposit);
    assert_eq!(guild.ranks[5].tabs[0].withdrawals_per_day, 0);
    assert_eq!(load_guild_member_rank(&db, &guild, member), 5);
    drop(db);
    let db = redb::Database::create(&path).unwrap();
    create_tables(&db);
    assert_eq!(load_guild(&db, 7), guild);
    assert_eq!(load_guild_member_rank(&db, &guild, member), 5);
}

#[test]
fn guild_ranks_promote_demote_require_rights_and_strict_hierarchy() {
    let db = temp_database();
    let (master, officer, member) = members(&db);
    let promote = || GuildRankRequest::Promote {
        character_name: "Member".into(),
    };
    apply_rank_request(&db, 7, officer, promote()).unwrap();
    apply_rank_request(&db, 7, officer, promote()).unwrap();
    assert_eq!(
        apply_rank_request(&db, 7, officer, promote()),
        Err(GuildRankError::RankTooHigh)
    );
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            member,
            GuildRankRequest::Demote {
                character_name: "Officer".into()
            }
        ),
        Err(GuildRankError::Permissions)
    );
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::SetPermissions {
            rank: 1,
            rights: GUILD_RIGHT_CHAT_LISTEN,
            gold_per_day: 0,
        },
    )
    .unwrap();
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            officer,
            GuildRankRequest::Demote {
                character_name: "Member".into()
            }
        ),
        Err(GuildRankError::Permissions)
    );
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::Promote {
                character_name: "Master".into()
            }
        ),
        Err(GuildRankError::RankTooHigh)
    );
}

#[test]
fn guild_ranks_reorder_preserves_members_and_rank_zero_is_immutable() {
    let db = temp_database();
    let (master, officer, member) = members(&db);
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::Move { rank: 1, up: false },
    )
    .unwrap();
    let guild = load_guild(&db, 7);
    assert_eq!(guild.ranks[2].name, "Officer");
    assert_eq!(load_guild_member_rank(&db, &guild, officer), 2);
    assert_eq!(load_guild_member_rank(&db, &guild, member), 4);
    assert_eq!(
        apply_rank_request(&db, 7, master, GuildRankRequest::Remove { rank: 2 }),
        Err(GuildRankError::RankInUse)
    );
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::Rename {
                rank: 0,
                name: "Boss".into()
            }
        ),
        Err(GuildRankError::Permissions)
    );
    assert_eq!(
        apply_rank_request(&db, 7, master, GuildRankRequest::Move { rank: 1, up: true }),
        Err(GuildRankError::RankTooHigh)
    );
    apply_rank_request(&db, 7, master, GuildRankRequest::Remove { rank: 1 }).unwrap();
    let guild = load_guild(&db, 7);
    assert_eq!(load_guild_member_rank(&db, &guild, officer), 1);
    assert_eq!(load_guild_member_rank(&db, &guild, member), 3);
}

#[test]
fn guild_ranks_retail_two_to_ten_rank_bounds() {
    let db = temp_database();
    let (master, _, _) = members(&db);
    for i in 5..10 {
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::Add {
                name: format!("Rank {i}"),
            },
        )
        .unwrap();
    }
    assert_eq!(
        apply_rank_request(
            &db,
            7,
            master,
            GuildRankRequest::Add {
                name: "Eleventh".into()
            }
        ),
        Err(GuildRankError::TooManyRanks)
    );
    for rank in (5..10).rev() {
        apply_rank_request(&db, 7, master, GuildRankRequest::Remove { rank }).unwrap();
    }
    // Move the member out of the bottom rank, then delete empty ranks.
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::Promote {
            character_name: "Member".into(),
        },
    )
    .unwrap();
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::Promote {
            character_name: "Member".into(),
        },
    )
    .unwrap();
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::Promote {
            character_name: "Member".into(),
        },
    )
    .unwrap();
    for rank in (2..5).rev() {
        apply_rank_request(&db, 7, master, GuildRankRequest::Remove { rank }).unwrap();
    }
    assert_eq!(
        apply_rank_request(&db, 7, master, GuildRankRequest::Remove { rank: 1 }),
        Err(GuildRankError::TooFewRanks)
    );
}

#[test]
fn guild_ranks_tab_view_toggle_preserves_other_tab_settings() {
    let db = temp_database();
    let (master, _, _) = members(&db);
    apply_rank_request(
        &db,
        7,
        master,
        GuildRankRequest::SetTab {
            rank: 4,
            tab: 0,
            view: false,
            deposit: true,
            withdrawals_per_day: 3,
        },
    )
    .unwrap();
    let guild = load_guild(&db, 7);
    assert!(!guild.ranks[4].tabs[0].view);
    assert!(guild.ranks[4].tabs[0].deposit);
    assert_eq!(guild.ranks[4].tabs[0].withdrawals_per_day, 3);
}
