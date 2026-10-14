//! Startup boundary: compact layout-2 masks become TC masks without changing capabilities.
use super::*;
use crate::persistence::create_tables;
use shared::protocol::*;

#[test]
fn guild_ranks_layout_two_preserves_each_capability_and_bank_data_across_restart() {
    // Compact layout 2 bits 0..11; expected a352b1fa GuildRankRights values.
    let rights = [
        (GUILD_RIGHT_CHAT_LISTEN, 0x1),
        (GUILD_RIGHT_CHAT_SPEAK, 0x2),
        (GUILD_RIGHT_OFFICER_LISTEN, 0x4),
        (GUILD_RIGHT_OFFICER_SPEAK, 0x8),
        (GUILD_RIGHT_INVITE, 0x10),
        (GUILD_RIGHT_REMOVE, 0x20),
        (GUILD_RIGHT_PROMOTE, 0x80),
        (GUILD_RIGHT_DEMOTE, 0x100),
        (GUILD_RIGHT_EDIT_MOTD, 0x1000),
        (GUILD_RIGHT_EDIT_INFO, 0x10000),
        (GUILD_RIGHT_EDIT_OFFICER_NOTE, 0x8000),
        (GUILD_RIGHT_VIEW_OFFICER_NOTE, 0x4000),
    ];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guild-rights.redb");
    let db = Database::create(&path).unwrap();
    create_tables(&db);
    let (character, _) =
        crate::persistence_character::create_character(&db, 1, "MixedOfficer", 1, 1).unwrap();
    crate::persistence_character::set_character_guild_by_id(&db, character, 7, "Retail Ranks")
        .unwrap();
    save_guild_member_rank(&db, character, 1);
    let mut guild = GuildData::default();
    guild.money = 987_654;
    guild.ranks[1].name = "Custom Officer".into();
    guild.ranks[1].withdraw_gold = false;
    guild.ranks[1].withdraw_repair = true;
    guild.ranks[1].gold_per_day = 123_456;
    guild.ranks[1].tabs[0] = GuildTabRights {
        view: true,
        deposit: false,
        withdrawals_per_day: 3,
    };
    // One guild per old single right plus a deliberately mixed officer mask.
    let masks: Vec<u32> = (0..12)
        .map(|bit| 1 << bit)
        .chain([0, 0xFFF, 0xD09])
        .collect();
    let txn = db.begin_write().unwrap();
    for (index, mask) in masks.iter().enumerate() {
        guild.ranks[1].rights = *mask;
        write(&txn, GUILDS, 7 + index as u32, &guild);
    }
    crate::persistence::set_layout_version(&txn, GUILD_LAYOUT_KEY, 2);
    txn.commit().unwrap();
    drop(db);

    let db = Database::create(&path).unwrap();
    create_tables(&db);
    let migrated: Vec<_> = masks
        .iter()
        .enumerate()
        .map(|(index, mask)| {
            let id = 7 + index as u32;
            let loaded = load_guild(&db, id);
            let expected = rights
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .fold(0, |result, (_, (_, tc))| result | tc);
            assert_eq!(loaded.ranks[1].rights, expected, "old mask {mask:#x}");
            let member = if id == 7 {
                character
            } else {
                let (member, _) = crate::persistence_character::create_character(
                    &db,
                    1,
                    &format!("Rank{}", char::from(b'A' + index as u8)),
                    1,
                    1,
                )
                .unwrap();
                crate::persistence_character::set_character_guild_by_id(
                    &db,
                    member,
                    id,
                    "Retail Ranks",
                )
                .unwrap();
                save_guild_member_rank(&db, member, 1);
                member
            };
            for (bit, (right, _)) in rights.iter().enumerate() {
                assert_eq!(
                    crate::guild::character_has_right(&db, member, id, *right),
                    mask & (1 << bit) != 0
                );
            }
            let mut expected_guild = guild.clone();
            expected_guild.ranks[1].rights = expected;
            assert_eq!(loaded, expected_guild);
            loaded
        })
        .collect();
    assert_eq!(load_guild_member_rank(&db, &migrated[0], character), 1);
    drop(db);
    let db = Database::create(&path).unwrap();
    create_tables(&db);
    for (index, expected) in migrated.iter().enumerate() {
        assert_eq!(&load_guild(&db, 7 + index as u32), expected);
    }
}
