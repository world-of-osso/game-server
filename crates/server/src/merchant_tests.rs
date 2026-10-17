//! Real-DB tests against data/world.db.

#[path = "merchant_economy_tests.rs"]
mod economic;
#[path = "merchant_guild_repair_tests.rs"]
mod guild_repair;
#[path = "merchant_mount_cost_tests.rs"]
mod mount_costs;
#[path = "owned_commerce_buyback_tests.rs"]
mod owned_commerce_sources;

use std::path::PathBuf;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Channel, Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::components::{CharacterAppearance, Health, Position, UnitFactionTemplate};
use shared::protocol::{
    InteractNpc, InteractionChannel, InteractionKind, InteractionOpened, ItemLocation,
    ProtocolPlugin, SellAllJunkItems,
};

use super::stock::{BuybackEntry, BuybackSlots, VendorStock};
use super::*;
use crate::durability::{DurabilitySlotState, EquippedDurability, load_item_durability};
use crate::faction::world_db_faction_templates;
use crate::item_templates::load_item_templates;
use crate::npc_interaction::{NpcInteractionPlugin, load_gossip_catalog};
use crate::persistence::ClientId;

const HUMAN_TEMPLATE: u32 = 1;
/// Godric Rothgar, Northshire armorer (npcflag vendor | repair, 8 items).
const GODRIC_ROTHGAR: u32 = 1213;
/// Brother Danil, Northshire general goods (npcflag vendor | vendor food).
const BROTHER_DANIL: u32 = 152;
/// Tharynn Bouden, Goldshire trade supplies (19 items; one limited pattern).
const THARYNN_BOUDEN: u32 = 66;

const LARGE_ROUND_SHIELD: u32 = 2129;
const TARNISHED_CHAIN_VEST: u32 = 2379;
/// Godric Rothgar's list slots (TrinityCore order).
const VEST_SLOT: u32 = 0;
const SHIELD_SLOT: u32 = 7;
const SPRING_WATER: u32 = 159;
const LINEN: u32 = 2589;
/// Poor-quality drops: Ruined Pelt 5c, Broken Fang 6c, Chipped Claw 4c.
const RUINED_PELT: u32 = 4865;
const BROKEN_FANG: u32 = 7073;
const CHIPPED_CLAW: u32 = 7074;
/// Pattern: Blue Linen Vest: maxcount 1, incrtime 9000 at Tharynn Bouden.
const BLUE_LINEN_VEST_PATTERN: u32 = 6270;

#[test]
fn guild_news_records_only_successful_source_qualified_vendor_purchases() {
    let mut h = harness().expect("authentic vendor database");
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    h.app.world_mut().entity_mut(h.player).insert(Level(50));
    let mut reputation = shared::reputation::CharacterReputation::default();
    reputation.set_rep(
        1119,
        shared::reputation::threshold_for_standing(shared::reputation::Standing::Exalted),
    );
    h.app.world_mut().entity_mut(h.player).insert(reputation);
    crate::persistence_character::set_character_guild_by_id(
        &h.app.world().resource::<GameDatabase>().db,
        h.character_id,
        7,
        "Real News",
    )
    .unwrap();
    // Lillehoff's installed slot: Greater Inscription of the Axe, quality7, ilvl30.
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(pair::<shared::protocol::SelectGossipOption>());
    let vendor = h.open(32540);
    let opened = h.take::<InteractionOpened>();
    assert_eq!(opened.len(), 1, "Actual Lillehoff interaction: {opened:?}");
    let InteractionKind::Gossip(menu) = &opened[0].kind else {
        panic!("Lillehoff's real gossip menu expected: {opened:?}")
    };
    let vendor_option = menu
        .options
        .iter()
        .find(|option| option.icon == 1)
        .expect("authored vendor gossip option")
        .option_id;
    h.send::<_, InteractionChannel>(shared::protocol::SelectGossipOption {
        npc: vendor.to_bits(),
        option_id: vendor_option,
    });
    let inventories = h.take::<VendorInventory>();
    assert_eq!(inventories.len(), 1);
    let item = inventories[0]
        .items
        .iter()
        .find(|item| item.item_id == 50335)
        .unwrap();
    let slot = item.slot;
    h.set_gold(0);
    h.buy(vendor, slot, 50335, 1);
    assert_eq!(h.take::<MerchantFailed>().len(), 1);
    assert!(
        crate::guild_news::query(&h.app.world().resource::<GameDatabase>().db, h.character_id)
            .entries
            .is_empty()
    );
    h.set_gold(1_000_000);
    h.buy(vendor, slot, 50335, 1);
    assert!(h.take::<MerchantFailed>().is_empty());
    let inventory =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id);
    assert!(inventory.items.iter().any(|item| item.item_id == 50335));
    let news =
        crate::guild_news::query(&h.app.world().resource::<GameDatabase>().db, h.character_id);
    assert_eq!(
        news.entries.len(),
        1,
        "Real committed source-qualified purchase must publish news"
    );
    assert_eq!(
        news.entries[0].kind,
        shared::protocol::GuildNewsKind::ItemPurchased {
            member: "Shopper".into(),
            item_id: 50335
        }
    );
    // Godric's real ordinary-quality shield is a purchase, not guild-news loot.
    let ordinary = h.open(GODRIC_ROTHGAR);
    h.take::<VendorInventory>();
    h.set_gold(100_000);
    h.buy(ordinary, SHIELD_SLOT, LARGE_ROUND_SHIELD, 1);
    assert!(h.take::<MerchantFailed>().is_empty());
    assert_eq!(
        crate::guild_news::query(&h.app.world().resource::<GameDatabase>().db, h.character_id)
            .entries,
        news.entries
    );
}

