use super::*;
use crate::persistence::{
    create_character, delete_character, list_characters_for_account, load_character,
    load_character_guild, save_character, set_character_guild_by_name, set_pending_guild_invite,
    take_pending_guild_invite, temp_database,
};

#[test]
fn race_source_admin_dwarf_shaman_uses_authentic_kit_and_persists() {
    let database = crate::persistence::GameDatabase::new(temp_database());
    let (account, _) = register_account(&database.db, "raceclassdb2", "testpass").unwrap();
    let owned = crate::player_create_info::OwnedKit::from_world_db().expect("native world data");
    let response = crate::admin_handlers::handle_create_character(
        &database,
        &owned.kit(),
        "raceclassdb2",
        "Dwarfshaman",
        3,
        7,
    );
    let crate::admin_types::AdminResponse::CharacterCreated { character_id, .. } = response else {
        panic!("source-valid Dwarf/Shaman admin creation failed: {response:?}");
    };
    let saved = load_character(&database.db, character_id).expect("saved Dwarf Shaman");
    assert_eq!((saved.race, saved.class, saved.level), (3, 7, 1));
    let location = owned.kit().starting_location(3, 7).unwrap();
    assert_eq!(
        (saved.x, saved.y, saved.z),
        (location.x, location.y, location.z)
    );
    let inventory =
        crate::persistence::inventory_store::load_inventory_record(&database.db, character_id)
            .unwrap();
    assert!(
        !inventory.equipped.is_empty(),
        "authentic source starting equipment"
    );
    assert_eq!(list_characters_for_account(&database.db, account).len(), 1);
}

#[test]
fn register_and_login_by_token() {
    let db = temp_database();
    let (id1, token) = register_account(&db, "user1", "pass123").unwrap();
    assert!(id1 > 0);

    let (id2, token2) = login_with_credentials(&db, Some(&token), "user1", "").unwrap();
    assert_eq!(id1, id2);
    assert_eq!(token, token2);
}

#[test]
fn login_with_password() {
    let db = temp_database();
    let (id1, _) = register_account(&db, "user1", "secret").unwrap();

    let (id2, _) = login_with_credentials(&db, None, "user1", "secret").unwrap();
    assert_eq!(id1, id2);
}

#[test]
fn login_wrong_password_fails() {
    let db = temp_database();
    register_account(&db, "user1", "correct").unwrap();

    let result = login_with_credentials(&db, None, "user1", "wrong");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid password"));
}

#[test]
fn login_unknown_user_fails() {
    let db = temp_database();
    let result = login_with_credentials(&db, None, "nobody", "pass");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn login_with_invalid_token_rejected() {
    let db = temp_database();
    register_account(&db, "user1", "pass").unwrap();

    // Invalid token, no username/password → falls through to username lookup, fails
    let result = login_with_credentials(&db, Some("bogus-token-12345"), "", "");
    assert!(result.is_err());

    // Invalid token with wrong username also fails
    let result = login_with_credentials(&db, Some("fake"), "nobody", "wrong");
    assert!(result.is_err());
}

#[test]
fn password_stored_as_argon2_hash_not_plaintext() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "mysecret").unwrap();

    // Read raw hash from PASSWORDS table
    let txn = db.begin_read().unwrap();
    let table = txn.open_table(PASSWORDS).unwrap();
    let stored = table.get(account_id).unwrap().unwrap();
    let hash_str = std::str::from_utf8(stored.value()).unwrap();

    // Must not be plaintext
    assert_ne!(hash_str, "mysecret");
    // Must be argon2 format ($argon2...)
    assert!(
        hash_str.starts_with("$argon2"),
        "hash should be argon2 format, got: {hash_str}",
    );
}

#[test]
fn account_ids_are_unique_across_rapid_creates() {
    let db = temp_database();
    let mut ids = std::collections::HashSet::new();
    for i in 0..20 {
        let name = format!("user{i}");
        let (account_id, _) = register_account(&db, &name, "pass").unwrap();
        assert!(
            ids.insert(account_id),
            "duplicate account_id {account_id} at iteration {i}",
        );
    }
    assert_eq!(ids.len(), 20);
}

