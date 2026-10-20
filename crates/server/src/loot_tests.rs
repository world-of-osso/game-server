//! Real-DB tests against data/world.db.

use std::path::PathBuf;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Channel, Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::components::{CharacterAppearance, Player};

use super::*;
use crate::combat::{CorpseDelay, RespawnDelay, mark_npc_dead_world};
use crate::inventory::free_bag_cells;
use crate::item_templates::load_item_templates;
use crate::merchant::load_vendor_catalog;
use crate::persistence::ClientId;

#[path = "loot/group_tests.rs"]
mod group_loot;

#[path = "loot/encounter_tests.rs"]
mod encounter_loot;

const KOBOLD_VERMIN: u32 = 6;
const MELTED_CANDLE: u32 = 755;
const TARNISHED_CHAIN_VEST: u32 = 2379;

fn world_db_path() -> Option<PathBuf> {
    let path = crate::test_data::world_db_path();
    Some(path)
}

#[derive(Resource)]
struct Inbox<M>(Vec<M>);

fn collect<M: lightyear::prelude::Message>(
    mut receivers: Query<&mut MessageReceiver<M>>,
    mut inbox: ResMut<Inbox<M>>,
) {
    for mut receiver in &mut receivers {
        inbox.0.extend(receiver.receive());
    }
}

fn add_inbox<M: lightyear::prelude::Message>(app: &mut App) {
    app.insert_resource(Inbox::<M>(Vec::new()));
    app.add_systems(First, collect::<M>);
}

type Pair<M> = (MessageSender<M>, MessageReceiver<M>);

fn pair<M: lightyear::prelude::Message>() -> Pair<M> {
    (MessageSender::default(), MessageReceiver::default())
}

struct Harness {
    app: App,
    looter: Looter,
}

#[derive(Clone, Copy)]
struct Looter {
    character_id: u64,
    player: Entity,
    connection: Entity,
}

#[test]
fn achievements_criteria_native_any_item_receipts_use_canonical_quality_and_count_once() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let mut h = harness_with_plugins(false, |app| {
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
        app.insert_resource(GameDatabase::new(db));
        app.add_plugins(AchievementPlugin);
    })
    .unwrap();
    let player = h.looter.player;
    let character = h.looter.character_id;
    h.app
        .world_mut()
        .entity_mut(h.looter.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    h.app.world_mut().entity_mut(player).insert((
        shared::components::Level(1),
        PlayerAchievementState::default(),
    ));
    h.app.update();
    // Controlled receipt input uses authentic item templates, not an authored Kobold drop claim.
    // The wire slot quality deliberately disagrees; only canonical template quality may gate.
    for (item_id, count, shown_quality, name) in
        [(124101, 5, 4, "Aethril"), (17203, 3, 0, "Sulfuron Ingot")]
    {
        let corpse = h.kill_kobold();
        let loot = CorpseLoot {
            looters: [(
                character,
                vec![Some(LootContent::Item {
                    item_id,
                    count,
                    quality: shown_quality,
                    name: name.into(),
                })],
            )]
            .into(),
        };
        h.app.world_mut().entity_mut(corpse).insert(loot);
        h.take_slot(corpse, 0);
        assert_eq!(
            h.count_of(item_id),
            0,
            "refused slot request cannot acquire anything"
        );
        assert!(
            h.app
                .world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .get(&6140)
                .is_none()
        );
        h.open(corpse, false);
        h.take_slot(corpse, 0);
        assert_eq!(
            h.count_of(item_id),
            count,
            "native settlement stores actual stack quantity"
        );
        let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
        if item_id == 124101 {
            assert_eq!(
                state.criteria_progress.get(&105347),
                Some(&1),
                "ItemId19, one receipt despite five items"
            );
            assert!(
                !state.criteria_progress.contains_key(&6140),
                "display quality cannot turn Aethril epic"
            );
            assert!(!state.criteria_progress.contains_key(&6142));
        } else {
            assert_eq!(
                state.criteria_progress.get(&6140),
                Some(&1),
                "one loot receipt despite three items"
            );
            assert_eq!(
                state.criteria_progress.get(&6142),
                Some(&1),
                "looting also acquires once"
            );
        }
        h.take_slot(corpse, 0);
        h.settle();
    }
    let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
    assert_eq!(state.criteria_progress.get(&6140), Some(&1));
    assert_eq!(state.criteria_progress.get(&6142), Some(&1));
    assert_eq!(state.criteria_progress.get(&105347), Some(&1));
    drop(h);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(saved.criteria_progress.get(&6140), Some(&1));
    assert_eq!(saved.criteria_progress.get(&6142), Some(&1));
    assert_eq!(saved.criteria_progress.get(&105347), Some(&1));
}