#[test]
fn achievements_criteria_acquire_item_credits_only_committed_retail_purchase() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let mut h = harness_with(|app| {
        app.add_plugins(AchievementPlugin);
    })
    .expect("authentic vendor database");
    h.app
        .insert_resource(load_achievement_catalog(&h.conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(PlayerAchievementState::default());
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    h.app.world_mut().entity_mut(h.player).insert(Level(10));
    let vendor = h.open(17904);
    let inventory = h.take::<VendorInventory>();
    assert_eq!(inventory.len(), 1);
    let item = inventory[0]
        .items
        .iter()
        .find(|item| item.item_id == 33999)
        .unwrap();
    let slot = item.slot;
    h.buy(vendor, slot, 33999, 1);
    assert_eq!(h.take::<MerchantFailed>().len(), 1);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .criteria_progress
            .contains_key(&1874)
    );
    // Purchase money and the authored Exalted Cenarion Expedition942 gate (item_sparse
    // MinFactionID/MinReputation) are setup; the merchant transaction is the acquisition.
    h.set_gold(20_000_000);
    let mut reputation = shared::reputation::CharacterReputation::default();
    reputation.set_rep(
        942,
        shared::reputation::threshold_for_standing(shared::reputation::Standing::Exalted),
    );
    h.app.world_mut().entity_mut(h.player).insert(reputation);
    h.buy(vendor, slot, 33999, 1);
    assert!(h.take::<MerchantFailed>().is_empty());
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .criteria_progress
            .get(&1874),
        Some(&1)
    );
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .criteria_progress
            .get(&6142),
        Some(&1),
        "native committed epic purchase must credit ObtainAnyItem91 behind ItemQualityExactly15/4"
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .criteria_progress
            .contains_key(&6140),
        "a purchase is not LootAnyItem90"
    );
    // Tree5192 is MeetsAtLeast8 amount1 (TC IsCompletedCriteriaTree): acquiring 33999
    // or knowing 43927 completes Cenarion War Hippogryph893.
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .account
            .as_deref()
            .unwrap()
            .earned_ids
            .contains(&893)
    );
    h.app.world_mut().entity_mut(h.player).insert(Level(30));
    h.set_gold(85_000_000);
    let ring_vendor = h.open(32172);
    let ring_inventory = h.take::<VendorInventory>();
    let ring_slot = ring_inventory[0]
        .items
        .iter()
        .find(|item| item.item_id == 40586)
        .unwrap()
        .slot;
    h.buy(ring_vendor, ring_slot, 40586, 1);
    assert!(h.take::<MerchantFailed>().is_empty());
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .criteria_progress
            .get(&7401),
        Some(&1)
    );
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .earned_ids
            .contains(&2084)
    );
    assert_eq!(
        persistence::load_achievements(
            &h.app.world().resource::<GameDatabase>().db,
            h.character_id
        )
        .criteria_progress
        .get(&1874),
        Some(&1)
    );
    for _ in 0..3 {
        h.app.update();
    }
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .criteria_progress
            .get(&1874),
        Some(&1)
    );
}

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
    conn: Connection,
    character_id: u64,
    player: Entity,
    connection: Entity,
}

fn harness() -> Option<Harness> {
    harness_with(|_| {})
}

fn harness_with(add_plugins: impl FnOnce(&mut App)) -> Option<Harness> {
    let path = world_db_path()?;
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    add_inbox::<InteractionOpened>(&mut app);
    add_inbox::<VendorInventory>(&mut app);
    add_inbox::<BuybackList>(&mut app);
    add_inbox::<MerchantFailed>(&mut app);
    app.insert_resource(world_db_faction_templates()?);
    app.insert_resource(load_gossip_catalog(&conn).unwrap());
    app.insert_resource(load_item_templates(&path).unwrap());
    crate::appearance_collection::tests::install_database(&mut app);
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins((NpcInteractionPlugin, MerchantPlugin));
    app.insert_resource(load_vendor_catalog(&conn).unwrap());
    app.insert_resource(load_item_durability(&conn).unwrap());
    add_plugins(&mut app);
    app.finish();
    app.cleanup();
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) = persistence::create_character_with_test_bags(db, 1, "Shopper", 1, 1)
        .expect("create character");
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Shopper".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(1),
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
            UnitFactionTemplate(HUMAN_TEMPLATE),
            crate::maps::InMap::continent(0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            InWorld { character_id },
            PlayerLink(player),
            (
                pair::<InteractNpc>(),
                pair::<InteractionOpened>(),
                pair::<VendorInventory>(),
                pair::<BuybackList>(),
                pair::<MerchantFailed>(),
                pair::<BuyItem>(),
                pair::<SellItem>(),
                pair::<SellAllJunkItems>(),
                pair::<BuybackItemRequest>(),
                pair::<RepairItem>(),
            ),
        ))
        .id();
    app.update();
    Some(Harness {
        app,
        conn,
        character_id,
        player,
        connection,
    })
}

impl Harness {
    /// Spawn a vendor from its world.db template and stand the player 2 yd from it.
    fn spawn_vendor(&mut self, entry: u32) -> Entity {
        let (name, faction, npcflag): (String, u32, u32) = self
            .conn
            .query_row(
                "SELECT name, faction, npcflag FROM content_creature_template WHERE entry = ?1",
                [entry],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        self.app
            .world_mut()
            .spawn((
                Npc {
                    template_id: entry,
                    name,
                },
                NpcFlags::from_azerothcore(npcflag),
                Position {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                UnitFactionTemplate(faction),
                crate::maps::InMap::continent(0),
            ))
            .id()
    }

    fn send<M: lightyear::prelude::Message, C: Channel>(&mut self, message: M) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<M>>(self.connection)
            .unwrap()
            .send::<C>(message);
        for _ in 0..3 {
            self.app.update();
        }
    }

    fn open(&mut self, entry: u32) -> Entity {
        let npc = self.spawn_vendor(entry);
        self.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
        npc
    }

    fn buy(&mut self, npc: Entity, slot: u32, item_id: u32, count: u32) {
        self.send::<_, MerchantChannel>(BuyItem {
            npc: npc.to_bits(),
            slot,
            item_id,
            count,
            destination: None,
        });
    }

    fn buy_into(&mut self, npc: Entity, slot: u32, item_id: u32, destination: ItemLocation) {
        self.send::<_, MerchantChannel>(BuyItem {
            npc: npc.to_bits(),
            slot,
            item_id,
            count: 1,
            destination: Some(destination),
        });
    }

    fn grant(&mut self, item_id: u32, count: u32) {
        let world = self.app.world();
        crate::inventory::grant_item(
            &world.resource::<GameDatabase>().db,
            world.resource::<ItemTemplates>(),
            world.resource::<VendorCatalog>(),
            self.character_id,
            (item_id, count),
        )
        .expect("grant");
    }

    /// Item id and count at each occupied bag location.
    fn bag_view(&self) -> Vec<(ItemLocation, u32, u32)> {
        let world = self.app.world();
        crate::inventory::load_model(
            &world.resource::<GameDatabase>().db,
            world.resource::<ItemTemplates>(),
            self.character_id,
        )
        .unwrap()
        .view()
        .into_iter()
        .map(|(location, stack)| (location, stack.item_id, stack.count))
        .collect()
    }

    fn sell(&mut self, npc: Entity, item_guid: u64, count: u32) {
        self.send::<_, MerchantChannel>(SellItem {
            npc: npc.to_bits(),
            item_guid,
            count,
        });
    }

    fn take<M: lightyear::prelude::Message>(&mut self) -> Vec<M> {
        std::mem::take(&mut self.app.world_mut().resource_mut::<Inbox<M>>().0)
    }

    fn inventory(&self) -> persistence::InventoryData {
        persistence::load_inventory(
            &self.app.world().resource::<GameDatabase>().db,
            self.character_id,
        )
    }

    fn set_gold(&mut self, gold: u64) {
        let db = &self.app.world().resource::<GameDatabase>().db;
        let mut inventory = persistence::load_inventory(db, self.character_id);
        inventory.gold = gold;
        persistence::save_inventory(db, self.character_id, &inventory);
    }

    fn count_of(&self, item_id: u32) -> Vec<u32> {
        self.inventory()
            .items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| item.stack_count)
            .collect()
    }

