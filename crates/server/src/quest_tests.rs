//! Quest flow through the real message path, with world.db quest rows and a
//! temp redb; each test skips when world.db is absent.
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Connected, Message, PeerId, RemoteId};
use shared::components::{CharacterAppearance, EquipmentAppearance};
use shared::protocol::{
    AbandonQuest, AchievementCatalogPage, AchievementDelta, AchievementStateUpdate, DestroyItem,
    EquipItem, EquipmentSnapshot, InventoryDelta, InventoryError, InventorySnapshot, LogXpGain,
    PlayerXpUpdate, QueryAchievementCatalog, QuestFailed, QuestFailedReason, QuestGiverAcceptQuest,
    QuestGiverChooseReward, QuestGiverCompleteQuest, QuestGiverHello, QuestGiverOfferReward,
    QuestGiverQueryQuest, QuestGiverQuestComplete, QuestGiverQuestDetails, QuestGiverQuestList,
    QuestGiverQuestState, QuestGiverRequestItems, QuestGiverStatus, QuestGiverStatusEntry,
    QuestGiverStatusQuery, QuestMarkerClass, QuestRewardItem, SetQuestWatched, SortBags, SplitItem,
    SwapItem, UseItem, XpGainReason,
};

use super::*;
use crate::combat::DeathState;
use crate::item_templates::{ItemTemplates, load_item_templates};
use crate::persistence::InventoryItemData;
use crate::quest_data::test_quest_db;

#[path = "quest_bonus_money_tests.rs"]
mod bonus_money_tests;
#[path = "quest_max_count_tests.rs"]
mod max_count_tests;

const DEPUTY_WILLEM: u32 = 823;
const MARSHAL_MCBRIDE: u32 = 197;
const DEPUTY_RAINER: u32 = 963;
const BLACKROCK_WORG: u32 = 49871;
// Retail Northshire's warrior chain (TrinityCore; Cataclysm removed A Threat Within,
// Kobold Camp Cleanup and Brotherhood of Thieves).
const BEATING_THEM_BACK: u32 = 28766;
const LIONS_FOR_LAMBS: u32 = 28774;
const JOIN_THE_BATTLE: u32 = 28789;
const THEY_SENT_ASSASSINS: u32 = 28797;
/// Westfall: Westbrook Garrison Needs Help! (239) leads to Deputy Rainer's Riverpaw
/// Gnoll Bounty (11), 8 Painted Gnoll Armbands for a Militia Buckler or Urchin's Pants
/// (AzerothCore's choice rewards; TrinityCore sniffed none).
const WESTBROOK_GARRISON_NEEDS_HELP: u32 = 239;
const RIVERPAW_GNOLL_BOUNTY: u32 = 11;
const PAINTED_GNOLL_ARMBAND: u32 = 782;
/// Elwynn's exploration quests (TrinityCore QUEST_OBJECTIVE_AREATRIGGER, ObjectID -1):
/// Marshal Dughan sends you to scout the Fargodeep Mine (62), then the Jasperlode Mine (76).
const MARSHAL_DUGHAN: u32 = 240;
const THE_FARGODEEP_MINE: u32 = 62;
const THE_JASPERLODE_MINE: u32 = 76;
/// `AreaTrigger.db2` 88 (involved with 62): a 10 yd sphere inside the Fargodeep Mine.
const FARGODEEP_TRIGGER: (f32, f32, f32) = (-9843.54, 127.525, 5.3698);
/// `AreaTrigger.db2` 87 (involved with 76): a 30 yd sphere at the Jasperlode Mine.
const JASPERLODE_TRIGGER: (f32, f32, f32) = (-9077.34, -552.925, 60.3476);
const MILITIA_BUCKLER: u32 = 2249;
const URCHINS_PANTS: u32 = 2238;

fn item_templates() -> ItemTemplates {
    static TEMPLATES: OnceLock<ItemTemplates> = OnceLock::new();
    TEMPLATES
        .get_or_init(|| {
            let path = crate::test_data::world_db_path();
            load_item_templates(&path).expect("load item templates")
        })
        .clone()
}

#[test]
fn quest_log_story_rewards_and_pushability_survive_restart_without_a_giver_dialog() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("quest-relog.redb");
    let mut app = setup_app().expect("authentic quest data");
    app.insert_resource(GameDatabase::new(redb::Database::create(&path).unwrap()));
    persistence::create_tables(db(&app));
    let client = spawn_client(&mut app, 1);
    enter_world(&mut app, &client);
    let npc = spawn_npc(&mut app, MARSHAL_MCBRIDE, 0.0);
    request(
        &mut app,
        &client,
        QuestGiverQueryQuest {
            npc,
            quest_id: BEATING_THEM_BACK,
        },
    );
    let details = received::<QuestGiverQuestDetails>(&mut app, &client)
        .pop()
        .unwrap();
    assert!(details.description.contains("Marshal McBride"));
    accept(&mut app, &client, npc, BEATING_THEM_BACK);
    let accepted = last_update(&mut app, &client);
    assert_eq!(accepted.changed.len(), 1);
    let character_id = client.character_id;
    drop(app);

    let mut restarted = setup_app().expect("authentic quest data");
    restarted.insert_resource(GameDatabase::new(redb::Database::open(&path).unwrap()));
    let saved = persistence::load_character(db(&restarted), character_id).unwrap();
    let player = restarted
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: saved.name,
                race: saved.race,
                class: saved.class,
                appearance: saved.appearance,
            },
            Position {
                x: saved.x,
                y: saved.y,
                z: saved.z,
            },
            ClientId(character_id),
            Level(saved.level as u8),
            saved.equipment_appearance,
        ))
        .id();
    let connection = restarted
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            PlayerLink(player),
        ))
        .id();
    add_message_pair::<QuestLogSnapshot>(&mut restarted, connection);
    add_message_pair::<QuestGiverQuestDetails>(&mut restarted, connection);
    let reconnected = Client {
        character_id,
        player,
        connection,
    };
    let snapshot = enter_world(&mut restarted, &reconnected);
    assert_eq!(snapshot.entries.len(), 1);
    assert!(received::<QuestGiverQuestDetails>(&mut restarted, &reconnected).is_empty());
    let wire = serde_json::to_value(&snapshot.entries[0]).unwrap();
    assert_eq!(
        wire["description"].as_str(),
        Some(details.description.as_str())
    );
    assert_eq!(
        wire["rewards"],
        serde_json::to_value(&details.rewards).unwrap()
    );
    assert_eq!(wire["pushable"], serde_json::json!(false));
    assert_eq!(
        serde_json::to_value(&accepted.changed[0]).unwrap()["description"],
        wire["description"]
    );
}

#[test]
fn quest_share_accept_replay_and_party_change_use_real_message_path() {
    use shared::protocol::{
        PushQuestToParty, QuestPushReason, QuestPushResponse, QuestPushResult, QuestShareOffer,
    };
    let mut app = setup_app().expect("authentic quest data");
    let a = spawn_named_client(&mut app, 10, "ShareA");
    let b = spawn_named_client(&mut app, 10, "ShareB");
    enter_world(&mut app, &a);
    enter_world(&mut app, &b);
    let online: crate::group::OnlineView = [(a.character_id, "ShareA"), (b.character_id, "ShareB")]
        .into_iter()
        .map(|(id, name)| {
            (
                id,
                crate::group::OnlineMember {
                    entity: 0,
                    name: name.into(),
                    class: 1,
                    level: 10,
                    portrait: Default::default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: crate::maps::RaidDifficulty::default(),
                },
            )
        })
        .collect();
    let mut groups = crate::group::GroupRegistry::default();
    groups.invite(a.character_id, "ShareB", &online);
    groups.respond_invite(b.character_id, true);
    app.insert_resource(groups);
    let giver = spawn_npc(&mut app, MARSHAL_DUGHAN, 0.0);
    accept(&mut app, &a, giver, THE_FARGODEEP_MINE);
    assert!(received::<QuestFailed>(&mut app, &a).is_empty());
    request(
        &mut app,
        &a,
        PushQuestToParty {
            quest_id: THE_FARGODEEP_MINE,
        },
    );
    let offers = received::<QuestShareOffer>(&mut app, &b);
    assert_eq!(offers.len(), 1);
    assert!(
        load_quest_log(db(&app), b.character_id)
            .active(THE_FARGODEEP_MINE)
            .is_none()
    );
    let answer = QuestPushResponse {
        offer_id: offers[0].offer_id,
        accepted: true,
    };
    request(&mut app, &b, answer.clone());
    assert!(
        load_quest_log(db(&app), b.character_id)
            .active(THE_FARGODEEP_MINE)
            .is_some()
    );
    assert!(
        received::<QuestPushResult>(&mut app, &a)
            .iter()
            .any(|r| r.reason == QuestPushReason::Accepted)
    );
    request(&mut app, &b, answer);
    assert_eq!(load_quest_log(db(&app), b.character_id).active.len(), 1);
    request(
        &mut app,
        &a,
        PushQuestToParty {
            quest_id: THE_FARGODEEP_MINE,
        },
    );
    assert_eq!(
        received::<QuestPushResult>(&mut app, &a)[0].reason,
        QuestPushReason::OnQuest
    );
    let rejection = received::<QuestPushResult>(&mut app, &b);
    assert_eq!(rejection[0].reason, QuestPushReason::OnQuest);
    assert!(rejection[0].recipient);
    assert_eq!(rejection[0].player_name, "ShareA");
    assert_eq!(rejection[0].quest_title, "The Fargodeep Mine");
    request(
        &mut app,
        &b,
        AbandonQuest {
            quest_id: THE_FARGODEEP_MINE,
        },
    );
    request(
        &mut app,
        &a,
        PushQuestToParty {
            quest_id: THE_FARGODEEP_MINE,
        },
    );
    let offer = received::<QuestShareOffer>(&mut app, &b).remove(0);
    app.world_mut()
        .resource_mut::<crate::group::GroupRegistry>()
        .leave(b.character_id);
    request(
        &mut app,
        &b,
        QuestPushResponse {
            offer_id: offer.offer_id,
            accepted: true,
        },
    );
    assert!(
        load_quest_log(db(&app), b.character_id)
            .active(THE_FARGODEEP_MINE)
            .is_none()
    );
    assert!(
        received::<QuestPushResult>(&mut app, &a)
            .iter()
            .any(|result| result.reason == QuestPushReason::NotAllowed)
    );
}