#[test]
fn achievements_criteria_loot_money_counts_committed_coins_not_refusals_or_retakes() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let mut h = harness_with_plugins(false, |app| {
        app.insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
        app.add_plugins(crate::achievement::AchievementPlugin);
    })
    .expect("authentic loot harness");
    let player = h.looter.player;
    h.app
        .world_mut()
        .entity_mut(h.looter.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::achievement::PlayerAchievementState::default());
    let corpse = h.kill_kobold();
    h.stock(corpse);
    h.take_slot(corpse, 0);
    assert_eq!(h.inventory().gold, 0);
    assert!(
        !persistence::load_achievements(
            &h.app.world().resource::<GameDatabase>().db,
            h.looter.character_id
        )
        .criteria_progress
        .contains_key(&4093)
    );
    h.open(corpse, false);
    h.take_slot(corpse, 0);
    assert_eq!(h.inventory().gold, 3);
    assert_eq!(
        persistence::load_achievements(
            &h.app.world().resource::<GameDatabase>().db,
            h.looter.character_id
        )
        .criteria_progress
        .get(&4093),
        Some(&3)
    );
    h.take_slot(corpse, 0);
    h.settle();
    assert_eq!(
        persistence::load_achievements(
            &h.app.world().resource::<GameDatabase>().db,
            h.looter.character_id
        )
        .criteria_progress
        .get(&4093),
        Some(&3)
    );
    let corpse2 = h.kill_kobold();
    h.stock(corpse2);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let mut inventory = h.inventory();
    inventory.gold = MAX_MONEY;
    persistence::save_inventory(db, h.looter.character_id, &inventory);
    h.open(corpse2, false);
    h.take_slot(corpse2, 0);
    assert_eq!(h.inventory().gold, MAX_MONEY);
    assert_eq!(
        persistence::load_achievements(
            &h.app.world().resource::<GameDatabase>().db,
            h.looter.character_id
        )
        .criteria_progress
        .get(&4093),
        Some(&3)
    );
}

fn harness(tables: bool) -> Option<Harness> {
    harness_with_plugins(tables, |_| {})
}

fn harness_with_plugins(tables: bool, extra: impl FnOnce(&mut App)) -> Option<Harness> {
    let path = world_db_path()?;
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(shared::protocol::ProtocolPlugin);
    add_inbox::<LootResponse>(&mut app);
    add_inbox::<LootSlotRemoved>(&mut app);
    add_inbox::<LootClosed>(&mut app);
    add_inbox::<CorpseLootable>(&mut app);
    add_inbox::<LootFailed>(&mut app);
    add_inbox::<shared::protocol::GroupLootRoll>(&mut app);
    app.init_resource::<crate::group::GroupRegistry>();
    app.insert_resource(load_item_templates(&path).unwrap());
    app.insert_resource(load_vendor_catalog(&conn).unwrap());
    crate::appearance_collection::tests::install_database(&mut app);
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins(LootPlugin);
    if tables {
        app.insert_resource(load_loot_tables(&conn).unwrap());
    }
    extra(&mut app);
    app.finish();
    app.cleanup();
    let looter = spawn_looter(&mut app, "Looter", 1);
    app.update();
    Some(Harness { app, looter })
}