#[test]
fn register_duplicate_username_fails() {
    let db = temp_database();
    register_account(&db, "user1", "pass").unwrap();
    let result = register_account(&db, "user1", "other");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already taken"));
}

#[test]
fn pending_registration_requires_approval_before_login() {
    let db = temp_database();
    request_registration(&db, "user1", "secret").unwrap();

    let result = login_with_credentials(&db, None, "user1", "secret");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("pending admin approval"));
}

#[test]
fn banned_account_is_rejected_for_password_and_token_login() {
    let db = temp_database();
    let (_account_id, token) = register_account(&db, "user1", "secret").unwrap();
    ban_account(&db, "user1", "cheating").unwrap();

    let password_login = login_with_credentials(&db, None, "user1", "secret");
    assert_eq!(password_login.unwrap_err(), "Account banned: cheating");

    let token_login = login_with_credentials(&db, Some(&token), "user1", "");
    assert_eq!(token_login.unwrap_err(), "Account banned: cheating");
}

#[test]
fn list_accounts_returns_usernames_hashes_and_token_counts() {
    let db = temp_database();
    let (acct1, token1) = register_account(&db, "user1", "pass1").unwrap();
    let (acct2, _token2) = register_account(&db, "user2", "pass2").unwrap();
    let (_acct1_again, _new_token1) = login_with_credentials(&db, None, "user1", "pass1").unwrap();

    let accounts = list_accounts(&db);

    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].username, "user1");
    assert_eq!(accounts[0].account_id, acct1);
    assert_eq!(accounts[0].token_count, 2);
    assert_eq!(accounts[1].username, "user2");
    assert_eq!(accounts[1].account_id, acct2);
    assert_eq!(accounts[1].token_count, 1);

    assert!(find_account_by_token(&db, &token1).is_some());
}

#[test]
fn inspect_account_reports_missing_existing_and_pending_users() {
    let db = temp_database();
    let (account_id, _token) = register_account(&db, "user1", "pass1").unwrap();
    request_registration(&db, "pending_user", "pass2").unwrap();

    let existing = inspect_account(&db, "user1");
    assert_eq!(
        existing,
        AccountInspection {
            username: "user1".to_string(),
            exists: true,
            account_id: Some(account_id),
            token_count: 1,
            pending_registration: false,
            banned: false,
            ban_reason: None,
        }
    );

    assert_eq!(
        inspect_account(&db, "missing"),
        AccountInspection {
            username: "missing".to_string(),
            exists: false,
            account_id: None,
            token_count: 0,
            pending_registration: false,
            banned: false,
            ban_reason: None,
        }
    );
    assert_eq!(
        inspect_account(&db, "pending_user"),
        AccountInspection {
            username: "pending_user".to_string(),
            exists: false,
            account_id: None,
            token_count: 0,
            pending_registration: true,
            banned: false,
            ban_reason: None,
        }
    );
}

#[test]
fn inspect_account_reports_ban_state() {
    let db = temp_database();
    let (account_id, _token) = register_account(&db, "user1", "pass1").unwrap();
    ban_account(&db, "user1", "toxicity").unwrap();

    assert_eq!(
        inspect_account(&db, "user1"),
        AccountInspection {
            username: "user1".to_string(),
            exists: true,
            account_id: Some(account_id),
            token_count: 1,
            pending_registration: false,
            banned: true,
            ban_reason: Some("toxicity".to_string()),
        }
    );
}

#[test]
fn unban_account_restores_login() {
    let db = temp_database();
    register_account(&db, "user1", "secret").unwrap();
    ban_account(&db, "user1", "appeal pending").unwrap();
    unban_account(&db, "user1").unwrap();

    let (_account_id, _token) = login_with_credentials(&db, None, "user1", "secret").unwrap();
}

#[test]
fn check_password_validates_existing_credentials() {
    let db = temp_database();
    register_account(&db, "admin", "admin").unwrap();

    assert!(check_password(&db, "admin", "admin").unwrap());
    assert!(!check_password(&db, "admin", "wrong").unwrap());
    assert!(!check_password(&db, "missing", "admin").unwrap());
}