#[test]
fn quest_timed_expiry_persists_and_cannot_reward_after_relog() {
    let mut app = setup_app().expect("authentic quest data");
    app.insert_resource(crate::maps::difficulty::ServerClock {
        fixed_unix: Some(1000),
        ..Default::default()
    });
    let client = spawn_client(&mut app, 30);
    // Authored quest 212 TimeAllowed=1800. Start-time persistence, not fabricated content.
    let mut log = load_quest_log(db(&app), client.character_id);
    log.active
        .push(crate::persistence::quest_store::ActiveQuestData {
            quest_id: 212,
            counts: vec![0],
            expires_at: Some(2800),
            failed: false,
        });
    log.watched.push(212);
    save_quest_log(db(&app), client.character_id, &log);
    let snapshot = enter_world(&mut app, &client);
    assert_eq!(snapshot.entries[0].expires_at, Some(2800));
    app.world_mut()
        .resource_mut::<crate::maps::difficulty::ServerClock>()
        .fixed_unix = Some(2799);
    app.update();
    assert!(
        !load_quest_log(db(&app), client.character_id)
            .active(212)
            .unwrap()
            .failed
    );
    app.world_mut()
        .resource_mut::<crate::maps::difficulty::ServerClock>()
        .fixed_unix = Some(2800);
    app.update();
    let saved = load_quest_log(db(&app), client.character_id);
    assert!(saved.active(212).unwrap().failed);
    assert!(!crate::quest_rules::is_complete_in_log(
        app.world().resource::<QuestDb>(),
        &saved,
        212,
        |_| 999
    ));
    app.world_mut()
        .entity_mut(client.connection)
        .remove::<InWorld>();
    app.update();
    assert!(enter_world(&mut app, &client).entries[0].failed);
}

fn setup_app() -> Option<App> {
    setup_app_with(|_| {})
}

#[test]
fn achievements_criteria_daily_quest_reward_counts_only_committed_authentic_daily() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let mut app = setup_app_with(|app| {
        app.register_message::<AchievementStateUpdate>();
        app.register_message::<AchievementDelta>();
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
        app.add_plugins(AchievementPlugin);
    })
    .expect("authentic quest data");
    let client = spawn_client(&mut app, 10);
    app.world_mut()
        .entity_mut(client.player)
        .insert(PlayerAchievementState::default());
    add_message_pair::<AchievementStateUpdate>(&mut app, client.connection);
    add_message_pair::<AchievementDelta>(&mut app, client.connection);
    enter_world(&mut app, &client);
    let giver = spawn_npc(&mut app, 37675, 0.0);
    accept(&mut app, &client, giver, 24629);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    turn_in(&mut app, &client, giver, 24629, None);
    assert!(!received::<QuestFailed>(&mut app, &client).is_empty());
    assert!(
        !persistence::load_achievements(db(&app), client.character_id)
            .criteria_progress
            .contains_key(&4984)
    );
    // Authored objective37990 receives native creature credit; not a perfume-use claim.
    kill(&mut app, &client, 37990, 10);
    turn_in(&mut app, &client, giver, 24629, None);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(24629));
    assert_eq!(
        persistence::load_achievements(db(&app), client.character_id)
            .criteria_progress
            .get(&4984),
        Some(&1)
    );
    turn_in(&mut app, &client, giver, 24629, None);
    assert!(!received::<QuestFailed>(&mut app, &client).is_empty());
    app.update();
    assert_eq!(
        persistence::load_achievements(db(&app), client.character_id)
            .criteria_progress
            .get(&4984),
        Some(&1)
    );
}

#[test]
fn achievements_criteria_quest_money_zone_count_and_abandon_credit_only_committed_requests() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    use shared::protocol::AchievementStateUpdate;
    let mut app = setup_app_with(|app| {
        app.register_message::<AchievementStateUpdate>();
        app.register_message::<AchievementDelta>();
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
        app.add_plugins(AchievementPlugin);
    })
    .expect("authentic quest data");
    let client = spawn_client(&mut app, 60);
    app.world_mut()
        .entity_mut(client.player)
        .insert(PlayerAchievementState::default());
    add_message_pair::<AchievementStateUpdate>(&mut app, client.connection);
    add_message_pair::<AchievementDelta>(&mut app, client.connection);
    enter_world(&mut app, &client);
    let progress = |app: &App, criterion: u32| {
        persistence::load_achievements(db(app), client.character_id)
            .criteria_progress
            .get(&criterion)
            .copied()
    };
    // Real Hellfire quest9355: giver/ender16837, QuestSortID3483, kill16857 x15; its
    // authored prerequisite10142 (also Hellfire) is seeded as rewarded history.
    mark_rewarded(&app, &client, 10142);
    let giver = spawn_npc(&mut app, 16837, 0.0);
    accept(&mut app, &client, giver, 9355);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    // Statistic94 criterion5289 counts a committed log removal, not a refused one.
    request(&mut app, &client, AbandonQuest { quest_id: 9355 });
    assert_eq!(progress(&app, 5289), Some(1));
    request(&mut app, &client, AbandonQuest { quest_id: 9355 });
    assert!(!received::<QuestFailed>(&mut app, &client).is_empty());
    assert_eq!(progress(&app, 5289), Some(1));
    accept(&mut app, &client, giver, 9355);
    turn_in(&mut app, &client, giver, 9355, None);
    assert!(!received::<QuestFailed>(&mut app, &client).is_empty());
    for criterion in [4092, 3912, 3631] {
        assert_eq!(
            progress(&app, criterion),
            None,
            "refused turn-in credited {criterion}"
        );
    }
    kill(&mut app, &client, 16857, 15);
    let before = persistence::load_inventory(db(&app), client.character_id).gold;
    turn_in(&mut app, &client, giver, 9355, Some(0));
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    let paid = persistence::load_inventory(db(&app), client.character_id).gold - before;
    assert!(paid > 0);
    // Gold from quest rewards4092, Hellfire quests3912 and Quests completed3631.
    assert_eq!(progress(&app, 4092), Some(paid));
    assert_eq!(progress(&app, 3912), Some(1));
    assert_eq!(progress(&app, 3631), Some(2));
    let saved = persistence::load_achievements(db(&app), client.character_id);
    app.world_mut()
        .entity_mut(client.connection)
        .remove::<InWorld>();
    app.world_mut()
        .entity_mut(client.player)
        .insert(saved.clone());
    enter_world(&mut app, &client);
    let reloaded = app
        .world()
        .get::<PlayerAchievementState>(client.player)
        .unwrap();
    // TC login case: highest of counted rewarded Hellfire quests (10142, 9355); money
    // and abandons are never replayed.
    assert_eq!(reloaded.criteria_progress.get(&3912), Some(&2));
    for criterion in [4092, 3631, 5289] {
        assert_eq!(
            reloaded.criteria_progress.get(&criterion),
            saved.criteria_progress.get(&criterion),
            "login changed {criterion}"
        );
    }
}

#[test]
fn achievements_real_quest_reward_grants_only_after_turn_in_and_survives_relog() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    use shared::protocol::AchievementStateUpdate;
    let mut app = setup_app_with(|app| {
        app.register_message::<AchievementStateUpdate>();
        app.register_message::<AchievementDelta>();
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
        app.add_plugins(AchievementPlugin);
    })
    .expect("authentic quest data required");
    let client = spawn_client(&mut app, 80);
    app.world_mut()
        .get_mut::<Player>(client.player)
        .unwrap()
        .class = 9;
    app.world_mut()
        .entity_mut(client.player)
        .insert(PlayerAchievementState::default());
    add_message_pair::<AchievementStateUpdate>(&mut app, client.connection);
    add_message_pair::<AchievementDelta>(&mut app, client.connection);
    enter_world(&mut app, &client);
    received::<AchievementStateUpdate>(&mut app, &client);
    // Authored Warlock prerequisite and separate giver/ender for Dreadsteed of Xoroth.
    mark_rewarded(&app, &client, 7629);
    let giver = spawn_npc(&mut app, 14436, 0.0);
    let ender = spawn_npc(&mut app, 14504, 0.0);
    accept(&mut app, &client, giver, 7631);
    let errors = received::<QuestFailed>(&mut app, &client);
    assert!(errors.is_empty(), "quest acceptance failed: {errors:?}");
    // Accepted/completed objectives are not a reward; a wrong ender is rejected.
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(client.player)
            .unwrap()
            .earned_ids
            .contains(&2357)
    );
    turn_in(&mut app, &client, giver, 7631, None);
    assert!(!received::<QuestFailed>(&mut app, &client).is_empty());
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(client.player)
            .unwrap()
            .earned_ids
            .contains(&2357)
    );
    turn_in(&mut app, &client, ender, 7631, None);
    let errors = received::<QuestFailed>(&mut app, &client);
    assert!(errors.is_empty(), "quest reward failed: {errors:?}");
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(7631));
    let updates = received::<AchievementStateUpdate>(&mut app, &client);
    assert_eq!(
        updates
            .iter()
            .filter(|update| update
                .completed
                .as_ref()
                .is_some_and(|toast| toast.achievement_id == 2357))
            .count(),
        1
    );
    let saved = persistence::load_achievements(db(&app), client.character_id);
    assert!(saved.earned_ids.contains(&2357));
    assert_eq!(saved.criteria_progress[&8905], 1);
    assert!(
        !saved.criteria_progress.contains_key(&4984),
        "ordinary reward must not count as daily"
    );
    assert!(saved.earned_at.contains_key(&2357));
    app.world_mut()
        .entity_mut(client.connection)
        .remove::<InWorld>();
    app.world_mut()
        .entity_mut(client.player)
        .insert(saved.clone());
    enter_world(&mut app, &client);
    let mut reloaded = app
        .world()
        .get::<PlayerAchievementState>(client.player)
        .unwrap()
        .clone();
    assert!(
        reloaded.account.take().is_some(),
        "login reconstructs account presentation"
    );
    // Type74 Login (criterion23250) is the one counter each world entry adds to.
    let mut saved = saved;
    let logins = saved.criteria_progress.remove(&23250).unwrap();
    assert_eq!(reloaded.criteria_progress.remove(&23250), Some(logins + 1));
    assert_eq!(reloaded, saved);
    assert!(
        received::<AchievementStateUpdate>(&mut app, &client)
            .iter()
            .all(|update| update.completed.is_none())
    );
}

