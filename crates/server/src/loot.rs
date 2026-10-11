//! Creature loot (docs/specs/loot-frame.md): Retail personal loot on corpses.
//!
//! When an NPC dies, each of its tappers (`creature_tap`; open-world personal loot,
//! Unit.cpp:11383-11397) in its map gets their own roll of the creature's loot tables
//! (`tables::LootTables::roll_creature`) and coins. Each of them is told the corpse
//! is lootable. Requests follow AzerothCore `WorldSession::HandleLootOpcode`,
//! `HandleAutostoreLootItemOpcode` and `HandleLootMoneyOpcode` (Handlers/
//! LootHandler.cpp): the looter must be alive and within `INTERACTION_DISTANCE`
//! (AzerothCore checks `IsWithinDistInMap(player, INTERACTION_DISTANCE)`, which adds
//! both combat reaches) and have loot on the corpse. An item goes to the bags with
//! `InventoryModel::store_item`, coins with `set_gold`, both saved at once and
//! replicated through `GoldUpdateQueue`. An item that does not fit stays in the
//! window. When a looter's last slot is taken, the window closes and the corpse
//! stops being lootable for them; when nobody has loot left the corpse decays
//! faster (`combat::corpse_looted`, also applied at death when nobody got any
//! loot, AzerothCore Unit.cpp:14312). `auto` takes every slot at once.
//!
//! Out of scope: group loot methods (round robin, need/greed rolls, master loot),
//! coin splitting, dungeon round-robin loot, chest and item loot.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{Health, Npc, Position};
use shared::protocol::{
    CorpseLootable, LootChannel, LootClosed, LootContent, LootError, LootFailed, LootRelease,
    LootResponse, LootSlot, LootSlotRemoved, LootSlotRequest, LootUnit,
};

use crate::auth::InWorld;
use crate::combat::{Corpse, DeathState, corpse_looted};
use crate::creature_tap::{CreatureTap, RewardPlayers};
use crate::gold::{GoldUpdateQueue, MAX_MONEY};
use crate::inventory::load_model;
use crate::item_templates::ItemTemplates;
use crate::merchant::VendorCatalog;
use crate::networking::PlayerLink;
use crate::npc_interaction::INTERACTION_DISTANCE;
use crate::persistence::inventory_store::save_inventory_record;
use crate::persistence::quest_store::load_quest_log;
use crate::persistence::{self, GameDatabase, InventoryItemData};
use crate::quest_data::QuestDb;
use crate::quest_rules::quest_needs_item;

mod tables;

pub(crate) use tables::{LootTables, RolledLoot, load_disenchant_tables, load_loot_tables};

/// Everyone's loot on one corpse, by character id; a taken slot becomes `None`.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub(crate) struct CorpseLoot {
    pub looters: HashMap<u64, Vec<Option<LootContent>>>,
}

/// A personal item-loot source attached to its owner, not an NPC corpse.
#[derive(Component)]
pub(crate) struct ItemLoot;

pub(crate) fn open_item_loot(
    world: &mut World,
    player: Entity,
    character_id: u64,
    rolled: RolledLoot,
) {
    let slots = loot_slots(rolled, world.resource::<VendorCatalog>());
    let response = LootResponse {
        corpse: player.to_bits(),
        auto: false,
        slots: remaining_slots(&slots),
    };
    let mut connections = world.query::<(Entity, &InWorld, &PlayerLink)>();
    let connection = connections
        .iter(world)
        .find(|(_, in_world, link)| in_world.character_id == character_id && link.0 == player)
        .map(|(entity, _, _)| entity)
        .expect("item caster has an in-world connection");
    world.entity_mut(player).insert((
        ItemLoot,
        CorpseLoot {
            looters: HashMap::from([(character_id, slots)]),
        },
    ));
    world
        .entity_mut(connection)
        .insert(ActiveLoot { corpse: player });
    if let Some(mut sender) = world.get_mut::<MessageSender<LootResponse>>(connection) {
        sender.send::<LootChannel>(response);
    }
}