fn spawn_looter(app: &mut App, name: &str, peer: u64) -> Looter {
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) = persistence::create_character(db, peer, name, 1, 1).unwrap();
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
            ClientId(character_id),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(peer)),
            Connected,
            HostClient { buffer: Vec::new() },
            InWorld { character_id },
            PlayerLink(player),
            (
                pair::<LootUnit>(),
                pair::<LootSlotRequest>(),
                pair::<LootRelease>(),
                pair::<LootResponse>(),
                pair::<LootSlotRemoved>(),
                pair::<LootClosed>(),
                pair::<CorpseLootable>(),
                pair::<LootFailed>(),
            ),
        ))
        .id();
    app.world_mut().entity_mut(connection).insert((
        pair::<shared::protocol::GroupLootRoll>(),
        pair::<shared::protocol::GroupLootChoice>(),
        pair::<shared::protocol::GroupLootHistoryRequest>(),
    ));
    Looter {
        character_id,
        player,
        connection,
    }
}

impl Harness {
    /// A Kobold Vermin 2 yd away, killed by the looter.
    fn kill_kobold(&mut self) -> Entity {
        let kobold = self
            .app
            .world_mut()
            .spawn((
                crate::maps::InMap::continent(0),
                Npc {
                    template_id: KOBOLD_VERMIN,
                    name: "Kobold Vermin".into(),
                },
                Position {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 0.0,
                    max: 100.0,
                },
                crate::creature_tap::CreatureTap::tapped_by(self.looter.character_id),
                RespawnDelay::for_spawn(180, 0),
                CorpseDelay::for_rank(0),
            ))
            .id();
        mark_npc_dead_world(self.app.world_mut(), kobold);
        self.settle();
        kobold
    }

    /// Replace the rolled loot with 3 copper and a Melted Candle.
    fn stock(&mut self, corpse: Entity) {
        let loot = CorpseLoot {
            looters: [(
                self.looter.character_id,
                vec![
                    Some(LootContent::Money { copper: 3 }),
                    Some(LootContent::Item {
                        item_id: MELTED_CANDLE,
                        name: "Melted Candle".into(),
                        quality: 0,
                        count: 1,
                    }),
                ],
            )]
            .into(),
        };
        self.app.world_mut().entity_mut(corpse).insert(loot);
    }

    fn send<M: lightyear::prelude::Message, C: Channel>(&mut self, from: Looter, message: M) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<M>>(from.connection)
            .unwrap()
            .send::<C>(message);
        self.settle();
    }

    fn open(&mut self, corpse: Entity, auto: bool) {
        let looter = self.looter;
        self.send::<_, LootChannel>(
            looter,
            LootUnit {
                corpse: corpse.to_bits(),
                auto,
            },
        );
    }

    fn take_slot(&mut self, corpse: Entity, slot: u8) {
        let looter = self.looter;
        self.send::<_, LootChannel>(
            looter,
            LootSlotRequest {
                corpse: corpse.to_bits(),
                slot,
            },
        );
    }

    /// Frames for a sent message to reach its inbox.
    fn settle(&mut self) {
        for _ in 0..3 {
            self.app.update();
        }
    }

    fn take<M: lightyear::prelude::Message>(&mut self) -> Vec<M> {
        std::mem::take(&mut self.app.world_mut().resource_mut::<Inbox<M>>().0)
    }

    fn inventory(&self) -> persistence::InventoryData {
        persistence::load_inventory(
            &self.app.world().resource::<GameDatabase>().db,
            self.looter.character_id,
        )
    }

    fn count_of(&self, item_id: u32) -> u32 {
        self.inventory()
            .items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| item.stack_count)
            .sum()
    }

    fn failures(&mut self) -> Vec<LootError> {
        self.take::<LootFailed>()
            .into_iter()
            .map(|failed| failed.error)
            .collect()
    }

    fn corpse(&self, corpse: Entity) -> Corpse {
        *self.app.world().get::<Corpse>(corpse).unwrap()
    }
}