    fn guid_of(&self, item_id: u32) -> u64 {
        self.inventory()
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .expect("item in bags")
            .item_guid
    }

    fn failures(&mut self) -> Vec<MerchantError> {
        self.take::<MerchantFailed>()
            .into_iter()
            .map(|failed| failed.error)
            .collect()
    }
}

#[test]
fn mount_journal_authentic_vendor_purchase_then_learn_preserves_appearance_collection() {
    let mut h = harness().expect("authentic Katie Hunter and mount data");
    let catalog = crate::mount_journal::load(&h.conn).unwrap();
    h.app.insert_resource(catalog);
    h.app.world_mut().entity_mut(h.player).insert(Level(60));
    h.app.world_mut().entity_mut(h.connection).insert((
        crate::auth::Authenticated { account_id: 1 },
        pair::<shared::protocol::SelectGossipOption>(),
    ));
    h.set_gold(10_000);
    let npc = h.open(384);
    let opening = h.take::<InteractionOpened>().pop().unwrap();
    if let InteractionKind::Gossip(menu) = opening.kind {
        let option_id = menu
            .options
            .iter()
            .find(|option| option.icon == 1)
            .expect("authored vendor role")
            .option_id;
        h.send::<_, InteractionChannel>(shared::protocol::SelectGossipOption {
            npc: npc.to_bits(),
            option_id,
        });
    }
    let offers = h
        .take::<VendorInventory>()
        .pop()
        .expect("authentic vendor inventory");
    let mount = offers
        .items
        .iter()
        .find(|item| item.item_id == 5655)
        .expect("Chestnut Mare Bridle");
    assert_eq!(mount.price, 10_000);
    let slot = mount.slot;
    let db = &h.app.world().resource::<GameDatabase>().db;
    let appearances = crate::persistence::appearance_store::load_account_appearances(db, 1);
    h.buy(npc, slot, 5655, 1);
    assert!(h.failures().is_empty());
    assert_eq!(h.inventory().gold, 0);
    assert_eq!(h.count_of(5655), vec![1]);
    let location = h
        .bag_view()
        .into_iter()
        .find(|(_, id, _)| *id == 5655)
        .unwrap()
        .0;
    assert!(
        crate::mount_journal::learn_item(h.app.world_mut(), h.character_id, h.player, location)
            .unwrap()
    );
    assert!(h.count_of(5655).is_empty());
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert_eq!(
        crate::persistence::mount_store::load(db, 1).get(&18),
        Some(&false)
    );
    assert_eq!(
        crate::persistence::appearance_store::load_account_appearances(db, 1),
        appearances
    );
    h.buy(npc, slot, 5655, 1);
    assert!(
        !h.failures().is_empty(),
        "Unaffordable mount purchase refuses"
    );
    assert!(h.count_of(5655).is_empty());
    assert_eq!(h.inventory().gold, 0);
}

/// Tarnished Chain Vest 44/55 and Large Round Shield 25/35 (0.8 copper a point).
fn damage_gear(h: &mut Harness) {
    let slot = |slot, item_id, current, max| DurabilitySlotState {
        slot,
        item_id,
        current,
        max,
    };
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(EquippedDurability {
            slots: vec![
                slot(EquipmentVisualSlot::Chest, TARNISHED_CHAIN_VEST, 44, 55),
                slot(EquipmentVisualSlot::OffHand, LARGE_ROUND_SHIELD, 25, 35),
            ],
        });
}

fn durability(h: &Harness) -> Vec<(u32, u32)> {
    h.app
        .world()
        .get::<EquippedDurability>(h.player)
        .unwrap()
        .slots
        .iter()
        .map(|entry| (entry.current, entry.max))
        .collect()
}

fn names(inventory: &VendorInventory) -> Vec<(u32, &str, u32)> {
    inventory
        .items
        .iter()
        .map(|item| (item.slot, item.name.as_str(), item.price))
        .collect()
}

#[test]
fn opening_godric_rothgar_sends_his_armor_list_and_an_empty_buyback() {
    let Some(mut h) = harness() else { return };
    let godric = h.open(GODRIC_ROTHGAR);

    assert_eq!(
        h.take::<InteractionOpened>(),
        vec![InteractionOpened {
            npc: godric.to_bits(),
            kind: InteractionKind::Role(NpcRole::Vendor),
        }]
    );
    let lists = h.take::<VendorInventory>();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].npc, godric.to_bits());
    assert!(lists[0].can_repair);
    assert_eq!(
        names(&lists[0]),
        // TrinityCore's Retail list order (npc_vendor.slot).
        vec![
            (0, "Tarnished Chain Vest", 89),
            (1, "Tarnished Chain Leggings", 90),
            (2, "Tarnished Chain Belt", 45),
            (3, "Tarnished Chain Boots", 67),
            (4, "Tarnished Chain Bracers", 45),
            (5, "Tarnished Chain Gloves", 45),
            (6, "Small Shield", 36),
            (7, "Large Round Shield", 78),
        ]
    );
    let vest = &lists[0].items[0];
    assert_eq!(
        (vest.stack_count, vest.num_available, vest.usable),
        (1, None, true)
    );
    assert_eq!(h.take::<BuybackList>(), vec![BuybackList::default()]);
}

