//! Quest flow through the real message path, with world.db quest rows and a
//! temp redb; each test skips when world.db is absent.
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{AppMessageExt, Connected, Message, PeerId, RemoteId};
use shared::components::{CharacterAppearance, EquipmentAppearance};
use shared::protocol::{
    AbandonQuest, DestroyItem, EquipItem, EquipmentSnapshot, InventoryDelta, InventoryError,
    InventorySnapshot, LogXpGain, PlayerXpUpdate, QuestFailed, QuestFailedReason,
    QuestGiverAcceptQuest, QuestGiverChooseReward, QuestGiverCompleteQuest, QuestGiverHello,
    QuestGiverOfferReward, QuestGiverQueryQuest, QuestGiverQuestComplete, QuestGiverQuestDetails,
    QuestGiverQuestList, QuestGiverQuestState, QuestGiverRequestItems, QuestGiverStatus,
    QuestGiverStatusEntry, QuestGiverStatusQuery, QuestMarkerClass, QuestRewardItem,
    SetQuestWatched, SortBags, SplitItem, SwapItem, UseItem, XpGainReason,
};

use super::*;
use crate::combat::DeathState;
use crate::item_templates::{ItemTemplates, load_item_templates};
use crate::persistence::{InventoryItemData, temp_database};
use crate::quest_data::test_quest_db;

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

fn setup_app() -> Option<App> {
    setup_app_with(|_| {})
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
    app.insert_resource(GameDatabase {
        db: temp_database(),
    });
    app.init_resource::<GoldUpdateQueue>();
    app.insert_resource(item_templates());
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
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) =
        persistence::create_character(db, 1, "Questor", 1, 1).expect("create character");
    let player = app
        .world_mut()
        .spawn((
            crate::maps::InMap::continent(0),
            Player {
                name: "Questor".into(),
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
fn riverpaw_gnoll_bounty_counts_bag_items_and_grants_the_chosen_reward() {
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
    assert!(
        !items
            .iter()
            .any(|&(id, _)| id == MILITIA_BUCKLER || id == PAINTED_GNOLL_ARMBAND)
    );
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