fn lootable(corpse: Entity, lootable: bool) -> CorpseLootable {
    CorpseLootable {
        corpse: corpse.to_bits(),
        lootable,
    }
}

#[test]
fn world_quest_bound_loot_persists_credit_without_creating_a_bag_item_or_replaying() {
    use crate::persistence::quest_store::{
        ActiveQuestData, QuestLogData, load_quest_log, save_quest_log,
    };
    use crate::quest_data::{ObjectiveDef, ObjectiveKind, QuestDb, QuestTemplate};
    use crate::world_quest_objectives::ObjectiveRule;
    let mut h = harness(true).unwrap();
    let mut quests = QuestDb::default();
    quests.quests.insert(
        80295,
        QuestTemplate {
            id: 80295,
            objectives: vec![ObjectiveDef {
                kind: ObjectiveKind::Item,
                object_id: MELTED_CANDLE,
                required: 2,
                text: "Bound item".into(),
            }],
            world_objective_rules: Some(vec![ObjectiveRule {
                flags2: 1,
                ..Default::default()
            }]),
            ..Default::default()
        },
    );
    h.app.insert_resource(quests);
    h.app.init_resource::<crate::quest::QuestDirty>();
    let log = QuestLogData {
        active: vec![ActiveQuestData {
            quest_id: 80295,
            counts: vec![0],
            ..Default::default()
        }],
        ..Default::default()
    };
    let db = &h.app.world().resource::<GameDatabase>().db;
    save_quest_log(db, h.looter.character_id, &log);
    let corpse = h.kill_kobold();
    h.stock(corpse);
    h.open(corpse, false);
    h.take_slot(corpse, 1);
    assert!(h.failures().is_empty());
    assert_eq!(
        h.count_of(MELTED_CANDLE),
        0,
        "bound objective suppresses item creation"
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        load_quest_log(db, h.looter.character_id)
            .active(80295)
            .unwrap()
            .counts,
        vec![1]
    );
    assert!(
        h.app
            .world()
            .resource::<crate::quest::QuestDirty>()
            .0
            .contains(&h.looter.character_id)
    );
    h.take_slot(corpse, 1);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        load_quest_log(db, h.looter.character_id)
            .active(80295)
            .unwrap()
            .counts,
        vec![1]
    );
    let other = h.kill_kobold();
    h.stock(other);
    h.open(other, false);
    h.take_slot(other, 1);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        load_quest_log(db, h.looter.character_id)
            .active(80295)
            .unwrap()
            .counts,
        vec![2]
    );
    assert_eq!(h.count_of(MELTED_CANDLE), 0);
    assert!(!crate::quest_rules::quest_needs_item(
        h.app.world().resource::<QuestDb>(),
        &load_quest_log(db, h.looter.character_id),
        MELTED_CANDLE,
        |_| 0
    ));
}

#[test]
fn killing_a_kobold_vermin_gives_its_killer_lootable_coins() {
    let Some(mut h) = harness(true) else { return };
    let kobold = h.kill_kobold();

    assert_eq!(h.take::<CorpseLootable>(), vec![lootable(kobold, true)]);
    let loot = h.app.world().get::<CorpseLoot>(kobold).unwrap();
    let slots = &loot.looters[&h.looter.character_id];
    let Some(LootContent::Money { copper }) = slots[0] else {
        panic!("money slot first: {slots:?}");
    };
    // mingold 1, maxgold 5, plus the sell price of any grey item the roll yielded.
    assert!(copper >= 1, "{copper}");
}

