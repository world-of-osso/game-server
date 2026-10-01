//! Using an item from the bags (`CMSG_USE_ITEM`, TrinityCore `WorldSession::HandleUseItemOpcode`
//! → `Player::ProcessItemCast` → `Player::CastItemUseSpell`): its on-use `ItemEffect`
//! spells are cast as the player; an expendable item loses one from its stack.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::{Connection, OpenFlags};
use shared::components::{CombatStatus, Level};
use shared::protocol::{InventoryChannel, InventoryError, InventoryErrorReason, UseItem};

use crate::auth::{Authenticated, InWorld};
use crate::gold::GoldUpdateQueue;
use crate::inventory::inventory_model::error;
use crate::item_templates::ItemTemplates;
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;
use crate::persistence::inventory_store::save_inventory_record;
use crate::spell_cast::{ItemCooldown, cast_item_spell};
use crate::spell_info::SpellStore;

/// `ITEM_SPELLTRIGGER_ON_USE` (ItemDefines.h).
const TRIGGER_ON_USE: u8 = 0;
/// `SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL` (SharedDefines.h:464).
const SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL: u32 = 0x1000_0000;

/// One on-use `ItemEffect` row of an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnUseSpell {
    pub spell_id: u32,
    /// Negative: the item is expendable and a cast uses one up
    /// (`ItemEffect.Charges`, `Spell::TakeCastItem`).
    pub charges: i32,
    pub cooldown: ItemCooldown,
}

/// On-use spells by item id, in `LegacySlotIndex` order.
#[derive(Resource, Debug, Default, Clone)]
pub struct ItemOnUseSpells(pub HashMap<u32, Vec<OnUseSpell>>);

const ON_USE_QUERY: &str = "SELECT x.ItemID, e.SpellID, e.Charges, e.SpellCategoryID, \
     e.CategoryCoolDownMSec FROM item_x_item_effect x JOIN item_effect e ON e.ID = x.ItemEffectID \
     WHERE e.TriggerType = ?1 AND e.SpellID != 0 ORDER BY x.ItemID, e.LegacySlotIndex";

/// world.db `item_x_item_effect` / `item_effect` (Retail DB2 ItemXItemEffect, ItemEffect).
pub fn load_item_on_use_spells(path: &Path) -> rusqlite::Result<ItemOnUseSpells> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = conn.prepare(ON_USE_QUERY)?;
    let mut spells: HashMap<u32, Vec<OnUseSpell>> = HashMap::new();
    let rows = statement.query_map([TRIGGER_ON_USE], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            OnUseSpell {
                spell_id: row.get(1)?,
                charges: row.get(2)?,
                cooldown: ItemCooldown {
                    category: row.get(3)?,
                    category_ms: row.get(4)?,
                },
            },
        ))
    })?;
    for row in rows {
        let (item_id, spell) = row?;
        spells.entry(item_id).or_default().push(spell);
    }
    Ok(ItemOnUseSpells(spells))
}

pub struct ItemUsePlugin;

impl Plugin for ItemUsePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_item_on_use_spells);
        app.add_systems(Update, process_use_requests);
    }
}

fn startup_load_item_on_use_spells(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    match load_item_on_use_spells(path) {
        Ok(spells) => {
            info!("Loaded on-use spells of {} items", spells.0.len());
            commands.insert_resource(spells);
        }
        Err(error) => error!(
            "item use disabled: load item_effect from {}: {error}",
            path.display()
        ),
    }
}

/// What one use request concerns.
struct UseRequest {
    connection: Entity,
    player: Entity,
    character_id: u64,
    request: UseItem,
}