/// Open a validated chest through the existing loot window and inventory path.
pub(crate) fn open_game_object_loot(
    world: &mut World,
    player: Entity,
    object: Entity,
    character: u64,
    loot_id: u32,
) -> Result<(), shared::protocol::InteractionError> {
    let db = world.resource::<GameDatabase>();
    let quests = world.resource::<QuestDb>();
    let log = load_quest_log(&db.db, character);
    let inventory = persistence::load_inventory(&db.db, character);
    let bag_count = |item| {
        inventory
            .items
            .iter()
            .filter(|slot| slot.item_id == item)
            .map(|slot| slot.stack_count)
            .sum()
    };
    let rolled = world.resource::<LootTables>().roll_game_object(
        loot_id,
        world.resource::<ItemTemplates>(),
        |item| quest_needs_item(quests, &log, item, bag_count),
        &mut rand::thread_rng(),
    );
    let slots = loot_slots(rolled, world.resource::<VendorCatalog>());
    let mut connections = world.query::<(Entity, &InWorld, &PlayerLink)>();
    let connection = connections
        .iter(world)
        .find(|(_, in_world, link)| in_world.character_id == character && link.0 == player)
        .map(|(entity, _, _)| entity)
        .ok_or(shared::protocol::InteractionError::InvalidTarget)?;
    let response = LootResponse {
        corpse: object.to_bits(),
        auto: false,
        slots: remaining_slots(&slots),
    };
    if world.get::<CorpseLoot>(object).is_none() {
        world.entity_mut(object).insert(CorpseLoot::default());
    }
    world
        .get_mut::<CorpseLoot>(object)
        .expect("inserted chest loot")
        .looters
        .insert(character, slots);
    world
        .entity_mut(connection)
        .insert(ActiveLoot { corpse: object });
    if let Some(mut sender) = world.get_mut::<MessageSender<LootResponse>>(connection) {
        sender.send::<LootChannel>(response);
    }
    Ok(())
}

/// The loot source whose loot window a connection has open.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActiveLoot {
    pub corpse: Entity,
}

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LootTables>().add_systems(
            Update,
            (
                generate_corpse_loot,
                process_loot_units,
                process_loot_slots,
                process_loot_releases,
                close_invalid_loot,
                drop_decayed_loot,
            )
                .chain(),
        );
    }
}

fn distance(a: &Position, b: &Position) -> f32 {
    Vec3::new(a.x, a.y, a.z).distance(Vec3::new(b.x, b.y, b.z))
}

/// Retail `Enum.ItemQuality.Poor`.
const POOR_QUALITY: u8 = 0;

/// Slots of a roll: coins first (the Retail money slot), then the items. A poor
/// (grey) item is not dropped: its vendor sell price per item joins the coins
/// (a deliberate deviation from Retail, docs/specs/loot.md).
fn loot_slots(rolled: tables::RolledLoot, catalog: &VendorCatalog) -> Vec<Option<LootContent>> {
    let mut copper = rolled.copper;
    let mut items = Vec::new();
    for drop in rolled.items {
        let goods = catalog.goods.get(&drop.item_id);
        if let Some(goods) = goods.filter(|goods| goods.quality == POOR_QUALITY) {
            copper += u64::from(goods.sell_price) * u64::from(drop.count);
            continue;
        }
        items.push(LootContent::Item {
            item_id: drop.item_id,
            name: goods.map(|goods| goods.name.clone()).unwrap_or_default(),
            quality: goods.map_or(0, |goods| goods.quality),
            count: drop.count,
        });
    }
    let money = (copper > 0).then_some(LootContent::Money { copper });
    money.into_iter().chain(items).map(Some).collect()
}

fn remaining_slots(slots: &[Option<LootContent>]) -> Vec<LootSlot> {
    slots
        .iter()
        .enumerate()
        .filter_map(|(slot, content)| {
            Some(LootSlot {
                slot: slot as u8,
                content: content.clone()?,
            })
        })
        .collect()
}

/// Persistence, templates and names: what taking a slot reads and writes.
#[derive(SystemParam)]
struct LootStore<'w> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
    catalog: Res<'w, VendorCatalog>,
    gold: ResMut<'w, GoldUpdateQueue>,
}