#[test]
fn looting_both_slots_moves_coins_and_candle_into_the_bags_and_ends_the_sparkle() {
    let Some(mut h) = harness(true) else { return };
    let kobold = h.kill_kobold();
    h.stock(kobold);
    h.take::<CorpseLootable>();
    let gold_before = h.inventory().gold;

    h.open(kobold, false);
    let responses = h.take::<LootResponse>();
    assert_eq!(responses.len(), 1);
    assert!(!responses[0].auto);
    assert_eq!(
        responses[0]
            .slots
            .iter()
            .map(|slot| slot.slot)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    let decay_before = h.corpse(kobold).remaining;

    h.take_slot(kobold, 0);
    assert_eq!(h.inventory().gold, gold_before + 3);
    assert!(
        h.take::<LootClosed>().is_empty(),
        "the candle is still there"
    );
    h.take_slot(kobold, 1);

    assert_eq!(h.count_of(MELTED_CANDLE), 1);
    let removed: Vec<u8> = h.take::<LootSlotRemoved>().iter().map(|r| r.slot).collect();
    assert_eq!(removed, [0, 1]);
    assert_eq!(h.take::<CorpseLootable>(), vec![lootable(kobold, false)]);
    assert_eq!(h.take::<LootClosed>().len(), 1);
    assert!(
        h.app
            .world()
            .get::<ActiveLoot>(h.looter.connection)
            .is_none()
    );
    // All loot taken: the rest of the 60 s decay is halved.
    let decay = h.corpse(kobold).remaining;
    assert!(
        (decay - decay_before / 2.0).abs() < 0.5,
        "{decay} vs {decay_before}"
    );
    assert!(h.failures().is_empty());
}

fn loot_appearance_item(item_id: u32, appearance: u32, collected: bool) {
    let mut h = harness(true).unwrap();
    let kobold = h.kill_kobold();
    h.app.world_mut().entity_mut(kobold).insert(CorpseLoot {
        looters: [(
            h.looter.character_id,
            vec![Some(LootContent::Item {
                item_id,
                name: "Appearance loot".into(),
                quality: 2,
                count: 1,
            })],
        )]
        .into(),
    });
    h.open(kobold, true);
    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(item_id), 1);
    assert_eq!(
        crate::persistence::appearance_store::load_account_appearances(
            &h.app.world().resource::<GameDatabase>().db,
            1
        )
        .has(appearance),
        collected
    );
    crate::appearance_collection::tests::assert_restart(h.app, 1, &[(appearance, collected)]);
}

#[test]
fn guild_news_committed_loot_uses_authentic_quality_and_excludes_refusal_retake() {
    let mut h = harness(true).unwrap();
    let db = &h.app.world().resource::<GameDatabase>().db;
    crate::persistence_character::set_character_guild_by_id(
        db,
        h.looter.character_id,
        7,
        "Looters",
    )
    .unwrap();
    // Controlled slot receipt, not a claim that a Kobold drops Thunderfury.
    let corpse = h.kill_kobold();
    h.app.world_mut().entity_mut(corpse).insert(CorpseLoot {
        looters: [(
            h.looter.character_id,
            vec![Some(LootContent::Item {
                item_id: 19019,
                name: "Thunderfury, Blessed Blade of the Windseeker".into(),
                quality: 0,
                count: 1,
            })],
        )]
        .into(),
    });
    h.take_slot(corpse, 0);
    assert_eq!(h.count_of(19019), 0);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(
        crate::guild_news::query(db, h.looter.character_id)
            .entries
            .is_empty()
    );
    h.open(corpse, false);
    h.take_slot(corpse, 0);
    assert_eq!(h.count_of(19019), 1);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let entries = crate::guild_news::query(db, h.looter.character_id).entries;
    assert_eq!(entries.len(), 1);
    assert!(
        matches!(&entries[0].kind, shared::protocol::GuildNewsKind::ItemLooted {
        member, item_id: 19019
    } if member == "Looter")
    );
    h.take_slot(corpse, 0);
    let ordinary = h.kill_kobold();
    h.stock(ordinary);
    h.open(ordinary, true);
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        crate::guild_news::query(db, h.looter.character_id).entries,
        entries
    );
}