#[test]
fn buying_the_tarnished_chain_vest_costs_89_copper_and_lands_in_the_backpack() {
    let Some(mut h) = harness() else { return };
    let godric = h.open(GODRIC_ROTHGAR);
    assert_eq!(h.inventory().gold, 100_000);

    h.buy(godric, VEST_SLOT, TARNISHED_CHAIN_VEST, 1);

    assert!(h.failures().is_empty());
    assert_eq!(h.inventory().gold, 99_911);
    assert_eq!(h.count_of(TARNISHED_CHAIN_VEST), vec![1]);
    let gold = h
        .app
        .world()
        .resource::<GoldUpdateQueue>()
        .changed_inventories()
        .clone();
    assert!(gold.contains(&h.character_id));
}

#[test]
fn buying_is_refused_without_the_vendor_open_or_the_money() {
    let Some(mut h) = harness() else { return };
    let godric = h.spawn_vendor(GODRIC_ROTHGAR);
    h.buy(godric, VEST_SLOT, TARNISHED_CHAIN_VEST, 1);
    assert_eq!(h.failures(), vec![MerchantError::NotInteracting]);

    h.send::<_, InteractionChannel>(InteractNpc {
        npc: godric.to_bits(),
    });
    h.set_gold(88);
    h.buy(godric, VEST_SLOT, TARNISHED_CHAIN_VEST, 1);
    assert_eq!(h.failures(), vec![MerchantError::NotEnoughMoney]);
    // The vest's slot does not hold the shield.
    h.buy(godric, VEST_SLOT, LARGE_ROUND_SHIELD, 1);
    assert_eq!(h.failures(), vec![MerchantError::ItemNotFound]);
    assert_eq!(h.inventory().gold, 88);
    assert!(h.count_of(TARNISHED_CHAIN_VEST).is_empty());
}

#[test]
fn a_purchase_that_does_not_fit_the_backpack_changes_nothing() {
    let Some(mut h) = harness() else { return };
    let godric = h.open(GODRIC_ROTHGAR);
    // 16 backpack slots, 3 hold the starter stacks.
    h.buy(godric, SHIELD_SLOT, LARGE_ROUND_SHIELD, 14);

    assert_eq!(h.failures(), vec![MerchantError::InventoryFull]);
    assert_eq!(h.inventory().gold, 100_000);
    assert!(h.count_of(LARGE_ROUND_SHIELD).is_empty());

    h.buy(godric, SHIELD_SLOT, LARGE_ROUND_SHIELD, 13);
    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(LARGE_ROUND_SHIELD), vec![1; 13]);
    assert_eq!(h.inventory().gold, 100_000 - 13 * 78);
}

#[test]
fn brother_danil_water_comes_five_per_purchase_and_stacks_to_twenty() {
    let Some(mut h) = harness() else { return };
    let danil = h.open(BROTHER_DANIL);
    let list = h.take::<VendorInventory>().remove(0);
    let water = list
        .items
        .iter()
        .find(|item| item.item_id == SPRING_WATER)
        .unwrap()
        .clone();
    assert_eq!(
        (water.price, water.stack_count, water.max_stack),
        (5, 5, 20)
    );

    h.buy(danil, water.slot, SPRING_WATER, 3);
    h.buy(danil, water.slot, SPRING_WATER, 2);

    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(SPRING_WATER), vec![20, 5]);
    assert_eq!(h.inventory().gold, 100_000 - 25);
    // Water has no sell price.
    let guid = h.guid_of(SPRING_WATER);
    h.sell(danil, guid, 0);
    assert_eq!(h.failures(), vec![MerchantError::NotInterested]);
    assert_eq!(h.count_of(SPRING_WATER), vec![20, 5]);
}

#[test]
fn transmogacq_receiving_a_bop_grant_collects_without_waiting_for_a_frame() {
    let mut h = harness().unwrap();
    h.grant(6953, 1);
    assert_eq!(h.count_of(6953), vec![1]);
    assert!(
        crate::persistence::appearance_store::load_account_appearances(
            &h.app.world().resource::<GameDatabase>().db,
            1
        )
        .has(2288)
    );
    crate::appearance_collection::tests::assert_restart(h.app, 1, &[(2288, true)]);
}

#[test]
fn transmogacq_buying_bop_collects_its_appearance() {
    let mut h = harness().unwrap();
    h.set_gold(100_000);
    // Wulmort Jinglepocket sells the BoP Gaudy Winter Veil Sweater.
    let entry = 13433;
    let slot = h
        .app
        .world()
        .resource::<VendorCatalog>()
        .items(entry)
        .iter()
        .position(|item| item.item_id == 70923)
        .unwrap() as u32;
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(pair::<shared::protocol::SelectGossipOption>());
    let npc = h.open(entry);
    let opening = h
        .take::<InteractionOpened>()
        .pop()
        .expect("vendor interaction opened");
    if let InteractionKind::Gossip(menu) = opening.kind {
        let option_id = menu
            .options
            .iter()
            .find(|option| option.icon == 1)
            .expect("vendor gossip option")
            .option_id;
        h.send::<_, InteractionChannel>(shared::protocol::SelectGossipOption {
            npc: npc.to_bits(),
            option_id,
        });
    }
    h.buy(npc, slot, 70923, 1);
    let errors = h.take::<MerchantFailed>();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(h.count_of(70923), vec![1]);
    assert!(
        crate::persistence::appearance_store::load_account_appearances(
            &h.app.world().resource::<GameDatabase>().db,
            1
        )
        .has(15791)
    );
    crate::appearance_collection::tests::assert_restart(h.app, 1, &[(15791, true)]);
}

#[test]
fn transmogacq_buying_an_unbound_boe_does_not_collect() {
    let mut h = harness().unwrap();
    h.set_gold(100_000);
    let npc = h.open(GODRIC_ROTHGAR);
    h.buy(npc, SHIELD_SLOT, LARGE_ROUND_SHIELD, 1);
    assert!(h.take::<MerchantFailed>().is_empty());
    assert_eq!(h.count_of(LARGE_ROUND_SHIELD), vec![1]);
    assert!(
        !crate::persistence::appearance_store::load_account_appearances(
            &h.app.world().resource::<GameDatabase>().db,
            1
        )
        .has(684)
    );
    crate::appearance_collection::tests::assert_restart(h.app, 1, &[(684, false)]);
}

