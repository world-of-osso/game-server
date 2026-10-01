//! Real-DB tests against data/world.db; each test skips when the DB is absent.

#[path = "merchant_economy_tests.rs"]
mod economic;

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

fn world_db_path() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return None;
    }
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
    app.insert_resource(GameDatabase {
        db: persistence::temp_database(),
    });
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins((NpcInteractionPlugin, MerchantPlugin));
    app.insert_resource(load_vendor_catalog(&conn).unwrap());
    app.insert_resource(load_item_durability(&conn).unwrap());
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
    let mut expected: Vec<(u8, u64)> = (0..12).map(|slot| (slot, u64::from(slot))).collect();
    expected[0] = (0, 12);
    assert_eq!(guids, expected);

    // Buying back slot 5 frees it; the next sale fills it.
    slots.remove(5);
    slots.add(sale(20, 20.0));
    assert_eq!(slots.get(5).unwrap().item.item_guid, 20);
}

#[test]
fn godric_repairs_all_damaged_gear_for_the_azerothcore_cost() {
    let Some(mut h) = harness() else { return };
    damage_gear(&mut h);
    let godric = h.open(GODRIC_ROTHGAR);

    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: None,
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
    });
    assert_eq!(h.failures(), vec![MerchantError::NotInteracting]);

    let godric = h.open(GODRIC_ROTHGAR);
    h.set_gold(10);
    h.send::<_, MerchantChannel>(RepairItem {
        npc: godric.to_bits(),
        item_guid: None,
    });

    assert_eq!(h.failures(), vec![MerchantError::NotEnoughMoney]);
    assert_eq!(h.inventory().gold, 2);
    assert_eq!(durability(&h), vec![(55, 55), (25, 35)]);
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