#[test]
fn achievements_criteria_currency_counts_committed_gain_not_requested_amount_or_balance() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    use crate::currency::{CurrencyPlugin, PlayerCurrencies};
    use shared::currency::{CurrencyWallet, currency_by_id};
    use shared::protocol::{CurrencyStateUpdate, EarnCurrency, SpendCurrency};

    let mut app = setup_app_with(|app| {
        for_currency_messages(app);
        app.add_plugins((CurrencyPlugin, AchievementPlugin));
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
    })
    .expect("real quest database");
    let client = spawn_named_client(&mut app, 1, "Currproof");
    add_message_pair::<EarnCurrency>(&mut app, client.connection);
    add_message_pair::<SpendCurrency>(&mut app, client.connection);
    add_message_pair::<CurrencyStateUpdate>(&mut app, client.connection);
    add_message_pair::<AchievementStateUpdate>(&mut app, client.connection);
    add_message_pair::<AchievementDelta>(&mut app, client.connection);
    add_message_pair::<QueryAchievementCatalog>(&mut app, client.connection);
    add_message_pair::<AchievementCatalogPage>(&mut app, client.connection);
    let mut wallet = CurrencyWallet::default();
    wallet.add(4, 990, currency_by_id(4).unwrap()).unwrap();
    app.world_mut()
        .entity_mut(client.player)
        .insert((PlayerCurrencies(wallet), PlayerAchievementState::default()));
    enter_world(&mut app, &client);
    assert!(
        !app.world()
            .get::<PlayerAchievementState>(client.player)
            .unwrap()
            .criteria_progress
            .contains_key(&41095)
    );
    // Valor's authored weekly cap is1000. Only10 of requested100 are granted.
    request(
        &mut app,
        &client,
        EarnCurrency {
            currency_id: 4,
            amount: 100,
        },
    );
    assert_eq!(
        app.world()
            .get::<PlayerCurrencies>(client.player)
            .unwrap()
            .0
            .get(4),
        1000
    );
    // The committed credit crosses the currency system's queued-message boundary.
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(client.player)
            .unwrap()
            .criteria_progress
            .get(&41095),
        Some(&10)
    );
    request(
        &mut app,
        &client,
        SpendCurrency {
            currency_id: 4,
            amount: 25,
        },
    );
    request(
        &mut app,
        &client,
        EarnCurrency {
            currency_id: 4,
            amount: 100,
        },
    );
    assert_eq!(
        app.world()
            .get::<PlayerAchievementState>(client.player)
            .unwrap()
            .criteria_progress
            .get(&41095),
        Some(&10)
    );
    // Failed grants never become achievement progress; request-only PvP gains remain forbidden.
    request(
        &mut app,
        &client,
        EarnCurrency {
            currency_id: 1792,
            amount: 100,
        },
    );
    assert_eq!(
        app.world()
            .get::<PlayerCurrencies>(client.player)
            .unwrap()
            .0
            .get(1792),
        0
    );
    let saved = persistence::load_achievements(
        &app.world().resource::<GameDatabase>().db,
        client.character_id,
    );
    assert_eq!(saved.criteria_progress.get(&41095), Some(&10));
}

fn for_currency_messages(app: &mut App) {
    register::<shared::protocol::EarnCurrency>(app);
    register::<shared::protocol::SpendCurrency>(app);
    register::<shared::protocol::CurrencyStateUpdate>(app);
    register::<AchievementStateUpdate>(app);
    register::<AchievementDelta>(app);
    register::<QueryAchievementCatalog>(app);
    register::<AchievementCatalogPage>(app);
}

/// `setup_app` plus the plugins `add_plugins` adds before the app is finished.
fn setup_app_with(add_plugins: impl FnOnce(&mut App)) -> Option<App> {
    let quests = test_quest_db()?.clone();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    register_messages(&mut app);
    crate::appearance_collection::tests::install_database(&mut app);
    app.init_resource::<GoldUpdateQueue>();
    app.insert_resource(item_templates());
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("open quest item-use toy catalog");
    app.insert_resource(crate::toy_catalog::load(&conn).expect("load quest item-use toy catalog"));
    app.insert_resource(quests);
    app.add_plugins(crate::inventory::InventoryPlugin);
    app.add_plugins(QuestPlugin);
    app.add_plugins(crate::experience::ExperiencePlugin);
    add_plugins(&mut app);
    app.finish();
    app.cleanup();
    Some(app)
}

macro_rules! for_quest_messages {
    ($apply:ident, $($arg:expr),*) => {
        $apply::<QuestLogSnapshot>($($arg),*);
        $apply::<shared::protocol::WorldQuestAvailability>($($arg),*);
        $apply::<QuestLogUpdate>($($arg),*);
        $apply::<QuestGiverStatusQuery>($($arg),*);
        $apply::<QuestGiverStatusMultiple>($($arg),*);
        $apply::<QuestGiverHello>($($arg),*);
        $apply::<QuestGiverQuestList>($($arg),*);
        $apply::<QuestGiverQueryQuest>($($arg),*);
        $apply::<QuestGiverQuestDetails>($($arg),*);
        $apply::<QuestGiverAcceptQuest>($($arg),*);
        $apply::<QuestGiverCompleteQuest>($($arg),*);
        $apply::<QuestGiverRequestItems>($($arg),*);
        $apply::<QuestGiverOfferReward>($($arg),*);
        $apply::<QuestGiverChooseReward>($($arg),*);
        $apply::<QuestGiverQuestComplete>($($arg),*);
        $apply::<AbandonQuest>($($arg),*);
        $apply::<SetQuestWatched>($($arg),*);
        $apply::<QuestFailed>($($arg),*);
        $apply::<shared::protocol::PushQuestToParty>($($arg),*);
        $apply::<shared::protocol::QuestPushResponse>($($arg),*);
        $apply::<shared::protocol::QuestShareOffer>($($arg),*);
        $apply::<shared::protocol::QuestPushResult>($($arg),*);
        $apply::<InventorySnapshot>($($arg),*);
        $apply::<EquipmentSnapshot>($($arg),*);
        $apply::<InventoryDelta>($($arg),*);
        $apply::<InventoryError>($($arg),*);
        $apply::<SwapItem>($($arg),*);
        $apply::<SplitItem>($($arg),*);
        $apply::<DestroyItem>($($arg),*);
        $apply::<UseItem>($($arg),*);
        $apply::<EquipItem>($($arg),*);
        $apply::<SortBags>($($arg),*);
        $apply::<PlayerXpUpdate>($($arg),*);
        $apply::<LogXpGain>($($arg),*);
    };
}

#[test]
fn world_quest_availability_preserves_both_authored_floor_areas() {
    let mut app = setup_app().expect("authentic quest fixture");
    app.insert_resource(crate::maps::difficulty::ServerClock {
        fixed_unix: Some(1000),
        ..Default::default()
    });
    app.insert_resource(crate::world_quests::Rotation {
        epoch: 1000,
        active_per_zone: 1000,
        ..Default::default()
    });
    {
        let mut quests = app.world_mut().resource_mut::<QuestDb>();
        assert!(
            quests.quests.contains_key(&82521),
            "real Pawns of Dark Masters template"
        );
        quests.world_quests.retain(|quest| quest.quest_id == 82521);
        assert_eq!(
            quests.world_quests.len(),
            1,
            "actual admitted world quest82521"
        );
        assert_eq!(quests.world_quest_areas[&82521].len(), 2);
    }
    let client = spawn_client(&mut app, 80);
    enter_world(&mut app, &client);
    let availability = received::<shared::protocol::WorldQuestAvailability>(&mut app, &client)
        .pop()
        .expect("received authoritative task availability");
    assert_eq!(
        availability.quests.len(),
        1,
        "one logical task, not duplicate quest entries"
    );
    assert_eq!(availability.quests[0].quest_id, 82521);
    let represented_maps: Vec<_> = availability.quests[0]
        .pois
        .iter()
        .map(|poi| poi.ui_map_id)
        .collect();
    assert_eq!(
        represented_maps,
        vec![2256, 2255],
        "every authored floor area must be conveyed"
    );
    assert!(
        availability.quests[0]
            .pois
            .iter()
            .all(|poi| poi.map_id == 2601)
    );
    assert_eq!(
        availability.quests[0]
            .pois
            .iter()
            .map(|poi| (poi.x, poi.y, poi.z))
            .collect::<Vec<_>>(),
        vec![(-2268, -1960, -1591), (-2275, -1992, -1607)],
    );
    assert_eq!(availability.quests[0].expires_at, 87400);
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(82521)
            .is_none()
    );
}

