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
    InventorySnapshot, QuestFailed, QuestFailedReason, QuestGiverAcceptQuest,
    QuestGiverChooseReward, QuestGiverCompleteQuest, QuestGiverHello, QuestGiverOfferReward,
    QuestGiverQueryQuest, QuestGiverQuestComplete, QuestGiverQuestDetails, QuestGiverQuestList,
    QuestGiverQuestState, QuestGiverRequestItems, QuestGiverStatus, QuestGiverStatusEntry,
    QuestGiverStatusQuery, QuestRewardItem, SetQuestWatched, SortBags, SplitItem, SwapItem,
    UseItem,
};

use super::*;
use crate::combat::DeathState;
use crate::item_templates::{ItemTemplates, load_item_templates};
use crate::persistence::{InventoryItemData, temp_database};
use crate::quest_data::test_quest_db;

const DEPUTY_WILLEM: u32 = 823;
const MARSHAL_MCBRIDE: u32 = 197;
const KOBOLD_VERMIN: u32 = 6;
const A_THREAT_WITHIN: u32 = 783;
const KOBOLD_CAMP_CLEANUP: u32 = 7;
const BROTHERHOOD_OF_THIEVES: u32 = 18;
const RED_BURLAP_BANDANA: u32 = 752;
const MILITIA_DAGGER: u32 = 2224;
const MILITIA_HAMMER: u32 = 5580;

fn item_templates() -> ItemTemplates {
    static TEMPLATES: OnceLock<ItemTemplates> = OnceLock::new();
    TEMPLATES
        .get_or_init(|| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
            load_item_templates(&path).expect("load item templates")
        })
        .clone()
}

fn setup_app() -> Option<App> {
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
    };
}

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
            Npc {
                template_id,
                name: String::new(),
            },
            origin(),
            LastDamager(client.player),
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
fn a_threat_within_accepted_from_willem_and_turned_in_at_mcbride_is_persisted() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let willem = spawn_npc(&mut app, DEPUTY_WILLEM, 2.0);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 3.0);
    assert!(enter_world(&mut app, &client).entries.is_empty());
    assert_eq!(
        statuses(&mut app, &client, &[willem, mcbride]),
        vec![QuestGiverStatus::Available, QuestGiverStatus::None]
    );

    request(&mut app, &client, QuestGiverHello { npc: willem });
    let list = received::<QuestGiverQuestList>(&mut app, &client)
        .pop()
        .unwrap();
    assert_eq!(list.quests[0].quest_id, A_THREAT_WITHIN);
    assert_eq!(list.quests[0].state, QuestGiverQuestState::Available);
    accept(&mut app, &client, willem, A_THREAT_WITHIN);

    let update = last_update(&mut app, &client);
    assert_eq!(update.changed.len(), 1);
    let entry = &update.changed[0];
    assert_eq!(
        (entry.quest_id, entry.title.as_str()),
        (A_THREAT_WITHIN, "A Threat Within")
    );
    assert!(entry.completed && entry.watched && entry.objectives.is_empty());
    assert_eq!(update.watched_quest_ids, vec![A_THREAT_WITHIN]);
    let markers = received::<QuestGiverStatusMultiple>(&mut app, &client)
        .pop()
        .unwrap();
    assert!(markers.statuses.contains(&QuestGiverStatusEntry {
        npc: mcbride,
        status: QuestGiverStatus::Reward,
    }));

    request(
        &mut app,
        &client,
        QuestGiverCompleteQuest {
            npc: mcbride,
            quest_id: A_THREAT_WITHIN,
        },
    );
    assert_eq!(
        received::<QuestGiverOfferReward>(&mut app, &client).len(),
        1
    );
    turn_in(&mut app, &client, mcbride, A_THREAT_WITHIN, None);

    let complete = received::<QuestGiverQuestComplete>(&mut app, &client);
    assert_eq!(complete[0].quest_id, A_THREAT_WITHIN);
    assert_eq!(
        last_update(&mut app, &client).removed,
        vec![A_THREAT_WITHIN]
    );
    let next = received::<QuestGiverQuestDetails>(&mut app, &client);
    assert_eq!(next[0].quest_id, KOBOLD_CAMP_CLEANUP);
    let saved = load_quest_log(db(&app), client.character_id);
    assert!(saved.active.is_empty() && saved.watched.is_empty());
    assert!(saved.is_rewarded(A_THREAT_WITHIN));
    // QuestXP level 1, RewardXPDifficulty 3; level 2 needs 250.
    assert_eq!(
        crate::persistence_progression::load_xp(db(&app), client.character_id),
        100
    );
    assert_eq!(
        statuses(&mut app, &client, &[mcbride]),
        vec![QuestGiverStatus::Available]
    );
}