impl LootStore<'_> {
    fn get(&self) -> Option<(&Database, &ItemTemplates)> {
        Some((&self.db.as_ref()?.db, self.templates.as_deref()?))
    }

    /// Store one slot's content for `character_id`.
    fn take(&mut self, character_id: u64, content: &LootContent) -> Result<(), LootError> {
        let (db, templates) = self.get().ok_or(LootError::NoPermission)?;
        let mut model =
            load_model(db, templates, character_id).map_err(|_| LootError::NoPermission)?;
        match content {
            LootContent::Money { copper } => {
                if model.gold() + copper > MAX_MONEY {
                    return Err(LootError::TooMuchGold);
                }
                model.set_gold(model.gold() + copper);
            }
            &LootContent::Item {
                item_id,
                ref name,
                quality,
                count,
            } => {
                let template = templates
                    .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
                    .copied()
                    .ok_or(LootError::NoPermission)?;
                let goods = self.catalog.goods.get(&item_id);
                let max_count = goods.map_or(0, |goods| goods.max_count);
                if max_count > 0
                    && model.owned_count(shared::item_data::ItemDefinitionSource::Retail, item_id)
                        + count
                        > max_count
                {
                    return Err(LootError::CantCarryMore);
                }
                let mut mint = || persistence::next_id(db, "item_instance");
                let item = InventoryItemData {
                    item_guid: mint(),
                    item_id,
                    name: name.clone(),
                    quality,
                    required_level: template.required_level,
                    stack_count: count,
                    vendor_sell_price: goods.map_or(0, |goods| goods.sell_price),
                    unique_auction: false,
                };
                model
                    .store_item(
                        item,
                        shared::item_data::ItemDefinitionSource::Retail,
                        template,
                        &mut mint,
                    )
                    .map_err(|_| LootError::InventoryFull)?;
            }
        }
        let gold = model.gold();
        save_inventory_record(db, character_id, &model.into_record());
        self.gold.record(character_id, gold);
        Ok(())
    }
}

#[derive(SystemParam)]
struct LootSenders<'w, 's> {
    responses: Query<'w, 's, &'static mut MessageSender<LootResponse>>,
    removed: Query<'w, 's, &'static mut MessageSender<LootSlotRemoved>>,
    closed: Query<'w, 's, &'static mut MessageSender<LootClosed>>,
    lootable: Query<'w, 's, &'static mut MessageSender<CorpseLootable>>,
    failed: Query<'w, 's, &'static mut MessageSender<LootFailed>>,
}

impl LootSenders<'_, '_> {
    fn response(&mut self, connection: Entity, response: LootResponse) {
        if let Ok(mut sender) = self.responses.get_mut(connection) {
            sender.send::<LootChannel>(response);
        }
    }

    fn removed(&mut self, connection: Entity, corpse: Entity, slot: u8) {
        if let Ok(mut sender) = self.removed.get_mut(connection) {
            sender.send::<LootChannel>(LootSlotRemoved {
                corpse: corpse.to_bits(),
                slot,
            });
        }
    }

    fn closed(&mut self, connection: Entity, corpse: Entity) {
        if let Ok(mut sender) = self.closed.get_mut(connection) {
            sender.send::<LootChannel>(LootClosed {
                corpse: corpse.to_bits(),
            });
        }
    }

    fn lootable(&mut self, connection: Entity, corpse: Entity, lootable: bool) {
        if let Ok(mut sender) = self.lootable.get_mut(connection) {
            sender.send::<LootChannel>(CorpseLootable {
                corpse: corpse.to_bits(),
                lootable,
            });
        }
    }

    fn failed(&mut self, connection: Entity, corpse: u64, error: LootError) {
        if let Ok(mut sender) = self.failed.get_mut(connection) {
            sender.send::<LootChannel>(LootFailed { corpse, error });
        }
    }
}

#[derive(SystemParam)]
struct LootRolls<'w> {
    tables: Res<'w, LootTables>,
    store: LootStore<'w>,
    quests: Option<Res<'w, QuestDb>>,
}

impl LootRolls<'_> {
    fn roll(&self, entry: u32, difficulty_id: u32, character_id: u64) -> Vec<Option<LootContent>> {
        let Some((db, templates)) = self.store.get() else {
            return Vec::new();
        };
        let log = load_quest_log(db, character_id);
        let inventory = persistence::load_inventory(db, character_id);
        let bag_count = |item_id: u32| {
            inventory
                .items
                .iter()
                .filter(|item| item.item_id == item_id)
                .map(|item| item.stack_count)
                .sum()
        };
        let needs = |item_id: u32| {
            self.quests
                .as_deref()
                .is_some_and(|quests| quest_needs_item(quests, &log, item_id, bag_count))
        };
        let rolled = self.tables.roll_creature(
            entry,
            difficulty_id,
            templates,
            needs,
            &mut rand::thread_rng(),
        );
        loot_slots(rolled, &self.store.catalog)
    }
}

type Killed<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Npc,
        Option<&'static CreatureTap>,
        &'static crate::maps::InMap,
        &'static mut Corpse,
        &'static mut DeathState,
        Option<&'static crate::zones::CreatureDifficulty>,
    ),
    Added<DeathState>,
>;