#[test]
fn approve_registration_activates_account() {
    let db = temp_database();
    request_registration(&db, "user1", "secret").unwrap();

    let (account_id, token) = approve_registration(&db, "user1").unwrap();
    assert!(account_id > 0);
    assert!(!token.is_empty());

    let (login_id, _new_token) = login_with_credentials(&db, None, "user1", "secret").unwrap();
    assert_eq!(account_id, login_id);
}

#[test]
fn set_account_credentials_links_username() {
    let db = temp_database();
    let (account_id, _token) = register_account(&db, "user1", "pass").unwrap();
    // Link a second username to same account via token (simulates token-only account getting credentials)
    // First create a fresh token-only account by manually inserting
    let token2 = uuid::Uuid::new_v4().to_string();
    let txn = db.begin_write().unwrap();
    {
        let mut accounts = txn.open_table(ACCOUNTS).unwrap();
        accounts.insert(token2.as_str(), account_id + 100).unwrap();
    }
    txn.commit().unwrap();

    let result = set_account_credentials(&db, &token2, "newuser", "newpass");
    assert_eq!(result.unwrap(), account_id + 100);

    let found = find_account_by_username(&db, "newuser");
    assert_eq!(found, Some(account_id + 100));
}

#[test]
fn set_account_credentials_rejects_taken_username() {
    let db = temp_database();
    let (_, token) = register_account(&db, "user1", "pass").unwrap();
    let result = set_account_credentials(&db, &token, "user1", "pass");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already taken"));
}

#[test]
fn set_account_credentials_rejects_bad_token() {
    let db = temp_database();
    let result = set_account_credentials(&db, "bad-token", "user1", "pass");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Token not found"));
}

#[test]
fn create_and_list_characters() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let (cid, data) = create_character(&db, account_id, "Hero", 1, 2).unwrap();
    assert_eq!(data.name, "Hero");
    let list = list_characters_for_account(&db, account_id);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, cid);
}

#[test]
fn create_character_rejects_duplicate_name() {
    let db = temp_database();
    let (acct1, _) = register_account(&db, "user1", "pass").unwrap();
    let (acct2, _) = register_account(&db, "user2", "pass").unwrap();
    create_character(&db, acct1, "UniqueHero", 1, 1).unwrap();
    let result = create_character(&db, acct2, "UniqueHero", 2, 2);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already taken"));
}

#[test]
fn create_character_rejects_invalid_race() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let result = create_character(&db, account_id, "BadRace", 255, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid race id"));
}

#[test]
fn create_character_rejects_invalid_class_and_race_class_combination() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    assert_eq!(
        create_character(&db, account_id, "Adventurer", 1, 14),
        Err("Invalid class id 14".to_string())
    );
    assert_eq!(
        create_character(&db, account_id, "Druid", 3, 11),
        Err("Class 11 is not available for race 3".to_string())
    );
    assert!(list_characters_for_account(&db, account_id).is_empty());
}

#[test]
fn create_character_rejects_name_too_short() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    assert!(create_character(&db, account_id, "A", 1, 1).is_err());
    assert!(create_character(&db, account_id, "", 1, 1).is_err());
    // Exactly 2 chars should succeed
    assert!(create_character(&db, account_id, "Ab", 1, 1).is_ok());
}

#[test]
fn create_character_rejects_name_too_long() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    assert!(create_character(&db, account_id, "Abcdefghijklm", 1, 1).is_err()); // 13 chars
    // Exactly 12 chars should succeed
    assert!(create_character(&db, account_id, "Abcdefghijkl", 1, 1).is_ok());
}

#[test]
fn create_character_rejects_special_characters() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    assert!(create_character(&db, account_id, "Hero!", 1, 1).is_err());
    assert!(create_character(&db, account_id, "Hero 1", 1, 1).is_err());
    assert!(create_character(&db, account_id, "Héro", 1, 1).is_err());
    assert!(create_character(&db, account_id, "My-Hero", 1, 1).is_err());
    assert!(create_character(&db, account_id, "Hero123", 1, 1).is_err());
    // Pure letters accepted
    assert!(create_character(&db, account_id, "Arthas", 1, 1).is_ok());
}

