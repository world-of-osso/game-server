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
//! Grouped corpse loot uses `group` for Mainline roll/history lifecycle and shared
//! coins. Personal item sources and ungrouped tappers retain their personal windows.

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

mod group;
mod tables;

pub(crate) use tables::{LootTables, RolledLoot, load_disenchant_tables, load_loot_tables};

/// Everyone's loot on one corpse, by character id; a taken slot becomes `None`.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub(crate) struct CorpseLoot {
    pub looters: HashMap<u64, Vec<Option<LootContent>>>,
}

/// A completed roll with neither personal nor shared/rolling loot left.
pub(crate) fn corpse_loot_is_empty(world: &World, corpse: Entity) -> bool {
    world.get::<CorpseLoot>(corpse).is_some_and(|loot| {
        loot.looters
            .values()
            .all(|slots| slots.iter().all(Option::is_none))
    }) && world
        .get::<group::CorpseGroupLoot>(corpse)
        .is_none_or(|shared| !shared.has_loot())
}

/// A personal item-loot source attached to its owner, not an NPC corpse.
#[derive(Component)]
pub(crate) struct ItemLoot;

/// TC client `LootType` of a loot source (Loot.h `GetLootTypeForClient`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LootKind(pub u8);

impl LootKind {
    /// Unit.cpp:11375-11398 creature corpses.
    pub(crate) const CORPSE: Self = Self(1);
    /// SpellEffects.cpp:3503 EffectDisEnchant.
    pub(crate) const DISENCHANTING: Self = Self(4);
    /// SpellEffects.cpp:3886 EffectSkinning.
    pub(crate) const SKINNING: Self = Self(6);
    /// GameObject.cpp:2684/3483 chests and gathering nodes.
    pub(crate) const CHEST: Self = Self(9);
}

/// A roll owns the corpse until every choice or timeout resolves.
#[derive(Component)]
pub(crate) struct PendingGroupLoot;