#[test]
fn transmogacq_selling_an_unbound_boe_collects_its_appearance() {
    let mut h = harness().unwrap();
    h.grant(25, 1);
    let npc = h.open(GODRIC_ROTHGAR);
    let guid = h
        .inventory()
        .items
        .iter()
        .find(|item| item.item_id == 25)
        .unwrap()
        .item_guid;
    assert!(
        !crate::persistence::appearance_store::load_account_appearances(
            &h.app.world().resource::<GameDatabase>().db,
            1
        )
        .has(154)
    );
    h.sell(npc, guid, 0);
    assert!(h.inventory().items.iter().all(|item| item.item_id != 25));
    assert!(
        crate::persistence::appearance_store::load_account_appearances(
            &h.app.world().resource::<GameDatabase>().db,
            1
        )
        .has(154)
    );
    crate::appearance_collection::tests::assert_restart(h.app, 1, &[(154, true)]);
}

#[test]
fn selling_linen_pays_its_sell_price_and_buyback_returns_it() {
    let Some(mut h) = harness() else { return };
    let godric = h.open(GODRIC_ROTHGAR);
    h.take::<BuybackList>();
    let linen = h.guid_of(LINEN);

    h.sell(godric, linen, 5);

    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(LINEN), vec![15]);
    assert_eq!(h.inventory().gold, 100_000 + 5 * 13);
    let lists = h.take::<BuybackList>();
    assert_eq!(
        lists,
        vec![BuybackList {
            items: vec![BuybackItem {
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
                slot: 0,
                item_id: LINEN,
                name: "Linen Cloth".into(),
                quality: 1,
                count: 5,
                price: 65,
            }],
        }]
    );

    h.send::<_, MerchantChannel>(BuybackItemRequest {
        npc: godric.to_bits(),
        slot: 0,
    });

    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(LINEN), vec![20]);
    assert_eq!(h.inventory().gold, 100_000);
    assert_eq!(h.take::<BuybackList>(), vec![BuybackList::default()]);
    h.send::<_, MerchantChannel>(BuybackItemRequest {
        npc: godric.to_bits(),
        slot: 0,
    });
    assert_eq!(h.failures(), vec![MerchantError::ItemNotFound]);
}

#[test]
fn tharynn_boudens_limited_pattern_sells_out_after_one() {
    let Some(mut h) = harness() else { return };
    let tharynn = h.open(THARYNN_BOUDEN);
    let list = h.take::<VendorInventory>().remove(0);
    // TrinityCore's Retail list.
    assert_eq!(list.items.len(), 22);
    let pattern = list
        .items
        .iter()
        .find(|item| item.item_id == BLUE_LINEN_VEST_PATTERN)
        .unwrap()
        .clone();
    assert_eq!(pattern.num_available, Some(1));

    h.buy(tharynn, pattern.slot, BLUE_LINEN_VEST_PATTERN, 1);

    assert!(h.failures().is_empty());
    let refreshed = h.take::<VendorInventory>();
    assert_eq!(refreshed.len(), 1);
    assert_eq!(refreshed[0].items.len(), 21);
    assert!(
        refreshed[0]
            .items
            .iter()
            .all(|item| item.item_id != BLUE_LINEN_VEST_PATTERN)
    );
    h.buy(tharynn, pattern.slot, BLUE_LINEN_VEST_PATTERN, 1);
    assert_eq!(h.failures(), vec![MerchantError::SoldOut]);
    assert_eq!(h.count_of(BLUE_LINEN_VEST_PATTERN), vec![1]);
}

#[test]
fn limited_stock_restocks_one_purchase_per_incrtime() {
    let slot = VendorSlot {
        item_id: 2319,
        maxcount: 3,
        incrtime: 900,
    };
    let vendor = Entity::from_bits(42);
    let mut stock = VendorStock::default();
    assert_eq!(stock.available(vendor, 1, slot, 1, 0.0), Some(3));
    stock.consume(vendor, 1, slot, 1, 3, 10.0);
    assert_eq!(stock.available(vendor, 1, slot, 1, 909.0), Some(0));
    assert_eq!(stock.available(vendor, 1, slot, 1, 910.0), Some(1));
    assert_eq!(stock.available(vendor, 1, slot, 1, 2710.0), Some(3));
}

fn sale(item_guid: u64, sold_at: f64) -> BuybackEntry {
    BuybackEntry {
        item: InventoryItemData {
            item_guid,
            item_id: LINEN,
            name: "Linen Cloth".into(),
            quality: 1,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 13,
            unique_auction: false,
        },
        price: 13,
        sold_at,
    }
}

#[test]
fn a_thirteenth_sale_replaces_the_oldest_buyback() {
    let mut slots = BuybackSlots::default();
    for guid in 0..13 {
        slots.add(sale(guid, guid as f64));
    }
    let guids: Vec<(u8, u64)> = slots
        .entries()
        .map(|(slot, entry)| (slot, entry.item.item_guid))
        .collect();
    let mut expected: Vec<(u8, u64)> = (1..12).map(|slot| (slot, u64::from(slot))).collect();
    expected.push((0, 12));
    assert_eq!(guids, expected);

    // Buying back slot 5 frees it; the next sale fills it.
    slots.remove(5);
    slots.add(sale(20, 20.0));
    assert_eq!(slots.get(5).unwrap().item.item_guid, 20);
}

#[test]
fn same_tick_sales_keep_request_order_across_multiple_wraps() {
    let mut slots = BuybackSlots::default();
    for guid in 0..25 {
        slots.add(sale(guid, 0.0));
    }
    assert_eq!(
        slots
            .entries()
            .map(|(_, entry)| entry.item.item_guid)
            .collect::<Vec<_>>(),
        (13..25).collect::<Vec<_>>()
    );
}