#[test]
fn max_characters_per_account_enforced() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    for i in 0..10 {
        let name = format!("Hero{}", (b'a' + i) as char);
        create_character(&db, account_id, &name, 1, 1).unwrap();
    }
    let result = create_character(&db, account_id, "Eleventh", 1, 1);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Maximum characters"));
    assert_eq!(list_characters_for_account(&db, account_id).len(), 10);
}

#[test]
fn delete_character_removes_from_all_tables() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let (cid, _) = create_character(&db, account_id, "ToDelete", 1, 1).unwrap();
    delete_character(&db, account_id, cid).unwrap();
    assert!(crate::persistence::load_character(&db, cid).is_none());
    assert!(list_characters_for_account(&db, account_id).is_empty());
    assert!(create_character(&db, account_id, "ToDelete", 1, 1).is_ok());
}

#[test]
fn delete_character_rejects_wrong_account() {
    let db = temp_database();
    let (acct1, _) = register_account(&db, "user1", "pass").unwrap();
    let (acct2, _) = register_account(&db, "user2", "pass").unwrap();
    let (cid, _) = create_character(&db, acct1, "MyChar", 1, 1).unwrap();
    let result = delete_character(&db, acct2, cid);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not belong"));
}

#[test]
fn delete_nonexistent_character_returns_error() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let result = delete_character(&db, account_id, 99999);
    assert!(result.is_err());
}

#[test]
fn character_creation_assigns_starting_position_per_race() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();

    // Human (race 1) starts at Northshire
    let (cid_human, data_human) = create_character(&db, account_id, "HumanHero", 1, 1).unwrap();
    assert_eq!(
        (data_human.x, data_human.y, data_human.z),
        (-8914.57, -133.909, 80.5378)
    );

    // Orc (race 2) starts at Valley of Trials — different from Human
    let (_, data_orc) = create_character(&db, account_id, "OrcWarrior", 2, 1).unwrap();
    assert_eq!(
        (data_orc.x, data_orc.y, data_orc.z),
        (-618.518, -4251.67, 38.718)
    );
    assert!(
        (data_orc.x - data_human.x).abs() > 100.0,
        "different start zones"
    );

    // Night Elf (race 4) starts at Shadowglen
    let (_, data_nelf) = create_character(&db, account_id, "ElfDruid", 4, 1).unwrap();
    assert_eq!(
        (data_nelf.x, data_nelf.y, data_nelf.z),
        (10311.3, 831.463, 1326.57)
    );

    // Persisted position matches
    let loaded = load_character(&db, cid_human).unwrap();
    assert!((loaded.x - data_human.x).abs() < 0.01);
}

#[test]
fn character_guild_membership_round_trip() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let (character_id, _) = create_character(&db, account_id, "Guildy", 1, 1).unwrap();
    set_character_guild_by_name(&db, "Guildy", 42, "Forty Two").unwrap();
    let guild = load_character_guild(&db, character_id).expect("guild should exist");
    assert_eq!(guild.guild_id, 42);
    assert_eq!(guild.guild_name, "Forty Two");
}

#[test]
fn pending_guild_invite_is_consumed_once() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let (character_id, _) = create_character(&db, account_id, "Invitee", 1, 1).unwrap();
    set_pending_guild_invite(&db, character_id, 7, "Lucky Seven", "Inviter");
    let invite = take_pending_guild_invite(&db, character_id).expect("invite should exist");
    assert_eq!(invite.guild_id, 7);
    assert_eq!(invite.guild_name, "Lucky Seven");
    assert_eq!(invite.inviter_name, "Inviter");
    assert!(take_pending_guild_invite(&db, character_id).is_none());
}

#[test]
fn set_level_persists_via_save_and_load() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let (cid, _) = create_character(&db, account_id, "Leveler", 1, 1).unwrap();
    let mut data = load_character(&db, cid).unwrap();
    assert_eq!(data.level, 1);
    data.level = 42;
    save_character(&db, cid, &data);
    let reloaded = load_character(&db, cid).unwrap();
    assert_eq!(reloaded.level, 42);
}