pub(crate) fn open_item_loot(
    world: &mut World,
    player: Entity,
    character_id: u64,
    rolled: RolledLoot,
    kind: LootKind,
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
        kind,
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
    let bag_count = |item| crate::quest_view::bag_count(&db.db, &inventory, item);
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
        world
            .entity_mut(object)
            .insert((CorpseLoot::default(), LootKind::CHEST));
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
        app.init_resource::<LootTables>()
            .init_resource::<group::GroupRolls>()
            .init_resource::<group::GroupRollRng>()
            .add_systems(
                Update,
                (
                    generate_corpse_loot,
                    group::process_rolls,
                    group::replay_history,
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

fn remaining_for(
    loot: &CorpseLoot,
    shared: Option<&group::CorpseGroupLoot>,
    character: u64,
) -> Option<Vec<LootSlot>> {
    let personal = loot
        .looters
        .get(&character)
        .filter(|slots| slots.iter().any(Option::is_some));
    let pool = shared
        .and_then(|shared| shared.pool(character))
        .filter(|pool| pool.has_loot());
    if personal.is_none() && pool.is_none() {
        return None;
    }
    let mut slots = personal.map_or_else(Vec::new, |slots| remaining_slots(slots));
    if let Some(pool) = pool {
        slots.extend(pool.remaining());
    }
    Some(slots)
}

/// Persistence, templates and names: what taking a slot reads and writes.
#[derive(SystemParam)]
struct LootStore<'w> {
    db: Option<Res<'w, GameDatabase>>,
    templates: Option<Res<'w, ItemTemplates>>,
    catalog: Res<'w, VendorCatalog>,
    scaling: Option<Res<'w, crate::item_stats::ItemScalingTables>>,
    gold: ResMut<'w, GoldUpdateQueue>,
    quests: Option<Res<'w, QuestDb>>,
    quest_dirty: Option<ResMut<'w, crate::quest::QuestDirty>>,
}

impl LootStore<'_> {
    fn get(&self) -> Option<(&Database, &ItemTemplates)> {
        Some((&self.db.as_ref()?.db, self.templates.as_deref()?))
    }

    /// Validate every member before writing any coin share (LootHandler::HandleLootMoneyOpcode).
    fn split_money(&mut self, characters: &[u64], copper: u64) -> Result<(), LootError> {
        let (db, templates) = self.get().ok_or(LootError::NoPermission)?;
        let share = copper / characters.len() as u64;
        let mut models = Vec::new();
        for &character in characters {
            let mut model =
                load_model(db, templates, character).map_err(|_| LootError::NoPermission)?;
            let gold = model
                .gold()
                .checked_add(share)
                .filter(|gold| *gold <= MAX_MONEY)
                .ok_or(LootError::TooMuchGold)?;
            model.set_gold(gold);
            models.push((character, gold, model.into_record()));
        }
        for (character, _, record) in &models {
            save_inventory_record(db, *character, record);
        }
        for (character, gold, _) in models {
            self.gold.record(character, gold);
            self.gold.record_looted_money(character, share);
        }
        Ok(())
    }

    fn persist_bound_item_credit(
        &mut self,
        character_id: u64,
        item_id: u32,
        count: u32,
    ) -> Result<bool, LootError> {
        let Some(quests) = self.quests.as_deref() else {
            return Ok(false);
        };
        let (db, templates) = self.get().ok_or(LootError::NoPermission)?;
        if templates
            .get(shared::item_data::ItemDefinitionSource::Retail, item_id)
            .is_none()
        {
            return Err(LootError::NoPermission);
        }
        let mut log = load_quest_log(db, character_id);
        let (changed, suppress_item) =
            crate::quest_rules::credit_bound_item(quests, &mut log, item_id, count);
        if changed {
            crate::persistence::quest_store::save_quest_log(db, character_id, &log);
            self.quest_dirty
                .as_mut()
                .expect("quest credit requires QuestPlugin")
                .0
                .insert(character_id);
        }
        Ok(suppress_item)
    }

    /// Store one slot's content for `character_id` from a `kind` loot source.
    fn take(
        &mut self,
        character_id: u64,
        content: &LootContent,
        kind: LootKind,
    ) -> Result<(), LootError> {
        if let LootContent::Item { item_id, count, .. } = content {
            if self.persist_bound_item_credit(character_id, *item_id, *count)? {
                return Ok(());
            }
        }
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
        if let LootContent::Item { item_id, .. } = content {
            crate::guild_news::record_item(
                db,
                character_id,
                *item_id,
                templates
                    .get(shared::item_data::ItemDefinitionSource::Retail, *item_id)
                    .expect("committed loot item template"),
                self.scaling.as_deref(),
                crate::guild_news::ItemNewsSource::Looted,
            );
        }
        self.gold.record(character_id, gold);
        match content {
            LootContent::Item { item_id, count, .. } => {
                self.gold.record_acquisition(character_id, *item_id, *count);
                self.gold
                    .record_looted_item(character_id, *item_id, *count, kind.0);
            }
            LootContent::Money { copper } => self.gold.record_looted_money(character_id, *copper),
        }
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
    fn roll(
        &self,
        entry: u32,
        difficulty_id: u32,
        character_id: u64,
        quest_only: Option<bool>,
    ) -> Vec<Option<LootContent>> {
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
        let mut rolled = self.tables.roll_creature(
            entry,
            difficulty_id,
            templates,
            needs,
            &mut rand::thread_rng(),
        );
        if let Some(quest_only) = quest_only {
            rolled.items.retain(|item| item.needs_quest == quest_only);
            if quest_only {
                rolled.copper = 0;
            }
        }
        loot_slots(rolled, &self.store.catalog)
    }
}

type Killed<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Npc,
        &'static Position,
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
    mut groups: group::GroupGeneration,
    mut senders: LootSenders,
    mut commands: Commands,
) {
    for (corpse, npc, position, tap, map, mut decay, mut death, difficulty) in &mut killed {
        let difficulty_id = difficulty.map_or(0, |difficulty| difficulty.0);
        let tappers = tap.map_or(&[][..], |tap| tap.tappers.as_slice());
        let rewarded = crate::creature_tap::rewarded_characters(
            tappers,
            (map, position),
            &players,
            groups.groups.as_deref(),
        );
        let (loot, shared) =
            groups.distribute(corpse, npc.template_id, difficulty_id, &rewarded, &rolls);
        bevy::log::debug!(
            corpse = corpse.to_bits(),
            entry = npc.template_id,
            rewarded = ?rewarded,
            personal_loot = loot.looters.values().any(|slots| slots.iter().any(Option::is_some)),
            shared_loot = shared.has_loot(),
            pending_rolls = shared.has_pending_rolls(),
            "generated corpse loot"
        );
        if loot.looters.is_empty() && !shared.has_loot() {
            corpse_looted(&mut decay, &mut death);
        }
        for (connection, in_world) in &connections {
            if remaining_for(&loot, Some(&shared), in_world.character_id).is_some() {
                senders.lootable(connection, corpse, true);
            }
        }
        let pending = shared.has_pending_rolls();
        commands
            .entity(corpse)
            .insert((loot, shared, LootKind::CORPSE));
        if pending {
            commands.entity(corpse).insert(PendingGroupLoot);
        }
    }
}

type Corpses<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static mut CorpseLoot,
        Option<&'static mut group::CorpseGroupLoot>,
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
    connections: Query<'w, 's, (Entity, &'static InWorld, Option<&'static ActiveLoot>)>,
    kinds: Query<'w, 's, &'static LootKind>,
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
        let (corpse_position, loot, shared, _, _, bounds, map) = self
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
        let slots = remaining_for(loot, shared, character_id).ok_or(LootError::NoPermission)?;
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
        Ok(slots)
    }

    /// Take one slot. Returns whether the looter's window closed (nothing left).
    fn take(
        &mut self,
        connection: Entity,
        character_id: u64,
        corpse: Entity,
        slot: u8,
    ) -> Result<bool, LootError> {
        let (content, members) = self.slot_content(corpse, character_id, slot)?;
        let kind = *self
            .kinds
            .get(corpse)
            .expect("every loot source records its loot type");
        if let LootContent::Money { copper } = content {
            if let Some(members) = &members {
                self.store.split_money(members, copper)?;
            } else {
                self.store.take(character_id, &content, kind)?;
            }
        } else {
            self.store.take(character_id, &content, kind)?;
        }
        self.clear_slot(corpse, character_id, slot);
        self.senders.removed(connection, corpse, slot);
        let affected = members.unwrap_or_else(|| vec![character_id]);
        self.refresh_viewers(corpse, connection, &affected);
        let (_, loot, shared, ..) = self.corpses.get(corpse).expect("read above");
        Ok(remaining_for(loot, shared, character_id).is_none())
    }

    fn slot_content(
        &self,
        corpse: Entity,
        character: u64,
        slot: u8,
    ) -> Result<(LootContent, Option<Vec<u64>>), LootError> {
        let (_, loot, shared, ..) = self
            .corpses
            .get(corpse)
            .map_err(|_| LootError::NoPermission)?;
        let (content, members) = if slot >= group::SHARED_SLOT_BASE {
            let pool = shared
                .and_then(|shared| shared.pool(character))
                .ok_or(LootError::NoPermission)?;
            (
                pool.slots.get(usize::from(slot - group::SHARED_SLOT_BASE)),
                Some(pool.members.clone()),
            )
        } else {
            (
                loot.looters
                    .get(&character)
                    .and_then(|slots| slots.get(usize::from(slot))),
                None,
            )
        };
        Ok((
            content.cloned().flatten().ok_or(LootError::NoPermission)?,
            members,
        ))
    }

    fn clear_slot(&mut self, corpse: Entity, character: u64, slot: u8) {
        let (_, mut loot, mut shared, mut decay, mut death, ..) =
            self.corpses.get_mut(corpse).expect("read above");
        if slot >= group::SHARED_SLOT_BASE {
            let pool = shared
                .as_deref_mut()
                .expect("shared slot")
                .pool_mut(character)
                .expect("eligible");
            pool.slots[usize::from(slot - group::SHARED_SLOT_BASE)] = None;
        } else {
            loot.looters.get_mut(&character).expect("personal slot")[usize::from(slot)] = None;
        }
        loot.looters
            .retain(|_, slots| slots.iter().any(Option::is_some));
        if loot.looters.is_empty()
            && !shared
                .as_deref()
                .is_some_and(group::CorpseGroupLoot::has_loot)
            && let (Some(decay), Some(death)) = (decay.as_deref_mut(), death.as_deref_mut())
        {
            corpse_looted(decay, death);
        }
    }

    fn refresh_viewers(&mut self, corpse: Entity, taker: Entity, affected: &[u64]) {
        let (_, loot, shared, ..) = self.corpses.get(corpse).expect("read above");
        for (connection, world, active) in self
            .connections
            .iter()
            .filter(|(_, world, _)| affected.contains(&world.character_id))
        {
            let slots = remaining_for(loot, shared, world.character_id);
            if slots.is_none() {
                self.senders.lootable(connection, corpse, false);
                if active.is_some_and(|active| active.corpse == corpse) {
                    self.senders.closed(connection, corpse);
                }
            } else if connection != taker && active.is_some_and(|active| active.corpse == corpse) {
                self.senders.response(
                    connection,
                    LootResponse {
                        corpse: corpse.to_bits(),
                        auto: false,
                        slots: slots.expect("present"),
                    },
                );
            }
        }
        // The first auto-loot request has no ActiveLoot yet.
        if let Some((_, world, _)) = self
            .connections
            .iter()
            .find(|(connection, _, _)| *connection == taker)
            && remaining_for(loot, shared, world.character_id).is_none()
        {
            let already_open = self
                .connections
                .get(taker)
                .ok()
                .is_some_and(|(_, _, active)| active.is_some_and(|active| active.corpse == corpse));
            if !already_open {
                self.senders.closed(taker, corpse);
            }
        }
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
                    .remove::<(ItemLoot, LootKind, CorpseLoot)>();
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
    decayed: Query<
        (Entity, &CorpseLoot, Option<&group::CorpseGroupLoot>),
        (
            Without<Corpse>,
            Without<ItemLoot>,
            Without<shared::protocol::GameObjectInfo>,
        ),
    >,
    connections: Query<(Entity, &InWorld)>,
    mut senders: LootSenders,
    mut commands: Commands,
) {
    for (corpse, loot, shared) in &decayed {
        for (connection, in_world) in &connections {
            if remaining_for(loot, shared, in_world.character_id).is_some() {
                senders.lootable(connection, corpse, false);
            }
        }
        commands
            .entity(corpse)
            .remove::<(CorpseLoot, group::CorpseGroupLoot)>();
    }
}

#[cfg(test)]
#[path = "loot_tests.rs"]
mod tests;