fn generate_corpse_loot(
    mut killed: Killed,
    players: RewardPlayers,
    connections: Query<(Entity, &InWorld)>,
    rolls: LootRolls,
    mut senders: LootSenders,
    mut commands: Commands,
) {
    for (corpse, npc, tap, map, mut decay, mut death, difficulty) in &mut killed {
        let difficulty_id = difficulty.map_or(0, |difficulty| difficulty.0);
        let mut loot = CorpseLoot::default();
        let tappers = tap.map_or(&[][..], |tap| tap.tappers.as_slice());
        for &character_id in tappers.iter().filter(|&&tapper| {
            players
                .iter()
                .any(|(id, _, at)| id.0 == tapper && at == map)
        }) {
            let slots = rolls.roll(npc.template_id, difficulty_id, character_id);
            if !slots.is_empty() {
                loot.looters.insert(character_id, slots);
            }
        }
        if loot.looters.is_empty() {
            corpse_looted(&mut decay, &mut death);
            continue;
        }
        for (connection, in_world) in &connections {
            if loot.looters.contains_key(&in_world.character_id) {
                senders.lootable(connection, corpse, true);
            }
        }
        commands.entity(corpse).insert(loot);
    }
}

type Corpses<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static mut CorpseLoot,
        Option<&'static mut Corpse>,
        Option<&'static mut DeathState>,
        Option<&'static crate::game_objects::GameObjectBounds>,
        Option<&'static crate::maps::InMap>,
    ),
>;

/// Everything a loot request reads and writes.
#[derive(SystemParam)]
struct Looting<'w, 's> {
    store: LootStore<'w>,
    corpses: Corpses<'w, 's>,
    players: Query<
        'w,
        's,
        (
            &'static Position,
            &'static Health,
            Option<&'static crate::maps::InMap>,
        ),
    >,
    senders: LootSenders<'w, 's>,
}

impl Looting<'_, '_> {
    /// AzerothCore `Player::SendLoot` checks, returning the looter's slots.
    fn open(
        &self,
        player: Entity,
        character_id: u64,
        corpse: Entity,
    ) -> Result<Vec<LootSlot>, LootError> {
        let (corpse_position, loot, _, _, bounds, map) = self
            .corpses
            .get(corpse)
            .map_err(|_| LootError::NoPermission)?;
        let (player_position, health, player_map) = self
            .players
            .get(player)
            .map_err(|_| LootError::NoPermission)?;
        if health.current <= 0.0 {
            return Err(LootError::PlayerDead);
        }
        let slots = loot
            .looters
            .get(&character_id)
            .ok_or(LootError::NoPermission)?;
        if map.is_some() && map != player_map {
            return Err(LootError::NoPermission);
        }
        let in_reach = bounds.map_or_else(
            || distance(player_position, corpse_position) <= INTERACTION_DISTANCE,
            |bounds| {
                bounds.reaches(
                    crate::game_objects::wow_position(corpse_position),
                    crate::game_objects::wow_position(player_position),
                    crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
                )
            },
        );
        if !in_reach {
            return Err(LootError::TooFar);
        }
        Ok(remaining_slots(slots))
    }

    /// Take one slot. Returns whether the looter's window closed (nothing left).
    fn take(
        &mut self,
        connection: Entity,
        character_id: u64,
        corpse: Entity,
        slot: u8,
    ) -> Result<bool, LootError> {
        let content = self
            .corpses
            .get(corpse)
            .ok()
            .and_then(|(_, loot, ..)| {
                loot.looters
                    .get(&character_id)?
                    .get(usize::from(slot))
                    .cloned()
                    .flatten()
            })
            .ok_or(LootError::NoPermission)?;
        self.store.take(character_id, &content)?;
        self.senders.removed(connection, corpse, slot);
        let (_, mut loot, mut decay, mut death, ..) =
            self.corpses.get_mut(corpse).expect("read above");
        let slots = loot.looters.get_mut(&character_id).expect("read above");
        slots[usize::from(slot)] = None;
        if slots.iter().any(Option::is_some) {
            return Ok(false);
        }
        loot.looters.remove(&character_id);
        if loot.looters.is_empty()
            && let (Some(decay), Some(death)) = (decay.as_deref_mut(), death.as_deref_mut())
        {
            corpse_looted(decay, death);
        }
        self.senders.lootable(connection, corpse, false);
        self.senders.closed(connection, corpse);
        Ok(true)
    }

    /// Take every slot; the first refusal is reported and its slot stays.
    fn take_all(
        &mut self,
        connection: Entity,
        character_id: u64,
        corpse: Entity,
        slots: &[LootSlot],
    ) -> bool {
        let mut error = None;
        for slot in slots {
            match self.take(connection, character_id, corpse, slot.slot) {
                Ok(true) => return true,
                Ok(false) => {}
                Err(refusal) => {
                    error.get_or_insert(refusal);
                }
            }
        }
        if let Some(error) = error {
            self.senders.failed(connection, corpse.to_bits(), error);
        }
        false
    }
}