#[test]
fn wrapped_buyback_list_restores_the_thirteenth_sale_by_its_real_slot() {
    let Some(mut h) = harness() else { return };
    let godric = h.open(GODRIC_ROTHGAR);
    h.take::<BuybackList>();
    let mut sold_ids = Vec::new();
    for index in 0..13 {
        let (slot, item_id) = if index == 12 {
            (5, 2385)
        } else {
            (VEST_SLOT, TARNISHED_CHAIN_VEST)
        };
        h.buy(godric, slot, item_id, 1);
        let guid = h
            .inventory()
            .items
            .iter()
            .find(|item| item.item_id == item_id)
            .unwrap()
            .item_guid;
        h.sell(godric, guid, 0);
        sold_ids.push(item_id);
    }
    assert!(h.failures().is_empty());
    let lists = h.take::<BuybackList>();
    let last = lists.last().unwrap();
    assert_eq!(
        last.items.iter().map(|item| item.slot).collect::<Vec<_>>(),
        (1..12).chain([0]).collect::<Vec<_>>()
    );
    assert_eq!(
        last.items
            .iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>(),
        sold_ids[1..]
    );
    let newest = last.items.last().unwrap();
    h.send::<_, MerchantChannel>(BuybackItemRequest {
        npc: godric.to_bits(),
        slot: newest.slot,
    });
    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(2385), vec![1]);
    assert!(h.count_of(TARNISHED_CHAIN_VEST).is_empty());
}

#[test]
fn godric_repairs_all_damaged_gear_for_the_azerothcore_cost() {
    let Some(mut h) = harness() else { return };
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);

    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: None,
        guild_bank: false,
    });

    assert!(h.failures().is_empty());
    // 11 * 0.8 -> 8 and 10 * 0.8 -> 8 copper.
    assert_eq!(h.inventory().gold, 100_000 - 16);
    assert_eq!(durability(&h), vec![(55, 55), (35, 35)]);
}

#[test]
fn repair_stops_at_the_money_left_and_needs_a_repairing_vendor() {
    let Some(mut h) = harness() else { return };
    damage_gear(&mut h);
    let danil = h.open(BROTHER_DANIL);
    h.send::<_, MerchantChannel>(RepairItem {
        npc: danil.to_bits(),
        item_guid: None,
        guild_bank: false,
    });
    assert_eq!(h.failures(), vec![MerchantError::NotInteracting]);

    let godric = h.open(GODRIC_ROTHGAR);
    h.set_gold(10);
    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: None,
        guild_bank: false,
    });

    assert_eq!(h.failures(), vec![MerchantError::NotEnoughMoney]);
    assert_eq!(h.inventory().gold, 2);
    assert_eq!(durability(&h), vec![(55, 55), (25, 35)]);
}

/// Grants `item_id` and equips it from the bags; returns its guid.
fn equip(h: &mut Harness, item_id: u32) -> u64 {
    h.grant(item_id, 1);
    let world = h.app.world();
    let db = &world.resource::<GameDatabase>().db;
    let mut model =
        crate::inventory::load_model(db, world.resource::<ItemTemplates>(), h.character_id)
            .unwrap();
    let (from, stack) = model
        .view()
        .into_iter()
        .find(|(location, stack)| {
            stack.item_id == item_id && matches!(location, ItemLocation::Bag { .. })
        })
        .unwrap();
    model.auto_equip(from).unwrap();
    crate::persistence::inventory_store::save_inventory_record(
        db,
        h.character_id,
        &model.into_record(),
    );
    stack.item_guid
}

#[test]
fn repairing_one_item_repairs_only_that_equipped_item() {
    let Some(mut h) = harness() else { return };
    let vest = equip(&mut h, TARNISHED_CHAIN_VEST);
    equip(&mut h, LARGE_ROUND_SHIELD);
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);

    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: Some(vest),
        guild_bank: false,
    });

    assert!(h.failures().is_empty());
    // The vest's 11 points cost 8 copper; the shield stays damaged.
    assert_eq!(h.inventory().gold, 100_000 - 8);
    assert_eq!(durability(&h), vec![(55, 55), (25, 35)]);
}

#[test]
fn repairing_an_undamaged_bag_item_is_refused() {
    let Some(mut h) = harness() else { return };
    damage_gear(&mut h);
    h.grant(LINEN, 1);
    let linen = h
        .inventory()
        .items
        .iter()
        .find(|item| item.item_id == LINEN)
        .unwrap()
        .item_guid;
    let godric = h.open(GODRIC_ROTHGAR);

    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: Some(linen),
        guild_bank: false,
    });

    assert_eq!(h.failures(), vec![MerchantError::ItemNotFound]);
    assert_eq!(h.inventory().gold, 100_000);
    assert_eq!(durability(&h), vec![(44, 55), (25, 35)]);
}

fn damaged_vest_in_backpack(h: &mut Harness) -> u64 {
    let guid = equip(h, TARNISHED_CHAIN_VEST);
    let world = h.app.world();
    let db = &world.resource::<GameDatabase>().db;
    let mut character = persistence::load_character(db, h.character_id).unwrap();
    character.durability = vec![persistence::DurabilityEntryData {
        slot: EquipmentVisualSlot::Chest,
        current: 44,
        max: 55,
    }];
    persistence::save_character(db, h.character_id, &character);
    let mut model =
        crate::inventory::load_model(db, world.resource::<ItemTemplates>(), h.character_id)
            .unwrap();
    model
        .swap(
            ItemLocation::Equipment(shared::protocol::EquipmentSlot::Chest),
            ItemLocation::Bag { bag: 0, slot: 15 },
        )
        .unwrap();
    save_inventory_record(db, h.character_id, &model.into_record());
    guid
}

fn bag_durability(h: &Harness, guid: u64) -> Option<shared::protocol::ItemDurability> {
    let world = h.app.world();
    crate::inventory::load_model(
        &world.resource::<GameDatabase>().db,
        world.resource::<ItemTemplates>(),
        h.character_id,
    )
    .unwrap()
    .inventory_snapshot()
    .bags
    .into_iter()
    .flat_map(|bag| bag.items)
    .find(|item| item.item.item_guid == guid)
    .unwrap()
    .item
    .durability
}