#[test]
fn transmogacq_looting_class_restricted_bop_collects_on_a_different_class() {
    // Warrior loots paladin-only Verigan's Fist: collection is not equip eligibility.
    loot_appearance_item(6953, 2288, true);
}

#[test]
fn transmogacq_looting_an_unbound_boe_does_not_collect() {
    loot_appearance_item(25, 154, false);
}

#[test]
fn auto_loot_takes_every_slot_at_once() {
    let Some(mut h) = harness(true) else { return };
    let kobold = h.kill_kobold();
    h.stock(kobold);
    let gold_before = h.inventory().gold;

    h.open(kobold, true);

    assert!(h.take::<LootResponse>()[0].auto);
    assert_eq!(h.inventory().gold, gold_before + 3);
    assert_eq!(h.count_of(MELTED_CANDLE), 1);
    assert_eq!(h.take::<LootClosed>().len(), 1);
    assert!(
        h.app
            .world()
            .get::<ActiveLoot>(h.looter.connection)
            .is_none()
    );
}

#[test]
fn an_item_that_does_not_fit_stays_in_the_window() {
    let Some(mut h) = harness(true) else { return };
    let kobold = h.kill_kobold();
    h.stock(kobold);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let templates = h.app.world().resource::<ItemTemplates>();
    let free = free_bag_cells(db, templates, h.looter.character_id);
    let mut inventory = persistence::load_inventory(db, h.looter.character_id);
    for index in 0..free {
        inventory.items.push(InventoryItemData {
            item_guid: 900_000 + index as u64,
            item_id: TARNISHED_CHAIN_VEST,
            name: "Tarnished Chain Vest".into(),
            quality: 1,
            required_level: 1,
            stack_count: 1,
            vendor_sell_price: 17,
            unique_auction: false,
        });
    }
    persistence::save_inventory(db, h.looter.character_id, &inventory);

    h.open(kobold, true);

    assert_eq!(h.failures(), vec![LootError::InventoryFull]);
    assert_eq!(h.count_of(MELTED_CANDLE), 0);
    let removed: Vec<u8> = h.take::<LootSlotRemoved>().iter().map(|r| r.slot).collect();
    assert_eq!(removed, [0], "the coins still fit");
    assert!(h.take::<LootClosed>().is_empty());
    assert_eq!(
        h.app.world().get::<ActiveLoot>(h.looter.connection),
        Some(&ActiveLoot { corpse: kobold })
    );
}

#[test]
fn a_player_who_did_not_kill_it_may_not_loot() {
    let Some(mut h) = harness(true) else { return };
    let kobold = h.kill_kobold();
    let stranger = spawn_looter(&mut h.app, "Stranger", 2);
    h.settle();

    h.send::<_, LootChannel>(
        stranger,
        LootUnit {
            corpse: kobold.to_bits(),
            auto: false,
        },
    );

    assert_eq!(h.failures(), vec![LootError::NoPermission]);
    assert!(h.take::<LootResponse>().is_empty());
}

#[test]
fn a_corpse_out_of_reach_is_refused_and_walking_away_closes_the_window() {
    let Some(mut h) = harness(true) else { return };
    let kobold = h.kill_kobold();
    h.stock(kobold);
    h.app
        .world_mut()
        .get_mut::<Position>(h.looter.player)
        .unwrap()
        .x = 20.0;
    h.open(kobold, false);
    assert_eq!(h.failures(), vec![LootError::TooFar]);

    h.app
        .world_mut()
        .get_mut::<Position>(h.looter.player)
        .unwrap()
        .x = 0.0;
    h.open(kobold, false);
    assert_eq!(h.take::<LootResponse>().len(), 1);
    h.app
        .world_mut()
        .get_mut::<Position>(h.looter.player)
        .unwrap()
        .x = 20.0;
    h.settle();
    assert_eq!(h.take::<LootClosed>().len(), 1);
    assert!(
        h.app
            .world()
            .get::<ActiveLoot>(h.looter.connection)
            .is_none()
    );
}