#[test]
fn character_load_populates_all_fields() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "user1", "pass").unwrap();
    let (cid, _) = create_character(&db, account_id, "FullLoad", 1, 2).unwrap();

    let data = load_character(&db, cid).unwrap();
    assert_eq!(data.name, "FullLoad");
    assert_eq!(data.race, 1);
    assert_eq!(data.class, 2);
    assert_eq!(data.level, 1);
    assert!(data.max_health > 0.0, "max_health populated");
    // Position should be set to starting zone
    assert!(
        data.x != 0.0 || data.y != 0.0 || data.z != 0.0,
        "position set"
    );

    // New characters start with empty bags and no money.
    let inv = crate::persistence::load_inventory(&db, cid);
    assert_eq!(inv, crate::persistence::InventoryData::default());

    // Guild defaults to none
    let guild = load_character_guild(&db, cid);
    assert!(guild.is_none(), "no guild by default");
}

#[test]
fn register_empty_username_rejected() {
    let db = temp_database();
    assert!(register_account(&db, "", "pass").is_err());
    assert!(register_account(&db, "  ", "pass").is_err());
}

#[test]
fn register_empty_password_rejected() {
    let db = temp_database();
    assert!(register_account(&db, "user1", "").is_err());
}

#[test]
fn set_credentials_enables_password_login() {
    let db = temp_database();
    let (_, _token) = register_account(&db, "tokenuser", "oldpass").unwrap();
    let (_, token2) = register_account(&db, "other", "pass").unwrap();
    // token2's account has no username "aliasuser" yet
    set_account_credentials(&db, &token2, "aliasuser", "newpass").unwrap();
    let (account_id, _) = login_with_credentials(&db, None, "aliasuser", "newpass").unwrap();
    let (other_id, _) = login_with_credentials(&db, Some(&token2), "", "").unwrap();
    assert_eq!(account_id, other_id);
}

#[test]
fn new_class_monk_characters_cover_additional_authored_races() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "fb_moremonks", "fbtest").unwrap();
    for (name, race, level, bit, faction) in [
        ("Kulmonk", 32, 10, 31, 1),
        ("Earthenhorde", 84, 10, 17, 2),
        ("Earthenally", 85, 10, 18, 3),
        // IsAlliedRace: 10 (Player::GetStartLevel), whatever ChrRaces.StartingLevel (1).
        ("Haranirally", 86, 10, 20, 4),
        ("Haranirhorde", 91, 10, 19, 116),
    ] {
        let (id, data) = create_character(&db, account_id, name, race, 10).unwrap();
        assert_eq!(data.level, level);
        assert_eq!(load_character(&db, id).unwrap().level, level);
        assert!(crate::race_class::race_mask_allows([1 << bit, 0], race));
        assert!(!crate::race_class::race_mask_allows([1 << 3, 0], race));
        assert_eq!(crate::faction::faction_template_for_race(race), faction);
        // Race starts are not implemented: the faction capital stands in.
        let capital = if faction == 1 || faction == 3 || faction == 4 {
            (-8841.06, 489.656)
        } else {
            (1798.27, -4363.27)
        };
        assert_eq!((data.x, data.y), capital, "{name}");
        for class in [12, 13] {
            assert!(crate::race_class::validate_race_class(race, class).is_err());
        }
    }
    assert_eq!(list_characters_for_account(&db, account_id).len(), 5);
}

