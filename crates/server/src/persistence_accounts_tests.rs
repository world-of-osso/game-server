use super::*;
use crate::persistence::{
    create_character, delete_character, list_characters_for_account, load_character,
    load_character_guild, save_character, set_character_guild_by_name,
    set_pending_guild_invite, take_pending_guild_invite, temp_database,
};

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
fn list_accounts_returns_usernames_hashes_and_token_counts() {
    let db = temp_database();
    let (acct1, token1) = register_account(&db, "user1", "pass1").unwrap();
    let (acct2, _token2) = register_account(&db, "user2", "pass2").unwrap();
    let (_acct1_again, _new_token1) =
        login_with_credentials(&db, None, "user1", "pass1").unwrap();

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
        }
    );
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
    assert!((data_human.x - (-8949.0)).abs() < 1.0, "human x");
    assert!((data_human.y - (-132.0)).abs() < 1.0, "human y");

    // Orc (race 2) starts at Valley of Trials — different from Human
    let (_, data_orc) = create_character(&db, account_id, "OrcWarrior", 2, 1).unwrap();
    assert!((data_orc.x - (-618.0)).abs() < 1.0, "orc x");
    assert!((data_orc.y - (-4251.0)).abs() < 1.0, "orc y");
    assert!((data_orc.x - data_human.x).abs() > 100.0, "different start zones");

    // Night Elf (race 4) starts at Shadowglen
    let (_, data_nelf) = create_character(&db, account_id, "ElfDruid", 4, 1).unwrap();
    assert!((data_nelf.x - 10311.0).abs() < 1.0, "night elf x");

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
    assert!(data.x != 0.0 || data.y != 0.0 || data.z != 0.0, "position set");

    // Inventory should have starter items
    let inv = crate::persistence::load_inventory(&db, cid);
    assert!(!inv.items.is_empty(), "starter items present");
    assert!(inv.gold > 0, "starter gold present");

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