#[test]
fn repairing_a_damaged_bag_item_persists_its_durability_and_charges_only_its_cost() {
    let Some(mut h) = harness() else { return };
    let guid = damaged_vest_in_backpack(&mut h);
    assert_eq!(
        bag_durability(&h, guid),
        Some(shared::protocol::ItemDurability {
            current: 44,
            max: 55
        })
    );
    let godric = h.open(GODRIC_ROTHGAR);
    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: Some(guid),
        guild_bank: false,
    });
    assert!(h.failures().is_empty());
    assert_eq!(h.inventory().gold, 100_000 - 8);
    assert_eq!(
        bag_durability(&h, guid),
        Some(shared::protocol::ItemDurability {
            current: 55,
            max: 55
        })
    );
    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: Some(guid),
        guild_bank: false,
    });
    assert_eq!(h.failures(), vec![MerchantError::ItemNotFound]);
    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: Some(u64::MAX),
        guild_bank: false,
    });
    assert_eq!(h.failures(), vec![MerchantError::ItemNotFound]);
    assert_eq!(h.inventory().gold, 100_000 - 8);
    // Re-equipping must retain the repaired item's state, not its old slot value.
    let world = h.app.world();
    let db = &world.resource::<GameDatabase>().db;
    let mut model =
        crate::inventory::load_model(db, world.resource::<ItemTemplates>(), h.character_id)
            .unwrap();
    model
        .auto_equip(ItemLocation::Bag { bag: 0, slot: 15 })
        .unwrap();
    assert_eq!(
        model
            .equipment_snapshot()
            .items
            .into_iter()
            .find(|entry| entry.item.item_guid == guid)
            .unwrap()
            .item
            .durability,
        Some(shared::protocol::ItemDurability {
            current: 55,
            max: 55
        })
    );
}

#[test]
fn bag_repair_with_insufficient_money_keeps_damage_and_gold() {
    let Some(mut h) = harness() else { return };
    let guid = damaged_vest_in_backpack(&mut h);
    h.set_gold(7);
    let godric = h.open(GODRIC_ROTHGAR);
    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: Some(guid),
        guild_bank: false,
    });
    assert_eq!(h.failures(), vec![MerchantError::NotEnoughMoney]);
    assert_eq!(h.inventory().gold, 7);
    assert_eq!(
        bag_durability(&h, guid),
        Some(shared::protocol::ItemDurability {
            current: 44,
            max: 55
        })
    );
}

#[test]
fn admin_set_gold_persists_and_queues_the_live_money() {
    use bevy::ecs::system::RunSystemOnce;

    use crate::admin_handlers::handle_set_gold;
    use crate::admin_types::AdminResponse;

    let Some(mut h) = harness() else { return };
    let response = h
        .app
        .world_mut()
        .run_system_once(|db: Res<GameDatabase>, mut commands: Commands| {
            handle_set_gold(&db, &mut commands, "Shopper", 250_000)
        })
        .unwrap();

    assert!(matches!(
        response,
        AdminResponse::GoldSet {
            copper: 250_000,
            ..
        }
    ));
    assert_eq!(h.inventory().gold, 250_000);
    let queue = h.app.world().resource::<GoldUpdateQueue>();
    assert!(queue.changed_inventories().contains(&h.character_id));
}

#[test]
fn admin_set_durability_damages_the_live_player_and_rejects_over_100() {
    use bevy::ecs::system::RunSystemOnce;

    use crate::admin_handlers::handle_set_durability;
    use crate::admin_types::AdminResponse;

    let Some(mut h) = harness() else { return };
    damage_gear(&mut h);
    let set = |percent| {
        move |db: Res<GameDatabase>, mut commands: Commands| {
            handle_set_durability(&db, &mut commands, "Shopper", percent)
        }
    };
    let rejected = h.app.world_mut().run_system_once(set(101)).unwrap();
    assert!(
        matches!(rejected, AdminResponse::Error(message) if message == "percent must be 0..=100")
    );

    let accepted = h.app.world_mut().run_system_once(set(50)).unwrap();
    assert!(matches!(
        accepted,
        AdminResponse::DurabilitySet { percent: 50, .. }
    ));
    assert_eq!(durability(&h), vec![(27, 55), (17, 35)]);
}

