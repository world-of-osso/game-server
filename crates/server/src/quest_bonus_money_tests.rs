//! Ordinary quest62 through actual exploration, owner messages and durable reward storage.
use super::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};

const BASE_COPPER: u32 = 105_000;
const BONUS_COPPER: u32 = 120;

fn qualify_native_bonus_rows() {
    let connection = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    for (id, bonus, suppressed) in [(62, 120, false), (54083, 46200, true), (35, 0, false)] {
        let row = connection
            .query_row(
                "SELECT RewardBonusMoney,Flags FROM tdb_quest_template WHERE ID=?1",
                [id],
                |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)),
            )
            .unwrap();
        assert_eq!(row.0, bonus, "native quest{id} authored bonus");
        assert_eq!(row.1 & 0x100 != 0, suppressed, "native quest{id} flag");
    }
    let tuning = connection
        .query_row(
            "SELECT l.maxlevel,m.Difficulty_5 FROM content_tuning_level l
             JOIN quest_money_reward m ON m.ID=l.maxlevel WHERE l.ID=73",
            [],
            |row| Ok((row.get::<_, u8>(0)?, row.get::<_, u32>(1)?)),
        )
        .unwrap();
    assert_eq!(tuning, (30, BASE_COPPER));
}

fn bonus_app(path: &std::path::Path) -> App {
    let mut app = setup_app_with(|app| {
        app.insert_resource(crate::test_data::achievement_catalog());
        app.add_plugins(AchievementPlugin);
    })
    .expect("authentic ordinary quest fixture");
    app.insert_resource(GameDatabase::new(redb::Database::create(path).unwrap()));
    persistence::create_tables(db(&app));
    app
}

fn explore_fargodeep(app: &mut App, client: &Client) {
    let (x, y, z) = FARGODEEP_TRIGGER;
    walk_to(app, client, (x + 20.0, y, z));
    walk_to(app, client, FARGODEEP_TRIGGER);
    assert!(
        load_quest_log(db(app), client.character_id)
            .active(62)
            .is_some()
    );
    assert_eq!(entry_completed(app, client, 62), Some(true));
    walk_to(app, client, (0.0, 0.0, 0.0));
}

fn ordinary_bonus_turn_in(level: u8, expected: u32) {
    qualify_native_bonus_rows();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ordinary-bonus.redb");
    let mut app = bonus_app(&path);
    let client = spawn_client(&mut app, level);
    app.world_mut()
        .entity_mut(client.player)
        .insert(PlayerAchievementState::default());
    add_message_pair::<AchievementStateUpdate>(&mut app, client.connection);
    add_message_pair::<AchievementDelta>(&mut app, client.connection);
    enter_world(&mut app, &client);
    let giver = spawn_npc(&mut app, MARSHAL_DUGHAN, 0.0);
    request(
        &mut app,
        &client,
        QuestGiverQueryQuest {
            npc: giver,
            quest_id: 62,
        },
    );
    let details = received::<QuestGiverQuestDetails>(&mut app, &client)
        .pop()
        .unwrap();
    assert_eq!(
        details.rewards.money, BASE_COPPER,
        "source BuildQuestRewards stays base"
    );
    accept(&mut app, &client, giver, 62);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let accepted = last_update(&mut app, &client);
    assert_eq!(accepted.changed[0].rewards.money, BASE_COPPER);
    let before = persistence::load_inventory(db(&app), client.character_id).gold;
    turn_in(&mut app, &client, giver, 62, None);
    assert!(!received::<QuestFailed>(&mut app, &client).is_empty());
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id).gold,
        before
    );
    assert!(received::<QuestGiverQuestComplete>(&mut app, &client).is_empty());
    explore_fargodeep(&mut app, &client);
    request(
        &mut app,
        &client,
        QuestGiverCompleteQuest {
            npc: giver,
            quest_id: 62,
        },
    );
    let offer = received::<QuestGiverOfferReward>(&mut app, &client)
        .pop()
        .unwrap();
    assert_eq!(offer.rewards.money, BASE_COPPER, "offer stays base");
    turn_in(&mut app, &client, giver, 62, None);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let complete = received::<QuestGiverQuestComplete>(&mut app, &client)
        .pop()
        .unwrap();
    let paid = persistence::load_inventory(db(&app), client.character_id).gold - before;
    eprintln!(
        "ordinary62 player{level}: packet={}, persisted_delta={paid}, xp={}",
        complete.money, complete.xp
    );
    assert_eq!(
        complete.money, expected,
        "actual ordinary completed settlement"
    );
    assert_eq!(paid, u64::from(expected));
    if level == crate::experience::MAX_LEVEL {
        assert_eq!(complete.xp, 0);
        assert!(received::<LogXpGain>(&mut app, &client).is_empty());
    }
    crate::test_data::drain_persistence(app.world().resource::<GameDatabase>());
    let achievements = persistence::load_achievements(db(&app), client.character_id);
    assert_eq!(
        achievements.criteria_progress.get(&4092),
        Some(&u64::from(expected))
    );
    turn_in(&mut app, &client, giver, 62, None);
    assert!(!received::<QuestFailed>(&mut app, &client).is_empty());
    assert!(received::<QuestGiverQuestComplete>(&mut app, &client).is_empty());
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id).gold,
        before + u64::from(expected)
    );
    let character = client.character_id;
    crate::test_data::drain_persistence(app.world().resource::<GameDatabase>());
    drop(app);
    let reopened = GameDatabase::new(redb::Database::open(&path).unwrap());
    assert_eq!(
        persistence::load_inventory(&reopened.db, character).gold,
        before + u64::from(expected)
    );
    assert!(load_quest_log(&reopened.db, character).is_rewarded(62));
    assert_eq!(
        persistence::load_achievements(&reopened.db, character)
            .criteria_progress
            .get(&4092),
        Some(&u64::from(expected))
    );
}

#[test]
#[ignore = "pushed native regression: player-cap bonus settlement missing"]
fn ordinary_quest_bonus_at_player_cap_pays_authored_money_once_and_survives_reopen() {
    ordinary_bonus_turn_in(crate::experience::MAX_LEVEL, BASE_COPPER + BONUS_COPPER);
}

#[test]
#[ignore = "required bonus catalog must fail startup explicitly when missing"]
fn ordinary_quest_bonus_missing_runtime_catalog_is_an_explicit_load_error() {
    let directory = tempfile::tempdir().unwrap();
    let world = directory.path().join("world.db");
    std::os::unix::fs::symlink(crate::test_data::world_db_path(), &world).unwrap();
    let error = crate::quest_data::load_quest_db(&world)
        .err()
        .expect("missing required quest-reward-bonus.json must not load with zero bonuses");
    assert!(
        error.to_string().contains("quest-reward-bonus.json"),
        "{error}"
    );
}

#[test]
#[ignore = "control bundled with native bonus RED"]
fn ordinary_quest_bonus_below_player_cap_above_quest_ceiling_keeps_base_money() {
    ordinary_bonus_turn_in(crate::experience::MAX_LEVEL - 1, BASE_COPPER);
}