#[test]
fn a_corpse_nobody_gets_loot_from_decays_in_half_the_time() {
    let Some(mut h) = harness(false) else { return };
    let kobold = h.kill_kobold();

    assert!(h.take::<CorpseLootable>().is_empty());
    assert_eq!(h.corpse(kobold), Corpse { remaining: 30.0 });
    h.open(kobold, false);
    assert_eq!(h.failures(), vec![LootError::NoPermission]);
}

#[test]
fn a_decayed_corpse_stops_being_lootable_for_its_looters() {
    let Some(mut h) = harness(true) else { return };
    let kobold = h.kill_kobold();
    assert_eq!(h.take::<CorpseLootable>(), vec![lootable(kobold, true)]);

    // `combat::corpse_decay_system` removes `Corpse` when the decay runs out.
    h.app.world_mut().entity_mut(kobold).remove::<Corpse>();
    h.settle();

    assert_eq!(h.take::<CorpseLootable>(), vec![lootable(kobold, false)]);
    assert!(h.app.world().get::<CorpseLoot>(kobold).is_none());
    h.open(kobold, false);
    assert_eq!(h.failures(), vec![LootError::NoPermission]);
}

#[test]
fn a_poor_quality_drop_becomes_coins_at_its_sell_price() {
    use rand::SeedableRng;

    let Some(path) = world_db_path() else { return };
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tables = load_loot_tables(&conn).unwrap();
    let catalog = load_vendor_catalog(&conn).unwrap();
    let templates = load_item_templates(&path).unwrap();
    // Kobold Vermin drops the grey Melted Candle (sell price 1 copper) 20 % of kills.
    let mut rng = rand::rngs::StdRng::seed_from_u64(7);
    let rolled = std::iter::repeat_with(|| {
        tables.roll_creature(KOBOLD_VERMIN, 0, &templates, |_| false, &mut rng)
    })
    .find(|rolled| {
        rolled
            .items
            .iter()
            .any(|drop| drop.item_id == MELTED_CANDLE)
    })
    .unwrap();
    let candles: u64 = rolled
        .items
        .iter()
        .filter(|drop| drop.item_id == MELTED_CANDLE)
        .map(|drop| u64::from(drop.count))
        .sum();
    assert_eq!(catalog.goods[&MELTED_CANDLE].sell_price, 1);

    let slots = loot_slots(rolled.clone(), &catalog);

    assert_eq!(
        slots[0],
        Some(LootContent::Money {
            copper: rolled.copper + candles
        })
    );
    assert!(
        slots.iter().flatten().all(|content| match content {
            LootContent::Item { quality, .. } => *quality > 0,
            LootContent::Money { .. } => true,
        }),
        "no grey item left: {slots:?}"
    );
}

#[test]
fn source_requirements_loot_unique_limit_counts_retail_only() {
    use crate::persistence::inventory_store::{load_inventory_record, save_inventory_record};
    use shared::item_data::ItemDefinitionSource::Forever70205;
    for id in [2947, 2512, 2101] {
        let mut h = harness(false).expect("world loot fixture required");
        let plain = crate::item_templates::ItemTemplate {
            max_stack: 200,
            allowable_class: -1,
            allowable_race: u64::MAX,
            ..Default::default()
        };
        {
            let mut templates = h.app.world_mut().resource_mut::<ItemTemplates>();
            templates.retail.insert(id, plain);
            templates.forever.insert(id, plain);
        }
        h.app
            .world_mut()
            .resource_mut::<VendorCatalog>()
            .goods
            .insert(
                id,
                crate::merchant::VendorGoods {
                    name: format!("Retail {id}"),
                    quality: 1,
                    max_count: 1,
                    ..Default::default()
                },
            );
        let db = &h.app.world().resource::<GameDatabase>().db;
        let mut record = load_inventory_record(db, h.looter.character_id).unwrap();
        record.inventory.items.clear();
        record.inventory.items.push(InventoryItemData {
            item_guid: 1101,
            item_id: id,
            name: format!("Forever {id}"),
            quality: 1,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 1,
            unique_auction: false,
        });
        record.definition_sources.insert(1101, Forever70205);
        save_inventory_record(db, h.looter.character_id, &record);
        let corpse = h.kill_kobold();
        let slots = vec![
            Some(LootContent::Item {
                item_id: id,
                name: format!("Retail {id}"),
                quality: 1,
                count: 1
            });
            2
        ];
        h.app.world_mut().entity_mut(corpse).insert(CorpseLoot {
            looters: [(h.looter.character_id, slots)].into(),
        });
        h.open(corpse, false);
        h.take_slot(corpse, 0);
        assert!(
            h.failures().is_empty(),
            "Forever {id} must not block Retail loot"
        );
        assert_eq!(h.count_of(id), 2);
        let before = h.inventory();
        h.take_slot(corpse, 1);
        assert_eq!(h.failures(), vec![LootError::CantCarryMore]);
        assert_eq!(h.inventory(), before);
        assert!(
            h.app.world().get::<CorpseLoot>(corpse).unwrap().looters[&h.looter.character_id][1]
                .is_some()
        );
    }
}