#[test]
fn kobold_camp_cleanup_is_blocked_until_a_threat_within_is_turned_in() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    enter_world(&mut app, &client);

    request(
        &mut app,
        &client,
        QuestGiverQueryQuest {
            npc: mcbride,
            quest_id: KOBOLD_CAMP_CLEANUP,
        },
    );
    let expected = vec![QuestFailed {
        quest_id: KOBOLD_CAMP_CLEANUP,
        reason: QuestFailedReason::DontHaveRequirement,
    }];
    assert_eq!(received::<QuestFailed>(&mut app, &client), expected);
    assert!(received::<QuestGiverQuestDetails>(&mut app, &client).is_empty());
    accept(&mut app, &client, mcbride, KOBOLD_CAMP_CLEANUP);
    assert_eq!(received::<QuestFailed>(&mut app, &client), expected);
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active
            .is_empty()
    );

    mark_rewarded(&app, &client, A_THREAT_WITHIN);
    accept(&mut app, &client, mcbride, KOBOLD_CAMP_CLEANUP);
    assert!(received::<QuestFailed>(&mut app, &client).is_empty());
    assert!(
        load_quest_log(db(&app), client.character_id)
            .active(KOBOLD_CAMP_CLEANUP)
            .is_some()
    );
}

#[test]
fn kobold_vermin_kills_progress_kobold_camp_cleanup_to_turn_in() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let mcbride = spawn_npc(&mut app, MARSHAL_MCBRIDE, 2.0);
    mark_rewarded(&app, &client, A_THREAT_WITHIN);
    enter_world(&mut app, &client);
    accept(&mut app, &client, mcbride, KOBOLD_CAMP_CLEANUP);
    received::<QuestLogUpdate>(&mut app, &client);
    let gold_before = persistence::load_inventory(db(&app), client.character_id).gold;

    kill(&mut app, &client, KOBOLD_VERMIN, 3);
    let objective = &last_update(&mut app, &client).changed[0].objectives[0];
    assert_eq!(
        (
            objective.text.as_str(),
            objective.current,
            objective.required
        ),
        ("Kobold Vermin slain", 3, 8)
    );
    kill(&mut app, &client, KOBOLD_VERMIN, 6);
    let entry = &last_update(&mut app, &client).changed[0];
    assert_eq!(entry.objectives[0].current, 8);
    assert!(entry.completed && entry.objectives[0].completed);
    assert!(!entry.pois.is_empty());

    turn_in(&mut app, &client, mcbride, KOBOLD_CAMP_CLEANUP, None);
    assert_eq!(
        received::<QuestGiverQuestComplete>(&mut app, &client)[0].money,
        25
    );
    let inventory = persistence::load_inventory(db(&app), client.character_id);
    assert_eq!(inventory.gold, gold_before + 25);
    assert!(load_quest_log(db(&app), client.character_id).is_rewarded(KOBOLD_CAMP_CLEANUP));
}