#[test]
fn world_quest_availability_multiple_zones_cross_policy_windows_and_reconnect() {
    let mut app = setup_app().expect("authentic quest fixture");
    app.insert_resource(crate::maps::difficulty::ServerClock {
        fixed_unix: Some(999),
        ..Default::default()
    });
    app.insert_resource(crate::world_quests::Rotation {
        epoch: 1000,
        active_per_zone: 1,
        gap_seconds: 86400,
        ..Default::default()
    });
    {
        let mut quests = app.world_mut().resource_mut::<QuestDb>();
        // Two actual tasks per zone; retain their authentic areas, objectives and policy duration.
        quests
            .world_quests
            .retain(|quest| [82521, 82524, 86941, 86948].contains(&quest.quest_id));
        assert_eq!(quests.world_quests.len(), 4);
        assert!(
            quests
                .world_quests
                .iter()
                .all(|quest| quest.duration == 86400)
        );
    }
    let client = spawn_client(&mut app, 80);
    enter_world(&mut app, &client);
    assert!(
        received::<shared::protocol::WorldQuestAvailability>(&mut app, &client)
            .pop()
            .unwrap()
            .quests
            .is_empty()
    );
    for (now, ids, deadline) in [
        (1000, vec![82521, 86941], Some(87400)),
        (87400, vec![], None),
        (173799, vec![], None),
        (173800, vec![82524, 86948], Some(260200)),
    ] {
        app.world_mut()
            .resource_mut::<crate::maps::difficulty::ServerClock>()
            .fixed_unix = Some(now);
        // HostClient transfers in Last; the next frame clears unread messages.
        // This clock mutation is already local, unlike a queued client request.
        app.update();
        let snapshot = received::<shared::protocol::WorldQuestAvailability>(&mut app, &client)
            .pop()
            .unwrap_or_else(|| panic!("availability not received at realm time {now}"));
        assert_eq!(snapshot.server_time, now);
        assert_eq!(
            snapshot
                .quests
                .iter()
                .map(|quest| quest.quest_id)
                .collect::<Vec<_>>(),
            ids
        );
        assert!(
            snapshot
                .quests
                .iter()
                .all(|quest| Some(quest.expires_at) == deadline)
        );
        if now == 1000 {
            assert_eq!(
                snapshot.quests[0]
                    .pois
                    .iter()
                    .map(|poi| poi.ui_map_id)
                    .collect::<Vec<_>>(),
                vec![2256, 2255]
            );
        }
    }
    let current = app
        .world()
        .resource::<QuestDb>()
        .world_quest_windows
        .clone();
    app.world_mut().despawn(client.connection);
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            PlayerLink(client.player),
        ))
        .id();
    for_quest_messages!(add_message_pair, &mut app, connection);
    let reconnected = Client {
        character_id: client.character_id,
        player: client.player,
        connection,
    };
    enter_world(&mut app, &reconnected);
    let snapshot = received::<shared::protocol::WorldQuestAvailability>(&mut app, &reconnected)
        .pop()
        .unwrap();
    assert_eq!(snapshot.server_time, 173800);
    assert_eq!(
        snapshot
            .quests
            .iter()
            .map(|quest| quest.quest_id)
            .collect::<Vec<_>>(),
        vec![82524, 86948]
    );
    assert_eq!(
        app.world().resource::<QuestDb>().world_quest_windows,
        current
    );
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active
            .is_empty()
    );
}

#[test]
fn world_quest_area_entry_progress_reward_window_replay_and_expiry() {
    use crate::world_quests::{Area, Definition, Rotation};
    let mut app = setup_app().expect("authentic quest fixture");
    app.insert_resource(crate::maps::difficulty::ServerClock {
        fixed_unix: Some(1000),
        ..Default::default()
    });
    app.insert_resource(Rotation {
        epoch: 1000,
        gap_seconds: 0,
        ..Default::default()
    });
    {
        let mut quests = app.world_mut().resource_mut::<QuestDb>();
        let quest = quests.quests.get_mut(&BEATING_THEM_BACK).unwrap();
        quest.quest_info_modifiers = 0x100;
        quest.reward_money = 125;
        quest.money_scaling = None;
        quest.captured_reward_xp = Some(0);
        quest.reward_xp_difficulty = 5;
        quest.reward_items = vec![(117, 2)];
        quests.world_quest_xp.insert((30, 5), 1234);
        quests.world_quest_rewards.insert(
            BEATING_THEM_BACK,
            crate::world_quest_rewards::Rewards {
                xp_multiplier: 1.,
                currencies: vec![shared::protocol::WorldQuestNumericReward {
                    id: shared::currency::HONOR_CURRENCY_ID,
                    amount: 7,
                }],
                reputation: vec![shared::protocol::WorldQuestNumericReward { id: 72, amount: 75 }],
                ..Default::default()
            },
        );
        quests.world_quests = vec![Definition {
            quest_id: BEATING_THEM_BACK,
            zone_id: 37,
            duration: 100,
        }];
        quests.world_quest_areas.insert(
            BEATING_THEM_BACK,
            vec![Area {
                map_id: 0,
                ui_map_id: 37,
                vertices: vec![
                    (10., 10., 100.),
                    (20., 10., 100.),
                    (20., 20., 100.),
                    (10., 20., 100.),
                ],
            }],
        );
    }
    let client = spawn_client(&mut app, 30);
    enter_world(&mut app, &client);
    let availability = received::<shared::protocol::WorldQuestAvailability>(&mut app, &client)
        .pop()
        .unwrap();
    assert_eq!(availability.quests.len(), 1);
    assert_eq!(availability.quests[0].expires_at, 1100);
    assert_eq!(
        (
            availability.quests[0].pois[0].x,
            availability.quests[0].pois[0].y,
            availability.quests[0].pois[0].z
        ),
        (15, 15, 100)
    );
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(BEATING_THEM_BACK)
            .is_none()
    );
    app.world_mut().entity_mut(client.player).insert(Position {
        x: 15.,
        y: 100.,
        z: -15.,
    });
    app.update();
    let log = load_quest_log(db(&app), client.character_id);
    assert_eq!(
        log.active(BEATING_THEM_BACK).unwrap().expires_at,
        Some(1100)
    );
    assert!(log.watched.contains(&BEATING_THEM_BACK));
    app.world_mut()
        .resource_mut::<QuestDb>()
        .quests
        .get_mut(&THE_FARGODEEP_MINE)
        .unwrap()
        .time_allowed = 500;
    let giver = spawn_npc(&mut app, MARSHAL_DUGHAN, 15.);
    app.world_mut()
        .entity_mut(Entity::try_from_bits(giver).unwrap())
        .insert(Position {
            x: 15.,
            y: 100.,
            z: -15.,
        });
    accept(&mut app, &client, giver, THE_FARGODEEP_MINE);
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(THE_FARGODEEP_MINE)
            .is_some(),
        "task availability deadlines are not ordinary quest timers"
    );
    app.world_mut().entity_mut(client.player).insert(origin());
    app.update();
    let log = load_quest_log(db(&app), client.character_id);
    assert!(log.active(BEATING_THEM_BACK).is_some());
    assert!(!log.watched.contains(&BEATING_THEM_BACK));
    app.world_mut().entity_mut(client.player).insert(Position {
        x: 15.,
        y: 100.,
        z: -15.,
    });
    app.update();
    let gold = persistence::load_inventory(db(&app), client.character_id).gold;
    let food_before = crate::quest_view::bag_count(
        db(&app),
        &persistence::load_inventory(db(&app), client.character_id),
        117,
    );
    let reputation_before =
        persistence::load_reputation(db(&app), client.character_id, Default::default).get_value(72);
    kill(&mut app, &client, BLACKROCK_WORG, 1);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(BEATING_THEM_BACK)
            .unwrap()
            .counts[0],
        1
    );
    app.world_mut().entity_mut(client.player).insert(origin());
    kill(&mut app, &client, BLACKROCK_WORG, 5);
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(BEATING_THEM_BACK)
            .is_none()
    );
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id).gold,
        gold + 125
    );
    assert_eq!(
        crate::quest_view::bag_count(
            db(&app),
            &persistence::load_inventory(db(&app), client.character_id),
            117
        ),
        food_before + 2
    );
    // Automatic settlement queues XP in PostUpdate; the XP system drains it next frame.
    app.update();
    let gains = received::<LogXpGain>(&mut app, &client);
    assert_eq!(
        gains
            .iter()
            .filter(|gain| gain.reason == XpGainReason::Quest)
            .map(|gain| gain.amount)
            .collect::<Vec<_>>(),
        vec![1234]
    );
    assert_eq!(
        persistence::load_currencies(db(&app), client.character_id)
            .get(shared::currency::HONOR_CURRENCY_ID),
        7
    );
    assert_eq!(
        persistence::load_reputation(db(&app), client.character_id, Default::default).get_value(72),
        reputation_before + 75
    );
    app.update();
    assert!(
        received::<LogXpGain>(&mut app, &client)
            .iter()
            .all(|gain| gain.reason != XpGainReason::Quest)
    );
    assert_eq!(
        crate::quest_view::bag_count(
            db(&app),
            &persistence::load_inventory(db(&app), client.character_id),
            117
        ),
        food_before + 2
    );
    assert_eq!(
        persistence::load_currencies(db(&app), client.character_id)
            .get(shared::currency::HONOR_CURRENCY_ID),
        7
    );
    assert_eq!(
        persistence::load_inventory(db(&app), client.character_id).gold,
        gold + 125
    );
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(BEATING_THEM_BACK)
            .is_none()
    );
    app.world_mut().entity_mut(client.player).insert(Position {
        x: 15.,
        y: 100.,
        z: -15.,
    });
    app.world_mut()
        .resource_mut::<crate::maps::difficulty::ServerClock>()
        .fixed_unix = Some(1100);
    app.update();
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(BEATING_THEM_BACK)
            .unwrap()
            .expires_at,
        Some(1200)
    );
    app.world_mut().entity_mut(client.player).insert(origin());
    app.world_mut()
        .resource_mut::<crate::maps::difficulty::ServerClock>()
        .fixed_unix = Some(1200);
    app.update();
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(BEATING_THEM_BACK)
            .is_none()
    );
}