fn backpack(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

#[test]
fn a_merchant_cursor_dropped_on_backpack_slot_ten_buys_the_vest_there() {
    let Some(mut h) = harness() else { return };
    let godric = h.open(GODRIC_ROTHGAR);

    h.buy_into(godric, VEST_SLOT, TARNISHED_CHAIN_VEST, backpack(9));

    assert!(h.failures().is_empty());
    assert!(
        h.bag_view()
            .contains(&(backpack(9), TARNISHED_CHAIN_VEST, 1))
    );
    assert_eq!(h.inventory().gold, 100_000 - 89);
}

#[test]
fn buying_onto_an_occupied_slot_stacks_the_same_item_and_refuses_another() {
    let Some(mut h) = harness() else { return };
    let danil = h.open(BROTHER_DANIL);
    let list = h.take::<VendorInventory>().remove(0);
    let water = list
        .items
        .iter()
        .find(|item| item.item_id == SPRING_WATER)
        .unwrap()
        .slot;
    h.buy_into(danil, water, SPRING_WATER, backpack(6));
    h.buy_into(danil, water, SPRING_WATER, backpack(6));
    assert!(h.failures().is_empty());
    assert!(h.bag_view().contains(&(backpack(6), SPRING_WATER, 10)));

    // Backpack slot 1 holds the starter Linen Cloth.
    h.buy_into(danil, water, SPRING_WATER, backpack(0));

    assert_eq!(h.failures(), vec![MerchantError::CantStack]);
    assert_eq!(h.count_of(SPRING_WATER), vec![10]);
    assert_eq!(h.inventory().gold, 100_000 - 10);
}

#[test]
fn a_purchase_overflowing_its_slot_fills_it_and_stores_the_rest() {
    let Some(mut h) = harness() else { return };
    h.grant(SPRING_WATER, 17);
    let danil = h.open(BROTHER_DANIL);
    let list = h.take::<VendorInventory>().remove(0);
    let water = list
        .items
        .iter()
        .find(|item| item.item_id == SPRING_WATER)
        .unwrap()
        .slot;
    let (at, _, _) = *h
        .bag_view()
        .iter()
        .find(|(_, item, _)| *item == SPRING_WATER)
        .unwrap();

    // Five more: three fill the stack to 20, two go to a free slot.
    h.buy_into(danil, water, SPRING_WATER, at);
    assert!(h.failures().is_empty());
    assert!(h.bag_view().contains(&(at, SPRING_WATER, 20)));
    assert_eq!(h.count_of(SPRING_WATER), vec![20, 2]);

    // A full stack takes no more (TrinityCore `CanStoreItem_InSpecificSlot`).
    h.buy_into(danil, water, SPRING_WATER, at);
    assert_eq!(h.failures(), vec![MerchantError::CantStack]);
}

#[test]
fn sell_all_junk_sells_three_grey_stacks_for_their_summed_price() {
    let Some(mut h) = harness() else { return };
    h.grant(RUINED_PELT, 2);
    h.grant(BROKEN_FANG, 1);
    h.grant(CHIPPED_CLAW, 3);
    let godric = h.open(GODRIC_ROTHGAR);
    h.take::<BuybackList>();

    h.send::<_, MerchantChannel>(SellAllJunkItems {
        npc: godric.to_bits(),
    });

    assert!(h.failures().is_empty());
    assert_eq!(h.inventory().gold, 100_000 + 2 * 5 + 6 + 3 * 4);
    for grey in [RUINED_PELT, BROKEN_FANG, CHIPPED_CLAW] {
        assert!(h.count_of(grey).is_empty(), "{grey} sold");
    }
    assert_eq!(h.count_of(LINEN), vec![20], "common items stay");
    // TrinityCore `SellItemToVendor` puts each sale in a buyback slot.
    let buyback = h.take::<BuybackList>().pop().unwrap();
    assert_eq!(buyback.items.len(), 3);
}

#[test]
fn sell_all_junk_needs_the_vendor_open() {
    let Some(mut h) = harness() else { return };
    h.grant(RUINED_PELT, 2);
    let godric = h.spawn_vendor(GODRIC_ROTHGAR);

    h.send::<_, MerchantChannel>(SellAllJunkItems {
        npc: godric.to_bits(),
    });

    assert_eq!(h.failures(), vec![MerchantError::NotInteracting]);
    assert_eq!(h.count_of(RUINED_PELT), vec![2]);
}

impl Harness {
    fn source_collision(&mut self, id: u32) {
        use crate::persistence::inventory_store::{load_inventory_record, save_inventory_record};
        use shared::item_data::ItemDefinitionSource::Forever70205;
        let plain = crate::item_templates::ItemTemplate {
            max_stack: 200,
            allowable_class: -1,
            allowable_race: u64::MAX,
            ..Default::default()
        };
        let mut templates = self.app.world_mut().resource_mut::<ItemTemplates>();
        templates.retail.insert(id, plain);
        templates.forever.insert(id, plain);
        let mut catalog = self.app.world_mut().resource_mut::<VendorCatalog>();
        catalog.goods.insert(
            id,
            catalog::VendorGoods {
                name: format!("Retail {id}"),
                quality: 1,
                buy_price: 1,
                sell_price: 1,
                buy_count: 1,
                max_count: 1,
            },
        );
        catalog.vendors.insert(
            GODRIC_ROTHGAR,
            vec![catalog::VendorSlot {
                item_id: id,
                maxcount: 0,
                incrtime: 0,
            }],
        );
        let db = &self.app.world().resource::<GameDatabase>().db;
        let mut record = load_inventory_record(db, self.character_id).unwrap();
        record.inventory.items.clear();
        record.inventory.items.push(InventoryItemData {
            item_guid: 1001,
            item_id: id,
            name: format!("Forever {id}"),
            quality: 1,
            required_level: 0,
            stack_count: 1,
            vendor_sell_price: 1,
            unique_auction: false,
        });
        record.definition_sources.insert(1001, Forever70205);
        save_inventory_record(db, self.character_id, &record);
    }
}

#[test]
fn source_requirements_static_merchant_unique_limit_counts_retail_only() {
    for id in [2947, 2512, 2101] {
        let mut h = harness().expect("world vendor fixture required");
        h.source_collision(id);
        let npc = h.open(GODRIC_ROTHGAR);
        h.buy(npc, 0, id, 1);
        assert!(
            h.failures().is_empty(),
            "Forever {id} must not block Retail purchase"
        );
        assert_eq!(h.count_of(id), vec![1, 1]);
        h.buy(npc, 0, id, 1);
        assert_eq!(h.failures(), vec![MerchantError::CantCarryMore]);
        assert_eq!(h.count_of(id), vec![1, 1]);
    }
}

/// TC BuyItemFromVendorSlot/SellItemToVendor: committed purchases (219), sales (220) and
/// sale copper (59) only; refusals and buybacks credit nothing.
#[test]
fn achievements_criteria_vendor_purchases_and_sales_count_only_committed_trades() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let mut h = harness_with(|app| {
        app.add_plugins(AchievementPlugin);
    })
    .expect("authentic vendor database");
    h.app
        .insert_resource(load_achievement_catalog(&h.conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(PlayerAchievementState::default());
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(crate::auth::Authenticated { account_id: 1 });
    // Hidden NPE trackers14223 (criterion48159) / 14221 (47940); Gold from vendors921 (4091).
    let counters = |h: &Harness| {
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap();
        [48159, 47940, 4091].map(|id| state.criteria_progress.get(&id).copied())
    };
    let godric = h.open(GODRIC_ROTHGAR);
    h.set_gold(88);
    h.buy(godric, VEST_SLOT, TARNISHED_CHAIN_VEST, 1);
    assert_eq!(h.failures(), vec![MerchantError::NotEnoughMoney]);
    assert_eq!(counters(&h), [None, None, None]);
    h.set_gold(100_000);
    h.buy(godric, VEST_SLOT, TARNISHED_CHAIN_VEST, 1);
    assert!(h.failures().is_empty());
    assert_eq!(counters(&h), [Some(1), None, None]);
    let linen = h.guid_of(LINEN);
    h.sell(godric, linen, 5);
    assert!(h.failures().is_empty());
    assert_eq!(counters(&h), [Some(1), Some(1), Some(5 * 13)]);
    h.send::<_, MerchantChannel>(BuybackItemRequest {
        npc: godric.to_bits(),
        slot: 0,
    });
    assert!(h.failures().is_empty());
    assert_eq!(counters(&h), [Some(1), Some(1), Some(5 * 13)]);
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.player)
        .unwrap();
    assert!(state.earned_ids.contains(&14223) && state.earned_ids.contains(&14221));
}