fn grant_bandanas(app: &mut App, client: &Client, count: u32) {
    let db = db(app);
    let mut inventory = persistence::load_inventory(db, client.character_id);
    inventory.items.push(InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id: RED_BURLAP_BANDANA,
        name: "Red Burlap Bandana".into(),
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
fn brotherhood_of_thieves_counts_bag_items_and_grants_the_chosen_reward() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 2);
    let willem = spawn_npc(&mut app, DEPUTY_WILLEM, 2.0);
    mark_rewarded(&app, &client, A_THREAT_WITHIN);
    enter_world(&mut app, &client);
    accept(&mut app, &client, willem, BROTHERHOOD_OF_THIEVES);
    received::<QuestLogUpdate>(&mut app, &client);

    grant_bandanas(&mut app, &client, 8);
    let entry = &last_update(&mut app, &client).changed[0];
    assert_eq!(entry.objectives[0].text, "Red Burlap Bandana");
    assert_eq!((entry.objectives[0].current, entry.completed), (8, true));

    request(
        &mut app,
        &client,
        QuestGiverCompleteQuest {
            npc: willem,
            quest_id: BROTHERHOOD_OF_THIEVES,
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
    // Militia Dagger, Hammer, Shortsword, Warhammer, Quarterstaff.
    assert_eq!(
        choices,
        vec![MILITIA_DAGGER, MILITIA_HAMMER, 1161, 5579, 1159]
    );
    turn_in(&mut app, &client, willem, BROTHERHOOD_OF_THIEVES, None);
    assert_eq!(
        received::<QuestFailed>(&mut app, &client)[0].reason,
        QuestFailedReason::InvalidRewardChoice
    );
    turn_in(&mut app, &client, willem, BROTHERHOOD_OF_THIEVES, Some(1));

    let complete = received::<QuestGiverQuestComplete>(&mut app, &client)
        .pop()
        .unwrap();
    assert_eq!(
        complete.items,
        vec![QuestRewardItem {
            item_id: MILITIA_HAMMER,
            name: "Militia Hammer".into(),
            count: 1,
        }]
    );
    let items = bag_items(&app, &client);
    assert!(items.contains(&(MILITIA_HAMMER, 1)));
    assert!(
        !items
            .iter()
            .any(|&(id, _)| id == MILITIA_DAGGER || id == RED_BURLAP_BANDANA)
    );
}

#[test]
fn abandoning_a_threat_within_removes_it_and_willem_offers_it_again() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let willem = spawn_npc(&mut app, DEPUTY_WILLEM, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, willem, A_THREAT_WITHIN);
    received::<QuestLogUpdate>(&mut app, &client);

    request(
        &mut app,
        &client,
        AbandonQuest {
            quest_id: A_THREAT_WITHIN,
        },
    );

    let update = last_update(&mut app, &client);
    assert_eq!(
        (update.removed, update.watched_quest_ids),
        (vec![A_THREAT_WITHIN], vec![])
    );
    let saved = load_quest_log(db(&app), client.character_id);
    assert!(saved.active.is_empty() && saved.rewarded.is_empty());
    assert_eq!(
        statuses(&mut app, &client, &[willem]),
        vec![QuestGiverStatus::Available]
    );
}

#[test]
fn untracking_a_quest_persists_and_resends_the_watch_list() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let willem = spawn_npc(&mut app, DEPUTY_WILLEM, 2.0);
    enter_world(&mut app, &client);
    accept(&mut app, &client, willem, A_THREAT_WITHIN);
    received::<QuestLogUpdate>(&mut app, &client);

    request(
        &mut app,
        &client,
        SetQuestWatched {
            quest_id: A_THREAT_WITHIN,
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
    assert_eq!(snapshot.entries[0].quest_id, A_THREAT_WITHIN);
    assert!(snapshot.watched_quest_ids.is_empty());
}

#[test]
fn quest_giver_out_of_range_is_too_far() {
    let Some(mut app) = setup_app() else { return };
    let client = spawn_client(&mut app, 1);
    let willem = spawn_npc(&mut app, DEPUTY_WILLEM, 50.0);
    enter_world(&mut app, &client);

    accept(&mut app, &client, willem, A_THREAT_WITHIN);

    assert_eq!(
        received::<QuestFailed>(&mut app, &client),
        vec![QuestFailed {
            quest_id: A_THREAT_WITHIN,
            reason: QuestFailedReason::TooFar,
        }]
    );
}