#[test]
fn world_quest_player_kills_use_authoritative_deaths_personal_credit_and_faction_flags() {
    use crate::quest_data::{ObjectiveDef, ObjectiveKind, QuestTemplate};
    use crate::world_quest_objectives::ObjectiveRule;
    let mut app = setup_app_with(|app| {
        app.add_message::<crate::death::PlayerDeathTransition>();
    })
    .unwrap();
    let killer = spawn_named_client(&mut app, 30, "PvpQuest");
    let ally = spawn_named_client(&mut app, 30, "PvpAlly");
    let enemy = spawn_named_client(&mut app, 30, "PvpEnemy");
    app.world_mut()
        .get_mut::<Player>(enemy.player)
        .unwrap()
        .race = 2;
    for player in [ally.player, enemy.player] {
        app.world_mut()
            .entity_mut(player)
            .insert(shared::components::Health {
                current: 100.,
                max: 100.,
            });
    }
    let objectives = vec![
        ObjectiveDef {
            kind: ObjectiveKind::PlayerKills,
            object_id: 0,
            required: 2,
            text: "Players defeated".into(),
        },
        ObjectiveDef {
            kind: ObjectiveKind::PlayerKills,
            object_id: 0,
            required: 2,
            text: "Same faction".into(),
        },
        ObjectiveDef {
            kind: ObjectiveKind::Monster,
            object_id: 0,
            required: 2,
            text: "Never player credit".into(),
        },
    ];
    app.world_mut().resource_mut::<QuestDb>().quests.insert(
        900001,
        QuestTemplate {
            id: 900001,
            objectives,
            world_objective_rules: Some(vec![
                ObjectiveRule::default(),
                ObjectiveRule {
                    flags: 128,
                    flags2: 0,
                    weight: 0.,
                },
                ObjectiveRule::default(),
            ]),
            ..Default::default()
        },
    );
    let active = crate::persistence::quest_store::ActiveQuestData {
        quest_id: 900001,
        counts: vec![0; 3],
        ..Default::default()
    };
    for client in [&killer, &ally] {
        save_quest_log(
            db(&app),
            client.character_id,
            &QuestLogData {
                active: vec![active.clone()],
                ..Default::default()
            },
        );
    }
    crate::combat::kill_unit(app.world_mut(), killer.player, ally.player);
    assert_eq!(
        load_quest_log(db(&app), killer.character_id)
            .active(900001)
            .unwrap()
            .counts,
        vec![1, 1, 0]
    );
    assert_eq!(
        load_quest_log(db(&app), ally.character_id)
            .active(900001)
            .unwrap()
            .counts,
        vec![0, 0, 0]
    );
    crate::combat::kill_unit(app.world_mut(), killer.player, ally.player);
    assert_eq!(
        load_quest_log(db(&app), killer.character_id)
            .active(900001)
            .unwrap()
            .counts,
        vec![1, 1, 0]
    );
    crate::combat::kill_unit(app.world_mut(), killer.player, enemy.player);
    assert_eq!(
        load_quest_log(db(&app), killer.character_id)
            .active(900001)
            .unwrap()
            .counts,
        vec![2, 1, 0]
    );
}

#[path = "zephras_quest_tests.rs"]
mod zephras_tests;

fn register<M: Message + serde::Serialize + serde::de::DeserializeOwned>(app: &mut App) {
    app.register_message::<M>();
}

fn register_messages(app: &mut App) {
    for_quest_messages!(register, app);
}

fn add_message_pair<M: Message>(app: &mut App, connection: Entity) {
    app.world_mut().entity_mut(connection).insert((
        MessageSender::<M>::default(),
        MessageReceiver::<M>::default(),
    ));
}

struct Client {
    character_id: u64,
    player: Entity,
    connection: Entity,
}

fn origin() -> Position {
    Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }
}

fn spawn_client(app: &mut App, level: u8) -> Client {
    spawn_named_client(app, level, "Questor")
}

fn spawn_named_client(app: &mut App, level: u8, name: &str) -> Client {
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) =
        persistence::create_character(db, 1, name, 1, 1).expect("create character");
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            origin(),
            ClientId(character_id),
            Level(level),
            EquipmentAppearance::default(),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            PlayerLink(player),
        ))
        .id();
    for_quest_messages!(add_message_pair, app, connection);
    Client {
        character_id,
        player,
        connection,
    }
}

fn enter_world(app: &mut App, client: &Client) -> QuestLogSnapshot {
    app.world_mut()
        .entity_mut(client.connection)
        .insert(InWorld {
            character_id: client.character_id,
        });
    app.update();
    let mut snapshots = received::<QuestLogSnapshot>(app, client);
    assert_eq!(snapshots.len(), 1);
    snapshots.remove(0)
}

fn spawn_npc(app: &mut App, template_id: u32, x: f32) -> u64 {
    app.world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Npc {
                template_id,
                name: String::new(),
            },
            Position { x, y: 0.0, z: 0.0 },
        ))
        .id()
        .to_bits()
}

fn request<M: Message>(app: &mut App, client: &Client, message: M) {
    app.world_mut()
        .entity_mut(client.connection)
        .get_mut::<MessageSender<M>>()
        .expect("connection sender")
        .send::<QuestChannel>(message);
    app.update();
    app.update();
}

fn received<M: Message>(app: &mut App, client: &Client) -> Vec<M> {
    app.world_mut()
        .entity_mut(client.connection)
        .get_mut::<MessageReceiver<M>>()
        .expect("connection receiver")
        .receive()
        .collect()
}

fn db(app: &App) -> &Database {
    &app.world().resource::<GameDatabase>().db
}

fn statuses(app: &mut App, client: &Client, npcs: &[u64]) -> Vec<QuestGiverStatus> {
    request(
        app,
        client,
        QuestGiverStatusQuery {
            npcs: npcs.to_vec(),
        },
    );
    let mut replies = received::<QuestGiverStatusMultiple>(app, client);
    let reply = replies.pop().expect("status reply");
    reply.statuses.iter().map(|entry| entry.status).collect()
}

fn accept(app: &mut App, client: &Client, npc: u64, quest_id: u32) {
    request(app, client, QuestGiverAcceptQuest { npc, quest_id });
}

fn turn_in(app: &mut App, client: &Client, npc: u64, quest_id: u32, choice_index: Option<u8>) {
    request(
        app,
        client,
        QuestGiverChooseReward {
            npc,
            quest_id,
            choice_index,
        },
    );
}

fn last_update(app: &mut App, client: &Client) -> QuestLogUpdate {
    received::<QuestLogUpdate>(app, client)
        .pop()
        .expect("quest log update")
}

fn mark_rewarded(app: &App, client: &Client, quest_id: u32) {
    let mut log = load_quest_log(db(app), client.character_id);
    log.mark_rewarded(quest_id);
    save_quest_log(db(app), client.character_id, &log);
}

#[test]
fn world_quest_single_leaf_criteria_credit_is_persisted_capped_and_fresh_for_each_task() {
    let mut app = setup_app().unwrap();
    app.world_mut()
        .resource_mut::<crate::maps::difficulty::ServerClock>()
        .fixed_unix = Some(1000);
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_world_quest(quest_id INTEGER); INSERT INTO content_world_quest VALUES(82387);
        CREATE TABLE content_world_quest_objective(quest_id INTEGER,kind INTEGER,object_id INTEGER,amount INTEGER,flags INTEGER,flags2 INTEGER,weight REAL,description TEXT,ordinal INTEGER);
        INSERT INTO content_world_quest_objective VALUES(82387,14,165946,1,0,0,0,'Have 50 or more stacks of Unseeming Shift',0);
        CREATE TABLE tdb_creature_template(entry INTEGER,name TEXT); CREATE TABLE content_item(ID INTEGER,name TEXT);
        CREATE TABLE content_world_quest_criterion(root_id INTEGER,criterion_id INTEGER,kind INTEGER,asset INTEGER);
        INSERT INTO content_world_quest_criterion VALUES(165946,69041,0,227455);").unwrap();
    {
        let mut quests = app.world_mut().resource_mut::<QuestDb>();
        quests.quests.insert(
            82387,
            crate::quest_data::QuestTemplate {
                id: 82387,
                ..Default::default()
            },
        );
        crate::world_quest_objectives::load(&conn, &mut quests).unwrap();
    }
    let client = spawn_client(&mut app, 80);
    enter_world(&mut app, &client);
    killed_monster_credit(app.world_mut(), client.player, 227455);
    let fresh = QuestLogData {
        active: vec![crate::persistence::quest_store::ActiveQuestData {
            quest_id: 82387,
            counts: vec![0],
            expires_at: Some(2000),
            ..Default::default()
        }],
        ..Default::default()
    };
    save_quest_log(db(&app), client.character_id, &fresh);
    killed_monster_credit(app.world_mut(), client.player, 225891);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82387)
            .unwrap()
            .counts,
        vec![0]
    );
    killed_monster_credit(app.world_mut(), client.player, 227455);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82387)
            .unwrap()
            .counts,
        vec![1]
    );
    killed_monster_credit(app.world_mut(), client.player, 227455);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82387)
            .unwrap()
            .counts,
        vec![1]
    );
    save_quest_log(db(&app), client.character_id, &fresh);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82387)
            .unwrap()
            .counts,
        vec![0]
    );
    killed_monster_credit(app.world_mut(), client.player, 227455);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82387)
            .unwrap()
            .counts,
        vec![1]
    );
}