/// Monk, Demon Hunter and Evoker creation: start level, map and position, and
/// the refused race/class combinations leave no character behind.
#[test]
fn new_class_characters_start_at_their_level_and_location() {
    use crate::maps::persistence::load_character_map;

    let db = temp_database();
    let (account_id, _) = register_account(&db, "fb_newclass", "fbtest").unwrap();

    let (monk, data) = create_character(&db, account_id, "Monkling", 1, 10).unwrap();
    assert_eq!((data.level, data.x, data.y), (1, -8914.57, -133.909));
    assert_eq!(load_character_map(&db, monk), 0);

    let (dh, data) = create_character(&db, account_id, "Illidariel", 4, 12).unwrap();
    assert_eq!((data.level, data.x, data.y), (8, 10311.3, 831.463));
    assert_eq!(load_character_map(&db, dh), 1, "Shadowglen is on Kalimdor");

    let (evoker, data) = create_character(&db, account_id, "Scalecommand", 52, 13).unwrap();
    assert_eq!((data.level, data.x, data.y), (10, -8841.06, 489.656));
    assert_eq!(load_character_map(&db, evoker), 0, "Stormwind");

    let (evoker, data) = create_character(&db, account_id, "Scaleborn", 70, 13).unwrap();
    assert_eq!((data.level, data.x, data.y), (10, 1798.27, -4363.27));
    assert_eq!(load_character_map(&db, evoker), 1, "Orgrimmar");

    for (name, race, class, level) in [
        ("Bloodhunter", 10, 12, 8),
        ("Voidhunter", 29, 12, 10),
        ("Alliedmonk", 37, 10, 10),
    ] {
        let (id, data) = create_character(&db, account_id, name, race, class).unwrap();
        assert_eq!(data.level, level);
        assert_eq!(load_character(&db, id).unwrap().level, level);
    }

    for (name, race, class) in [
        ("Humandh", 1, 12),
        ("Orcvoker", 2, 13),
        ("Dracmonk", 52, 10),
    ] {
        assert_eq!(
            create_character(&db, account_id, name, race, class).map(|(id, _)| id),
            Err(format!("Class {class} is not available for race {race}"))
        );
    }
    assert_eq!(list_characters_for_account(&db, account_id).len(), 7);
    assert_eq!(load_character(&db, dh).unwrap().level, 8);
}

/// Player::GetStartLevel (Player.cpp:24384-24398): a Death Knight is created at 8, an
/// allied race at 10 whatever its class, and a Human Warrior at 1.
#[test]
fn death_knights_and_allied_races_start_at_their_trinitycore_levels() {
    let db = temp_database();
    let (account_id, _) = register_account(&db, "fb_startlevel", "fbtest").unwrap();
    for (name, race, class, level) in [
        ("Deathlord", 1, 6, 8),
        ("Voidmage", 29, 8, 10),
        ("Ironknight", 34, 6, 10),
        ("Plainwarrior", 1, 1, 1),
    ] {
        let (id, data) = create_character(&db, account_id, name, race, class).unwrap();
        assert_eq!(data.level, level, "{name}");
        assert_eq!(load_character(&db, id).unwrap().level, level, "{name}");
    }
}

/// A Dracthyr stores its visage form with its dragon form; another race may not send one.
#[test]
fn dracthyr_keep_their_visage_form_and_other_races_cannot_send_one() {
    use shared::components::{CharacterAppearance, CustomizationChoiceSelection, FormAppearance};

    let db = temp_database();
    let (account_id, _) = register_account(&db, "fb_visage", "fbtest").unwrap();
    let appearance = CharacterAppearance {
        sex: 1,
        skin_color: 2,
        visage: Some(FormAppearance {
            hair_style: 3,
            customization_choices: vec![CustomizationChoiceSelection {
                option_id: 2064,
                choice_id: 30012,
            }],
            ..FormAppearance::default()
        }),
        ..CharacterAppearance::default()
    };
    let (id, _) = crate::persistence::create_character_with_appearance(
        &db,
        account_id,
        "Scalesong",
        52,
        13,
        appearance.clone(),
        &crate::player_create_info::OwnedKit::bare().kit(),
    )
    .unwrap();
    assert_eq!(load_character(&db, id).unwrap().appearance, appearance);
    assert_eq!(
        crate::persistence::create_character_with_appearance(
            &db,
            account_id,
            "Humanvisage",
            1,
            1,
            appearance,
            &crate::player_create_info::OwnedKit::bare().kit(),
        )
        .map(|(id, _)| id),
        Err("Race 1 has no visage form".to_string())
    );
}