type Requests<'w, 's, M> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static PlayerLink,
        Option<&'static ActiveLoot>,
        &'static mut MessageReceiver<M>,
    ),
>;

fn process_loot_units(
    mut requests: Requests<LootUnit>,
    mut looting: Looting,
    mut commands: Commands,
) {
    for (connection, in_world, link, active, mut receiver) in &mut requests {
        let mut current = active.map(|active| active.corpse);
        for request in receiver.receive() {
            let result = Entity::try_from_bits(request.corpse)
                .ok_or(LootError::NoPermission)
                .and_then(|corpse| {
                    Ok((corpse, looting.open(link.0, in_world.character_id, corpse)?))
                });
            let (corpse, slots) = match result {
                Ok(opened) => opened,
                Err(error) => {
                    looting.senders.failed(connection, request.corpse, error);
                    continue;
                }
            };
            if let Some(previous) = current.filter(|previous| *previous != corpse) {
                looting.senders.closed(connection, previous);
            }
            looting.senders.response(
                connection,
                LootResponse {
                    corpse: request.corpse,
                    auto: request.auto,
                    slots: slots.clone(),
                },
            );
            let closed =
                request.auto && looting.take_all(connection, in_world.character_id, corpse, &slots);
            if closed {
                current = None;
                commands.entity(connection).remove::<ActiveLoot>();
            } else {
                current = Some(corpse);
                commands.entity(connection).insert(ActiveLoot { corpse });
            }
        }
    }
}

fn process_loot_slots(
    mut requests: Requests<LootSlotRequest>,
    mut looting: Looting,
    mut commands: Commands,
) {
    for (connection, in_world, link, active, mut receiver) in &mut requests {
        let mut current = active.map(|active| active.corpse);
        for request in receiver.receive() {
            let Some(corpse) = current.filter(|corpse| corpse.to_bits() == request.corpse) else {
                looting
                    .senders
                    .failed(connection, request.corpse, LootError::NoPermission);
                continue;
            };
            if let Err(error) = looting.open(link.0, in_world.character_id, corpse) {
                looting.senders.failed(connection, request.corpse, error);
                continue;
            }
            match looting.take(connection, in_world.character_id, corpse, request.slot) {
                Ok(true) => {
                    current = None;
                    commands.entity(connection).remove::<ActiveLoot>();
                }
                Ok(false) => {}
                Err(error) => looting.senders.failed(connection, request.corpse, error),
            }
        }
    }
}

fn process_loot_releases(
    item_sources: Query<(), With<ItemLoot>>,
    mut requests: Requests<LootRelease>,
    mut senders: LootSenders,
    mut commands: Commands,
) {
    for (connection, _, _, active, mut receiver) in &mut requests {
        let releases = receiver
            .receive()
            .filter(|request| {
                active.is_some_and(|active| active.corpse.to_bits() == request.corpse)
            })
            .count();
        if let Some(active) = active.filter(|_| releases > 0) {
            senders.closed(connection, active.corpse);
            commands.entity(connection).remove::<ActiveLoot>();
            if item_sources.contains(active.corpse) {
                commands
                    .entity(active.corpse)
                    .remove::<(ItemLoot, CorpseLoot)>();
            }
        }
    }
}

/// Close a window whose corpse decayed, or whose looter died or walked away.
fn close_invalid_loot(
    connections: Query<(Entity, &InWorld, &PlayerLink, &ActiveLoot)>,
    mut looting: Looting,
    mut commands: Commands,
) {
    for (connection, in_world, link, active) in &connections {
        if looting
            .open(link.0, in_world.character_id, active.corpse)
            .is_ok()
        {
            continue;
        }
        looting.senders.closed(connection, active.corpse);
        commands.entity(connection).remove::<ActiveLoot>();
    }
}

/// A decayed corpse (no longer `Corpse`) loses its loot; its remaining looters are
/// told it is no longer lootable.
fn drop_decayed_loot(
    decayed: Query<(Entity, &CorpseLoot), (Without<Corpse>, Without<ItemLoot>)>,
    connections: Query<(Entity, &InWorld)>,
    mut senders: LootSenders,
    mut commands: Commands,
) {
    for (corpse, loot) in &decayed {
        for (connection, in_world) in &connections {
            if loot.looters.contains_key(&in_world.character_id) {
                senders.lootable(connection, corpse, false);
            }
        }
        commands.entity(corpse).remove::<CorpseLoot>();
    }
}

#[cfg(test)]
#[path = "loot_tests.rs"]
mod tests;