#[test]
fn world_quest_label_credit_uses_authentic_membership_only_on_real_creature_death() {
    let mut app = setup_app().unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content_world_quest(quest_id INTEGER); INSERT INTO content_world_quest VALUES(82414);
        CREATE TABLE content_world_quest_objective(quest_id INTEGER,kind INTEGER,object_id INTEGER,amount INTEGER,flags INTEGER,flags2 INTEGER,weight REAL,description TEXT,ordinal INTEGER);
        INSERT INTO content_world_quest_objective VALUES(82414,21,3520,5,0,0,0,'Insects slain',0);
        CREATE TABLE tdb_creature_template(entry INTEGER,name TEXT); CREATE TABLE content_item(ID INTEGER,name TEXT);
        CREATE TABLE content_world_quest_creature_label(label_id INTEGER,creature_entry INTEGER,difficulty_id INTEGER);
        INSERT INTO content_world_quest_creature_label VALUES(3520,222677,0);").unwrap();
    {
        let mut quests = app.world_mut().resource_mut::<QuestDb>();
        quests.quests.insert(
            82414,
            crate::quest_data::QuestTemplate {
                id: 82414,
                ..Default::default()
            },
        );
        crate::world_quest_objectives::load(&conn, &mut quests).unwrap();
    }
    let client = spawn_client(&mut app, 80);
    enter_world(&mut app, &client);
    save_quest_log(
        db(&app),
        client.character_id,
        &QuestLogData {
            active: vec![crate::persistence::quest_store::ActiveQuestData {
                quest_id: 82414,
                counts: vec![0],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    killed_monster_credit(app.world_mut(), client.player, 222677);
    kill(&mut app, &client, 3520, 1);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82414)
            .unwrap()
            .counts,
        vec![0]
    );
    kill(&mut app, &client, 222677, 1);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82414)
            .unwrap()
            .counts,
        vec![1]
    );
    app.world_mut().spawn((
        crate::maps::InMap::continent(0),
        Npc {
            template_id: 222677,
            name: String::new(),
        },
        origin(),
        crate::zones::CreatureDifficulty(1),
        crate::creature_tap::CreatureTap::tapped_by(client.character_id),
        DeathState { respawn_timer: 60. },
    ));
    app.update();
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82414)
            .unwrap()
            .counts,
        vec![1]
    );
    kill(&mut app, &client, 222677, 8);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(82414)
            .unwrap()
            .counts,
        vec![5]
    );
}

fn kill(app: &mut App, client: &Client, template_id: u32, count: usize) {
    for _ in 0..count {
        app.world_mut().spawn((
            crate::maps::InMap::continent(0),
            Npc {
                template_id,
                name: String::new(),
            },
            origin(),
            crate::creature_tap::CreatureTap::tapped_by(client.character_id),
            DeathState {
                respawn_timer: 60.0,
            },
        ));
    }
    // Credit is sent in the same frame's PostUpdate.
    app.update();
}

fn bag_items(app: &App, client: &Client) -> Vec<(u32, u32)> {
    persistence::load_inventory(db(app), client.character_id)
        .items
        .iter()
        .map(|item| (item.item_id, item.stack_count))
        .collect()
}

#[test]
fn join_the_battle_accepted_from_mcbride_and_turned_in_at_willem_is_persisted() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    let willem = spawn_npc(&mut app, DEPUTY_WILLEM, 3.0);
    mark_rewarded(&app, &client, BEATING_THEM_BACK);
    mark_rewarded(&app, &client, LIONS_FOR_LAMBS);
    assert!(enter_world(&mut app, &client).entries.is_empty());
    assert_eq!(
        received::<PlayerXpUpdate>(&mut app, &client),
        vec![PlayerXpUpdate {
            xp: 0,
            next_level_xp: 250,
            rested_xp: 0,
        }]
    );
    assert_eq!(
        statuses(&mut app, &client, &[mcbride, willem]),
        vec![
            QuestGiverStatus::Available(QuestMarkerClass::Normal),
            QuestGiverStatus::None
        ]
    );

    request(&mut app, &client, QuestGiverHello { npc: mcbride });
    let list = received::<QuestGiverQuestList>(&mut app, &client)
        .pop()
        .unwrap();
    assert!(
        list.quests
            .iter()
            .any(|quest| quest.quest_id == JOIN_THE_BATTLE
                && quest.state == QuestGiverQuestState::Available)
    );
    accept(&mut app, &client, mcbride, JOIN_THE_BATTLE);

    let update = last_update(&mut app, &client);
    assert_eq!(update.changed.len(), 1);
    let entry = &update.changed[0];
    assert_eq!(
        (entry.quest_id, entry.title.as_str()),
        (JOIN_THE_BATTLE, "Join the Battle!")
    );
    assert!(entry.completed && entry.watched && entry.objectives.is_empty());
    assert_eq!(update.watched_quest_ids, vec![JOIN_THE_BATTLE]);
    let markers = received::<QuestGiverStatusMultiple>(&mut app, &client)
        .pop()
        .unwrap();
    assert!(markers.statuses.contains(&QuestGiverStatusEntry {
        npc: willem,
        status: QuestGiverStatus::Reward(QuestMarkerClass::Normal),
    }));

    request(
        &mut app,
        &client,
        QuestGiverCompleteQuest {
            npc: willem,
            quest_id: JOIN_THE_BATTLE,
        },
    );
    assert_eq!(
        received::<QuestGiverOfferReward>(&mut app, &client).len(),
        1
    );
    turn_in(&mut app, &client, willem, JOIN_THE_BATTLE, None);

    let complete = received::<QuestGiverQuestComplete>(&mut app, &client);
    assert_eq!(complete[0].quest_id, JOIN_THE_BATTLE);
    // QuestXP level 1 (QuestLevel -1 scales to the player), RewardXPDifficulty 2.
    assert_eq!(complete[0].xp, 50);
    assert_eq!(
        received::<LogXpGain>(&mut app, &client),
        vec![LogXpGain {
            victim: None,
            original: 50,
            amount: 50,
            group_bonus: 1.0,
            reason: XpGainReason::Quest,
        }]
    );
    assert_eq!(
        received::<PlayerXpUpdate>(&mut app, &client),
        vec![PlayerXpUpdate {
            xp: 50,
            next_level_xp: 250,
            rested_xp: 0,
        }]
    );
    assert_eq!(
        last_update(&mut app, &client).removed,
        vec![JOIN_THE_BATTLE]
    );
    let next = received::<QuestGiverQuestDetails>(&mut app, &client);
    assert_eq!(next[0].quest_id, THEY_SENT_ASSASSINS);
    let saved = load_quest_log(db(&app), client.character_id);
    assert!(saved.active.is_empty() && saved.watched.is_empty());
    assert!(saved.is_rewarded(JOIN_THE_BATTLE));
    assert_eq!(
        crate::persistence_progression::load_xp(db(&app), client.character_id),
        50
    );
    assert_eq!(
        statuses(&mut app, &client, &[willem]),
        vec![QuestGiverStatus::Available(QuestMarkerClass::Normal)]
    );
}

#[test]
fn lions_for_lambs_is_blocked_until_beating_them_back_is_turned_in() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    enter_world(&mut app, &client);

    request(
        &mut app,
        &client,
        QuestGiverQueryQuest {
            npc: mcbride,
            quest_id: LIONS_FOR_LAMBS,
        },
    );
    let expected = vec![QuestFailed {
        quest_id: LIONS_FOR_LAMBS,
        reason: QuestFailedReason::DontHaveRequirement,
    }];
    assert_eq!(received::<QuestFailed>(&mut app, &client), expected);
    assert!(received::<QuestGiverQuestDetails>(&mut app, &client).is_empty());
    accept(&mut app, &client, mcbride, LIONS_FOR_LAMBS);
    assert_eq!(received::<QuestFailed>(&mut app, &client), expected);
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active
            .is_empty()
    );

    mark_rewarded(&app, &client, BEATING_THEM_BACK);
    accept(&mut app, &client, mcbride, LIONS_FOR_LAMBS);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(LIONS_FOR_LAMBS)
            .is_some()
    );
}

#[test]
fn blackrock_worg_kills_progress_beating_them_back_to_turn_in() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, mcbride, BEATING_THEM_BACK);
    received::<QuestLogUpdate>(&mut app, &client);
    let gold_before = persistence::load_inventory(db(&app), client.character_id).gold;

    kill(&mut app, &client, BLACKROCK_WORG, 3);
    let objective = &last_update(&mut app, &client).changed[0].objectives[0];
    assert_eq!(
        (
            objective.text.as_str(),
            objective.current,
            objective.required
        ),
        ("Blackrock Worg slain", 3, 6)
    );
    kill(&mut app, &client, BLACKROCK_WORG, 4);
    let entry = &last_update(&mut app, &client).changed[0];
    assert_eq!(entry.objectives[0].current, 6);
    assert!(entry.completed && entry.objectives[0].completed);
    assert!(!entry.pois.is_empty());

    turn_in(&mut app, &client, mcbride, BEATING_THEM_BACK, None);
    // Quest::MoneyValue: QuestMoneyReward at the level 1 player's quest level (ContentTuning
    // 1476: 1-10), RewardMoneyDifficulty 5 (55) × RewardMoneyMultiplier 1, not the 2000
    // MaxMoneyValue at level 10.
    assert_eq!(
        received::<QuestGiverQuestComplete>(&mut app, &client)[0].money,
        55
    );
    let inventory = persistence::load_inventory(db(&app), client.character_id);
    assert_eq!(inventory.gold, gold_before + 55);
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(BEATING_THEM_BACK));
}

fn grant_armbands(app: &mut App, client: &Client, count: u32) {
    let db = db(app);
    let mut inventory = persistence::load_inventory(db, client.character_id);
    inventory.items.push(InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id: PAINTED_GNOLL_ARMBAND,
        name: "Painted Gnoll Armband".into(),
        quality: 1,
        required_level: 0,
        stack_count: count,
        vendor_sell_price: 0,
        unique_auction: false,
    });
    persistence::save_inventory(db, client.character_id, &inventory);
    app.world_mut()
        .resource_mut::<GoldUpdateQueue>()
        .record_items(client.character_id);
    app.update();
}