fn collect_requests(world: &mut World) -> Vec<UseRequest> {
    let mut receivers = world.query_filtered::<(
        Entity,
        &InWorld,
        &PlayerLink,
        &mut MessageReceiver<UseItem>,
    ), With<Authenticated>>();
    receivers
        .iter_mut(world)
        .flat_map(|(connection, in_world, link, mut receiver)| {
            let (player, character_id) = (link.0, in_world.character_id);
            receiver
                .receive()
                .map(move |request| UseRequest {
                    connection,
                    player,
                    character_id,
                    request,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Apply each queued use request; refusals answer `InventoryError` (the cast path
/// answers its own `CastFailed`).
pub(crate) fn process_use_requests(world: &mut World) {
    for request in collect_requests(world) {
        if let Err(error) = use_item(world, &request) {
            if let Some(mut sender) =
                world.get_mut::<MessageSender<InventoryError>>(request.connection)
            {
                sender.send::<InventoryChannel>(error);
            }
        }
    }
}

/// `Player::ProcessItemCast` (Player.cpp:31665-31750): the item must be there and
/// usable at the player's level, and in combat every spell must allow it
/// (`SpellInfo::CanBeUsedInCombat`). Then its on-use spells are cast; once one
/// starts, an expendable item loses one from its stack. An item with no on-use spell
/// does nothing (`CastItemUseSpell` finds no effect to cast).
fn use_item(world: &mut World, request: &UseRequest) -> Result<(), InventoryError> {
    let (db, templates) = inventory_store(world)?;
    let model = crate::inventory::load_model(db, templates, request.character_id)?;
    let item_id = model.item_id_at(request.request.location)?;
    let template = templates.get(item_id).copied();
    let spells = world
        .get_resource::<ItemOnUseSpells>()
        .ok_or_else(|| {
            error(
                InventoryErrorReason::CantDoRightNow,
                "item_effect data is not loaded",
            )
        })?
        .0
        .get(&item_id)
        .cloned()
        .unwrap_or_default();
    let level = world
        .get::<Level>(request.player)
        .map_or(1, |level| level.0);
    if let Some(template) = template
        && u16::from(level) < template.required_level
    {
        return Err(error(
            InventoryErrorReason::CantEquipLevelI {
                level: template.required_level,
            },
            format!("item {item_id} needs level {}", template.required_level),
        ));
    }
    if spells.is_empty() {
        debug!("item {item_id} has no on-use spell");
        return Ok(());
    }
    let in_combat = world
        .get::<CombatStatus>(request.player)
        .is_some_and(|status| status.0);
    if in_combat
        && spells
            .iter()
            .any(|spell| peaceful_only(world, spell.spell_id))
    {
        return Err(error(
            InventoryErrorReason::NotInCombat,
            format!("item {item_id} cannot be used in combat"),
        ));
    }
    let mut expended = false;
    for spell in &spells {
        if cast_item_spell(world, request.player, spell.spell_id, spell.cooldown).is_ok() {
            expended |= spell.charges < 0;
        }
    }
    if expended {
        consume_one(world, request, item_id)?;
    }
    Ok(())
}

fn peaceful_only(world: &World, spell_id: u32) -> bool {
    world
        .get_resource::<SpellStore>()
        .and_then(|store| store.get(spell_id))
        .and_then(|spell| spell.misc.as_ref())
        .is_some_and(|misc| misc.attributes[0] & SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL != 0)
}

fn inventory_store(world: &World) -> Result<(&redb::Database, &ItemTemplates), InventoryError> {
    match (
        world.get_resource::<GameDatabase>(),
        world.get_resource::<ItemTemplates>(),
    ) {
        (Some(db), Some(templates)) => Ok((&db.db, templates)),
        _ => Err(error(
            InventoryErrorReason::InternalBagError,
            "no inventory database or item templates",
        )),
    }
}

/// `Spell::TakeCastItem`: an expendable item loses one; the owner gets the delta.
fn consume_one(
    world: &mut World,
    request: &UseRequest,
    item_id: u32,
) -> Result<(), InventoryError> {
    let (db, templates) = inventory_store(world)?;
    let mut model = crate::inventory::load_model(db, templates, request.character_id)?;
    if model.item_id_at(request.request.location)? != item_id {
        return Err(error(
            InventoryErrorReason::ItemNotFound,
            format!(
                "item {item_id} left {:?} during its cast",
                request.request.location
            ),
        ));
    }
    model.destroy(request.request.location, 1)?;
    save_inventory_record(db, request.character_id, &model.into_record());
    world
        .resource_mut::<GoldUpdateQueue>()
        .record_items(request.character_id);
    Ok(())
}

#[cfg(test)]
#[path = "item_use_tests.rs"]
mod tests;
