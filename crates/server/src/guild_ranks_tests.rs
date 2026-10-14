use super::*;
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
        let state = snapshot(&db, officer, None);
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