#[test]
fn transmogacq_quest_rewards_collect_chosen_and_unchosen_appearances() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 2);
    let rainer = spawn_npc(&mut app, DEPUTY_RAINER, 2.0);
    mark_rewarded(&app, &client, WESTBROOK_GARRISON_NEEDS_HELP);
    enter_world(&mut app, &client);
    accept(&mut app, &client, rainer, RIVERPAW_GNOLL_BOUNTY);
    received::<QuestLogUpdate>(&mut app, &client);

    grant_armbands(&mut app, &client, 8);
    let entry = &last_update(&mut app, &client).changed[0];
    assert_eq!(entry.objectives[0].text, "Painted Gnoll Armband");
    assert_eq!((entry.objectives[0].current, entry.completed), (8, true));

    request(
        &mut app,
        &client,
        QuestGiverCompleteQuest {
            npc: rainer,
            quest_id: RIVERPAW_GNOLL_BOUNTY,
        },
    );
    let offer = received::<QuestGiverOfferReward>(&mut app, &client)
        .pop()
        .unwrap();
    let choices: Vec<u32> = offer
        .rewards
        .choice_items
        .iter()
        .map(|item| item.item_id)
        .collect();
    assert_eq!(choices, vec![MILITIA_BUCKLER, URCHINS_PANTS]);
    turn_in(&mut app, &client, rainer, RIVERPAW_GNOLL_BOUNTY, None);
    assert_eq!(
        received::<QuestFailed>(&mut app, &client)[0].reason,
        QuestFailedReason::InvalidRewardChoice
    );
    turn_in(&mut app, &client, rainer, RIVERPAW_GNOLL_BOUNTY, Some(1));

    let complete = received::<QuestGiverQuestComplete>(&mut app, &client)
        .pop()
        .unwrap();
    assert_eq!(
        complete.items,
        vec![QuestRewardItem {
            item_id: URCHINS_PANTS,
            name: "Urchin's Pants".into(),
            count: 1,
        }]
    );
    let items = bag_items(&app, &client);
    assert!(items.contains(&(URCHINS_PANTS, 1)));
    let appearances = crate::persistence::appearance_store::load_account_appearances(db(&app), 1);
    assert!(appearances.has(763), "chosen Urchin's Pants appearance");
    assert!(appearances.has(356), "unchosen Militia Buckler appearance");
    assert!(
        !items
            .iter()
            .any(|&(id, _)| id == MILITIA_BUCKLER || id == PAINTED_GNOLL_ARMBAND)
    );
    crate::appearance_collection::tests::assert_restart(app, 1, &[(763, true), (356, true)]);
}

#[test]
fn transmogacq_quest_does_not_collect_unchosen_cosmetic_rewards() {
    let mut app = setup_app().unwrap();
    let client = spawn_client(&mut app, 10);
    // Can't Get Enough Turkey (14048), Ellen Moore (34708), 20 Slow-Roasted Turkey (44838).
    let giver = spawn_npc(&mut app, 34708, 2.0);
    let mut inventory = persistence::load_inventory(db(&app), client.character_id);
    inventory.items.push(InventoryItemData {
        item_guid: persistence::next_id(db(&app), "item_instance"),
        item_id: 44838,
        name: "Slow-Roasted Turkey".into(),
        quality: 1,
        required_level: 0,
        stack_count: 20,
        vendor_sell_price: 0,
        unique_auction: false,
    });
    persistence::save_inventory(db(&app), client.character_id, &inventory);
    enter_world(&mut app, &client);
    accept(&mut app, &client, giver, 14048);
    let errors = received::<QuestFailed>(&mut app, &client);
    assert!(errors.is_empty(), "{errors:?}");
    turn_in(&mut app, &client, giver, 14048, Some(3));
    let errors = received::<QuestFailed>(&mut app, &client);
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        bag_items(&app, &client).contains(&(46824, 1)),
        "selected Pilgrim's Robe"
    );
    let appearances = crate::persistence::appearance_store::load_account_appearances(db(&app), 1);
    assert!(
        appearances.has(11705),
        "selected BoP cosmetic is learned by ownership"
    );
    assert!(
        !appearances.has(11702),
        "unchosen Pilgrim's Attire is cosmetic, excluded from all-option grants"
    );
    crate::appearance_collection::tests::assert_restart(app, 1, &[(11705, true), (11702, false)]);
}

#[test]
fn abandoning_beating_them_back_removes_it_and_mcbride_offers_it_again() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, mcbride, BEATING_THEM_BACK);
    received::<QuestLogUpdate>(&mut app, &client);

    request(
        &mut app,
        &client,
        AbandonQuest {
            quest_id: BEATING_THEM_BACK,
        },
    );

    let update = last_update(&mut app, &client);
    assert_eq!(
        (update.removed, update.watched_quest_ids),
        (vec![BEATING_THEM_BACK], vec![])
    );
    let saved = load_quest_log(db(&app), client.character_id);
    assert!(saved.active.is_empty() && saved.rewarded.is_empty());
    assert_eq!(
        statuses(&mut app, &client, &[mcbride]),
        vec![QuestGiverStatus::Available(QuestMarkerClass::Normal)]
    );
}

#[test]
fn untracking_a_quest_persists_and_resends_the_watch_list() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, mcbride, BEATING_THEM_BACK);
    received::<QuestLogUpdate>(&mut app, &client);

    request(
        &mut app,
        &client,
        SetQuestWatched {
            quest_id: BEATING_THEM_BACK,
            watched: false,
        },
    );

    let update = last_update(&mut app, &client);
    assert!(update.watched_quest_ids.is_empty());
    assert!(!update.changed[0].watched);
    assert!(
        load_quest_log(db(&app), client.character_id)
            .watched
            .is_empty()
    );
    app.world_mut()
        .entity_mut(client.connection)
        .remove::<InWorld>();
    app.update();
    let snapshot = enter_world(&mut app, &client);
    assert_eq!(snapshot.entries[0].quest_id, BEATING_THEM_BACK);
    assert!(snapshot.watched_quest_ids.is_empty());
}

#[test]
fn quest_giver_out_of_range_is_too_far() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 50.0);
    enter_world(&mut app, &client);

    accept(&mut app, &client, mcbride, BEATING_THEM_BACK);

    assert_eq!(
        received::<QuestFailed>(&mut app, &client),
        vec![QuestFailed {
            quest_id: BEATING_THEM_BACK,
            reason: QuestFailedReason::TooFar,
        }]
    );
}

/// Moves the player to a WoW-space point and runs a frame.
fn walk_to(app: &mut App, client: &Client, (x, y, z): (f32, f32, f32)) {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    *app.world_mut()
        .get_mut::<Position>(client.player)
        .expect("player position") = Position { x, y, z };
    app.update();
}

fn entry_completed(app: &mut App, client: &Client, quest_id: u32) -> Option<bool> {
    received::<QuestLogUpdate>(app, client)
        .into_iter()
        .flat_map(|update| update.changed)
        .filter(|entry| entry.quest_id == quest_id)
        .map(|entry| entry.completed)
        .last()
}

#[test]
fn exploration_quests_complete_only_when_the_player_enters_their_mine() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 5);
    let dughan = spawn_npc(&mut app, MARSHAL_DUGHAN, 2.0);
    enter_world(&mut app, &client);

    accept(&mut app, &client, dughan, THE_FARGODEEP_MINE);
    let entry = last_update(&mut app, &client).changed.remove(0);
    assert_eq!(entry.quest_id, THE_FARGODEEP_MINE);
    assert!(!entry.completed, "accepting does not explore the mine");
    assert!(
        entry.objectives.is_empty(),
        "the protocol has no exploration kind"
    );
    request(
        &mut app,
        &client,
        QuestGiverCompleteQuest {
            npc: dughan,
            quest_id: THE_FARGODEEP_MINE,
        },
    );
    assert!(!received::<QuestGiverRequestItems>(&mut app, &client)[0].can_complete);

    // Approach: 11 yd from the 10 yd sphere's center is outside it.
    let (x, y, z) = FARGODEEP_TRIGGER;
    walk_to(&mut app, &client, (x + 11.0, y, z));
    walk_to(&mut app, &client, (x + 10.5, y, z));
    assert_eq!(entry_completed(&mut app, &client, THE_FARGODEEP_MINE), None);
    walk_to(&mut app, &client, (x + 2.0, y, z));
    assert_eq!(
        entry_completed(&mut app, &client, THE_FARGODEEP_MINE),
        Some(true)
    );
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(THE_FARGODEEP_MINE)
            .map(|active| active.counts.clone()),
        Some(vec![1])
    );

    walk_to(&mut app, &client, (0.0, 0.0, 0.0));
    turn_in(&mut app, &client, dughan, THE_FARGODEEP_MINE, None);
    assert_eq!(
        received::<QuestGiverQuestComplete>(&mut app, &client)[0].quest_id,
        THE_FARGODEEP_MINE
    );

    accept(&mut app, &client, dughan, THE_JASPERLODE_MINE);
    assert_eq!(
        entry_completed(&mut app, &client, THE_JASPERLODE_MINE),
        Some(false)
    );
    // The Fargodeep trigger does not explore the Jasperlode Mine.
    walk_to(&mut app, &client, (x + 20.0, y, z));
    walk_to(&mut app, &client, FARGODEEP_TRIGGER);
    assert_eq!(
        entry_completed(&mut app, &client, THE_JASPERLODE_MINE),
        None
    );
    let (x, y, z) = JASPERLODE_TRIGGER;
    walk_to(&mut app, &client, (x + 40.0, y, z));
    walk_to(&mut app, &client, (x + 25.0, y, z));
    assert_eq!(
        entry_completed(&mut app, &client, THE_JASPERLODE_MINE),
        Some(true)
    );
}