#[test]
fn achievements_criteria_loot_item_counts_only_committed_loot_slots() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let mut h = harness_with_plugins(false, |app| {
        app.add_plugins(AchievementPlugin);
    })
    .unwrap();
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    h.app.world_mut().entity_mut(h.looter.player).insert((
        PlayerAchievementState::default(),
        shared::components::Level(30),
    ));
    h.app
        .world_mut()
        .entity_mut(h.looter.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    h.settle();
    let corpse = h.kill_kobold();
    h.app.world_mut().entity_mut(corpse).insert(CorpseLoot {
        looters: [(
            h.looter.character_id,
            vec![Some(LootContent::Item {
                item_id: 6308,
                name: "Raw Bristle Whisker Catfish".into(),
                quality: 1,
                count: 3,
            })],
        )]
        .into(),
    });
    h.take_slot(corpse, 0);
    assert_eq!(h.count_of(6308), 0);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.looter.player)
            .unwrap()
            .criteria_progress
            .contains_key(&15886)
    );
    h.open(corpse, false);
    h.take_slot(corpse, 0);
    assert_eq!(h.count_of(6308), 3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.looter.player)
            .unwrap()
            .criteria_progress
            .get(&15886),
        Some(&3)
    );
    h.take_slot(corpse, 0);
    h.settle();
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.looter.player)
            .unwrap()
            .criteria_progress
            .get(&15886),
        Some(&3)
    );
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_achievements(db, h.looter.character_id)
            .criteria_progress
            .get(&15886),
        Some(&3)
    );
    // Corpse loot is LOOT_CORPSE, never the disenchanting GetLootByType criterion5304.
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.looter.player)
            .unwrap()
            .criteria_progress
            .contains_key(&5304)
    );
}

#[test]
#[ignore = "aura-world native RED/GREEN gate"]
fn aura_world_loot_city_of_gold_not_quest_enlisted_money() {
    for (spell, expected) in [(291619, 1020), (269083, 1000)] {
        let mut h = harness(false).unwrap();
        let auras = crate::auras::world_xp_tests::native_auras(spell);
        h.app.world_mut().entity_mut(h.looter.player).insert(auras);
        let corpse = h.kill_kobold();
        h.app.world_mut().entity_mut(corpse).insert(CorpseLoot {
            looters: [(
                h.looter.character_id,
                vec![Some(LootContent::Money { copper: 1000 })],
            )]
            .into(),
        });
        let before = h.inventory().gold;
        h.open(corpse, false);
        h.take_slot(corpse, 0);
        assert_eq!(
            h.inventory().gold - before,
            expected,
            "native{spell} loot Misc1 only"
        );
        h.take_slot(corpse, 0);
        assert_eq!(h.inventory().gold - before, expected, "no replay");
    }
}
