//! Explicit private-account setup, never a gameplay acceptance test.
use super::*;

#[test]
#[ignore = "creates disposable private escort accounts; requires ESCORT_QUEST_LIVE_DB"]
fn smartai_feero_prepare_private_accounts() {
    use crate::persistence::{quest_store, register_account};
    use crate::persistence_character as character_store;
    let path = std::path::PathBuf::from(std::env::var("ESCORT_QUEST_LIVE_DB").unwrap());
    let parent = path.parent().unwrap().canonicalize().unwrap();
    assert!(parent.ends_with("diagnostics/escort-quests-2026-10-15/private/data"));
    assert!(!path.exists(), "private account database must be fresh");
    let db = redb::Database::create(path).unwrap();
    crate::persistence::create_tables(&db);
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let spawn: (f32, f32, f32) = conn
        .query_row(
            "SELECT position_x,position_y,position_z FROM creature WHERE id1=4484",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    let previous: u32 = conn
        .query_row(
            "SELECT PrevQuestID FROM content_quest_template_addon WHERE ID=976",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(previous, 973);
    let kit = crate::player_create_info::OwnedKit::from_world_db().unwrap();
    for (account, name) in [
        ("fb_escort_modern", "Escortmodern"),
        ("fb_escort_forever", "Escortforevr"),
    ] {
        let (account_id, _) = register_account(&db, account, "fbtest").unwrap();
        let (id, mut character) = character_store::create_character_with_appearance(
            &db,
            account_id,
            name,
            4,
            11,
            Default::default(),
            &kit.kit(),
        )
        .unwrap();
        (character.x, character.y, character.z) =
            crate::terrain_height::wow_to_bevy(spawn.0, spawn.1, spawn.2);
        character.level = 30;
        character_store::save_character(&db, id, &character);
        quest_store::save_quest_log(
            &db,
            id,
            &quest_store::QuestLogData {
                rewarded: vec![previous],
                ..Default::default()
            },
        );
        assert_eq!(quest_store::load_quest_log(&db, id).rewarded, vec![973]);
        println!(
            "PRIVATE_ACCOUNT {account} {name}: level30 Night Elf druid; authored Feero spawn; prerequisite973 seeded, escort976 unaccepted"
        );
    }
}