#[test]
fn a_ghost_entering_the_mine_does_not_explore_it() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 5);
    let dughan = spawn_npc(&mut app, MARSHAL_DUGHAN, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, dughan, THE_FARGODEEP_MINE);
    received::<QuestLogUpdate>(&mut app, &client);

    let (x, y, z) = FARGODEEP_TRIGGER;
    walk_to(&mut app, &client, (x + 30.0, y, z));
    app.world_mut()
        .entity_mut(client.player)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Ghost,
            ..Default::default()
        });
    walk_to(&mut app, &client, FARGODEEP_TRIGGER);
    assert_eq!(entry_completed(&mut app, &client, THE_FARGODEEP_MINE), None);
    assert_eq!(
        load_quest_log(db(&app), client.character_id)
            .active(THE_FARGODEEP_MINE)
            .map(|active| active.counts.clone()),
        Some(vec![0])
    );
}

/// Stormwind Infantry (Northshire): fights the Blackrock Worgs.
const STORMWIND_INFANTRY: u32 = 49869;

fn spawn_worg(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Npc {
                template_id: BLACKROCK_WORG,
                name: "Blackrock Worg".into(),
            },
            Position {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            shared::components::UnitLevel(2),
            shared::components::Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id()
}

/// `Unit::DealDamage(attacker, worg, damage)`, then a frame for the rewards.
fn hit(app: &mut App, attacker: Entity, victim: Entity, damage: f32) {
    crate::combat::deal_direct_damage(app.world_mut(), attacker, victim, damage);
    app.update();
}

fn worg_kills(app: &App, client: &Client) -> Option<Vec<u32>> {
    load_quest_log(db(app), client.character_id)
        .active(BEATING_THEM_BACK)
        .map(|active| active.counts.clone())
}

fn kill_xp(app: &mut App, client: &Client) -> Vec<u32> {
    received::<LogXpGain>(app, client)
        .into_iter()
        .filter(|gain| gain.reason == XpGainReason::Kill)
        .map(|gain| gain.amount)
        .collect()
}

fn beating_them_back_accepted() -> Option<(App, Client, Entity)> {
    let mut app = setup_app()?;
    // The worg's creature template, for kill XP.
    app.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, mcbride, BEATING_THEM_BACK);
    received::<QuestLogUpdate>(&mut app, &client);
    let soldier = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Npc {
                template_id: STORMWIND_INFANTRY,
                name: "Stormwind Infantry".into(),
            },
            origin(),
        ))
        .id();
    Some((app, client, soldier))
}

#[test]
fn a_worg_a_soldier_wounded_and_the_player_killed_credits_the_player() {
    let Some((mut app, client, soldier)) = beating_them_back_accepted() else {
        return;
    };
    let worg = spawn_worg(&mut app);
    hit(&mut app, soldier, worg, 60.0);
    hit(&mut app, client.player, worg, 40.0);
    assert!(app.world().get::<DeathState>(worg).is_some());
    assert_eq!(worg_kills(&app, &client), Some(vec![1]));
    // The player dealt 40 of the 50 health it needed for full XP: 1 - 2 * 10 / 100.
    let partial = kill_xp(&mut app, &client);

    let worg = spawn_worg(&mut app);
    hit(&mut app, client.player, worg, 100.0);
    assert_eq!(worg_kills(&app, &client), Some(vec![2]));
    let full = kill_xp(&mut app, &client);
    assert_eq!(partial.len(), 1);
    assert_eq!(partial, vec![(full[0] as f32 * 0.8) as u32]);
}

#[test]
fn a_worg_the_player_tapped_and_a_soldier_finished_credits_the_player() {
    let Some((mut app, client, soldier)) = beating_them_back_accepted() else {
        return;
    };
    let worg = spawn_worg(&mut app);
    hit(&mut app, client.player, worg, 10.0);
    hit(&mut app, soldier, worg, 90.0);
    assert!(app.world().get::<DeathState>(worg).is_some());
    assert_eq!(worg_kills(&app, &client), Some(vec![1]));
}

#[test]
fn a_worg_only_soldiers_fought_credits_nobody() {
    let Some((mut app, client, soldier)) = beating_them_back_accepted() else {
        return;
    };
    let worg = spawn_worg(&mut app);
    hit(&mut app, soldier, worg, 100.0);
    assert!(app.world().get::<DeathState>(worg).is_some());
    assert_eq!(worg_kills(&app, &client), Some(vec![0]));
    assert!(kill_xp(&mut app, &client).is_empty());
}

/// Guard Thomas (Eastvale Logging Camp) gives Protect the Frontier.
const GUARD_THOMAS: u32 = 261;
const PROTECT_THE_FRONTIER: u32 = 52;
const PROWLER: u32 = 118;
/// `KillCredit1` 118: counts as a Prowler.
const GRAY_FOREST_WOLF: u32 = 1922;

#[test]
fn a_gray_forest_wolf_kill_counts_as_a_prowler() {
    let Some(mut app) = setup_app() else {
        return;
    };
    let client = spawn_client(&mut app, 1);
    let thomas = spawn_npc(&mut app, GUARD_THOMAS, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, thomas, PROTECT_THE_FRONTIER);
    let frontier = |app: &App| {
        load_quest_log(db(app), client.character_id)
            .active(PROTECT_THE_FRONTIER)
            .map(|active| active.counts.clone())
    };
    assert_eq!(frontier(&app), Some(vec![0, 0]));
    kill(&mut app, &client, GRAY_FOREST_WOLF, 2);
    kill(&mut app, &client, PROWLER, 1);
    assert_eq!(frontier(&app), Some(vec![3, 0]));
}

#[path = "quest_spell_tests.rs"]
mod spell;

#[path = "quest_objectives_tests.rs"]
mod objectives;

#[test]
fn source_requirements_quest_progress_turn_in_and_abandon_ignore_forever() {
    use crate::persistence::inventory_store::{load_inventory_record, save_inventory_record};
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
    for id in [2947, 2512, 2101] {
        let mut app = setup_app().expect("world quest fixture required");
        let client = spawn_client(&mut app, 2);
        let rainer = spawn_npc(&mut app, DEPUTY_RAINER, 2.0);
        let plain = app.world().resource::<ItemTemplates>().retail[&PAINTED_GNOLL_ARMBAND];
        {
            let mut templates = app.world_mut().resource_mut::<ItemTemplates>();
            templates.retail.insert(id, plain);
            templates.forever.insert(id, plain);
        }
        {
            let mut quests = app.world_mut().resource_mut::<QuestDb>();
            let quest = quests.quests.get_mut(&RIVERPAW_GNOLL_BOUNTY).unwrap();
            quest.objectives[0].object_id = id;
            quest.start_item = id;
            quest.provided_item_count = 1;
        }
        mark_rewarded(&app, &client, WESTBROOK_GARRISON_NEEDS_HELP);
        enter_world(&mut app, &client);
        accept(&mut app, &client, rainer, RIVERPAW_GNOLL_BOUNTY);
        let mut record = load_inventory_record(db(&app), client.character_id).unwrap();
        let provided = record
            .inventory
            .items
            .iter_mut()
            .find(|item| item.item_id == id)
            .unwrap();
        provided.stack_count = 8;
        provided.item_guid = persistence::next_id(db(&app), "item_instance");
        let guid = provided.item_guid;
        record.definition_sources.insert(guid, Forever70205);
        save_inventory_record(db(&app), client.character_id, &record);
        app.world_mut()
            .resource_mut::<GoldUpdateQueue>()
            .record_items(client.character_id);
        app.update();
        request(
            &mut app,
            &client,
            QuestGiverCompleteQuest {
                npc: rainer,
                quest_id: RIVERPAW_GNOLL_BOUNTY,
            },
        );
        let request_items = received::<QuestGiverRequestItems>(&mut app, &client);
        assert_eq!(
            request_items.len(),
            1,
            "Forever-only {id} must not complete quest"
        );
        assert!(!request_items[0].can_complete);
        turn_in(&mut app, &client, rainer, RIVERPAW_GNOLL_BOUNTY, Some(1));
        assert_eq!(
            received::<QuestFailed>(&mut app, &client)[0].reason,
            QuestFailedReason::ObjectivesIncomplete
        );
        let mut record = load_inventory_record(db(&app), client.character_id).unwrap();
        let retail_guid = persistence::next_id(db(&app), "item_instance");
        record.inventory.items.push(InventoryItemData {
            item_guid: retail_guid,
            stack_count: 9,
            ..record
                .inventory
                .items
                .iter()
                .find(|item| item.item_guid == guid)
                .unwrap()
                .clone()
        });
        record.definition_sources.insert(retail_guid, Retail);
        save_inventory_record(db(&app), client.character_id, &record);
        app.world_mut()
            .resource_mut::<GoldUpdateQueue>()
            .record_items(client.character_id);
        app.update();
        turn_in(&mut app, &client, rainer, RIVERPAW_GNOLL_BOUNTY, Some(1));
        assert_eq!(
            received::<QuestGiverQuestComplete>(&mut app, &client).len(),
            1
        );
        let inventory = persistence::load_inventory(db(&app), client.character_id);
        assert_eq!(
            inventory
                .items
                .iter()
                .find(|item| item.item_guid == guid)
                .unwrap()
                .stack_count,
            8
        );
        assert!(
            !inventory
                .items
                .iter()
                .any(|item| item.item_guid == retail_guid)
        );
        // Abandon also must not remove a same-ID Forever provided-item collision.
        let mut log = load_quest_log(db(&app), client.character_id);
        log.active
            .push(crate::persistence::quest_store::ActiveQuestData {
                quest_id: RIVERPAW_GNOLL_BOUNTY,
                counts: vec![0],
                ..Default::default()
            });
        save_quest_log(db(&app), client.character_id, &log);
        request(
            &mut app,
            &client,
            AbandonQuest {
                quest_id: RIVERPAW_GNOLL_BOUNTY,
            },
        );
        let inventory = persistence::load_inventory(db(&app), client.character_id);
        assert_eq!(
            inventory
                .items
                .iter()
                .find(|item| item.item_guid == guid)
                .unwrap()
                .stack_count,
            8
        );
    }
}
