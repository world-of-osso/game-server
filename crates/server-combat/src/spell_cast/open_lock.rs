//! Owned bag-item effect33/target26. DB2 ItemSparse.LockID + Lock alternatives.
//! TC a352b1fa Spell.cpp:6338/8655 and SpellEffects.cpp:1643: validate before
//! launch, persist UNLOCKED on the GUID; opening loot is a separate operation.
use std::collections::HashMap;

use super::CastFailure;
use crate::game_objects::locks::{LockCastResult, LockDb};
use crate::item_templates::ItemTemplates;
use crate::persistence::{ClientId, GameDatabase, inventory_store};
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;
use shared::item_data::ItemDefinitionSource::Retail;

const EFFECT_OPEN_LOCK: u32 = 33;
const TARGET_GAMEOBJECT_ITEM: u32 = 26;
const LOCKTYPE_LOCKPICKING: u32 = 1;

#[derive(Resource)]
pub struct ItemLocks {
    items: HashMap<u32, u32>,
    locks: LockDb,
}

pub(crate) fn load_data(path: &str) -> rusqlite::Result<ItemLocks> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT ID, LockID FROM item_sparse WHERE LockID != 0")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(ItemLocks {
        items: rows.collect::<rusqlite::Result<_>>()?,
        locks: crate::game_objects::locks::load_locks(path)?,
    })
}

pub(crate) fn load_startup(mut commands: Commands) {
    match load_data(crate::spell_info::WORLD_DB_PATH) {
        Ok(data) => {
            commands.insert_resource(data);
        }
        Err(error) => error!("Item lock metadata unavailable: {error}"),
    }
}

pub(crate) fn is_item_effect(effect: &SpellEffect) -> bool {
    effect.effect == EFFECT_OPEN_LOCK && effect.implicit_targets.contains(&TARGET_GAMEOBJECT_ITEM)
}

pub(crate) fn is_item_cast(spell: &SpellInfo) -> bool {
    spell.cast_target_item_guid.is_some() && spell.effects.iter().any(is_item_effect)
}

fn load_locked_item(world: &World, caster: Entity, guid: u64) -> Result<u32, CastFailure> {
    let owner = world
        .get::<ClientId>(caster)
        .ok_or(CastFailure::InvalidTarget)?
        .0;
    let db = world
        .get_resource::<GameDatabase>()
        .ok_or_else(|| CastFailure::Unsupported("Item lock inventory unavailable".into()))?;
    let model = crate::inventory::load_model(&db.db, world.resource::<ItemTemplates>(), owner)
        .map_err(|error| {
            CastFailure::Unsupported(format!("Item lock inventory unavailable: {error:?}"))
        })?;
    let item = model.bag_item(guid).ok_or(CastFailure::InvalidTarget)?;
    if model.definition_source(guid) != Retail || inventory_store::is_item_unlocked(&db.db, guid) {
        return Err(CastFailure::InvalidTarget);
    }
    let data = world
        .get_resource::<ItemLocks>()
        .ok_or_else(|| CastFailure::Unsupported("Item lock metadata unavailable".into()))?;
    data.items
        .get(&item.item_id)
        .copied()
        .ok_or(CastFailure::InvalidTarget)
}

fn check_effects(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    lock: u32,
) -> Result<(), CastFailure> {
    let data = world.resource::<ItemLocks>();
    for effect in spell.effects.iter().filter(|effect| is_item_effect(effect)) {
        if effect.misc_values[0] != LOCKTYPE_LOCKPICKING as i32 {
            return Err(CastFailure::Unsupported(format!(
                "Item lock type {} not implemented",
                effect.misc_values[0]
            )));
        }
        // Native RealPointsPerLevel goes through the normal cast value calculation.
        // Pinned TC CanOpenLock calls CalcValueAsInt without a caster; that reference
        // cannot certify Retail per-level lock thresholds (see class-completeness2).
        let points =
            super::effects::modded_base_points(world, caster, spell, effect).round() as i32;
        match data
            .locks
            .check_spell(lock, spell.id, LOCKTYPE_LOCKPICKING, points)
            .map_err(|error| CastFailure::Unsupported(error.to_string()))?
        {
            LockCastResult::Allowed => {}
            LockCastResult::InvalidTarget => return Err(CastFailure::InvalidTarget),
            LockCastResult::LowSkill => return Err(CastFailure::LowCastLevel),
        }
    }
    Ok(())
}

pub(crate) fn check_cast(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if is_item_cast(spell) {
        let guid = spell.cast_target_item_guid.expect("item cast GUID");
        let lock = load_locked_item(world, caster, guid)?;
        check_effects(world, caster, spell, lock)?;
    }
    Ok(())
}

/// Run after final validation and target planning, before publishing CastSuccess.
pub(crate) fn unlock_cast(world: &World, spell: &SpellInfo) -> Result<(), CastFailure> {
    if !is_item_cast(spell) {
        return Ok(());
    }
    let guid = spell.cast_target_item_guid.expect("validated item GUID");
    inventory_store::unlock_item(&world.resource::<GameDatabase>().db, guid)
        .map_err(|error| CastFailure::Unsupported(format!("Persist item unlock: {error}")))
}
