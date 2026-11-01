//! Equipped-item admission for ordinary player and class-follower casts.
use bevy::prelude::*;
use shared::protocol::EquipmentSlot;

use super::{CastFailure, attributes};
use crate::item_templates::ItemTemplates;
use crate::spell_info::{SpellEquippedItems, SpellInfo};
use crate::stats::EquippedItems;

fn matches_item(
    items: &EquippedItems,
    templates: &ItemTemplates,
    slot: EquipmentSlot,
    required: &SpellEquippedItems,
) -> bool {
    items.items.iter().any(|&(item_slot, source, id)| {
        item_slot == slot
            && templates.get(source, id).is_some_and(|item| {
                i32::from(item.scaling.class_id) == required.item_class
                    && (required.item_subclass == 0
                        || 1_u32
                            .checked_shl(u32::from(item.scaling.subclass_id))
                            .is_some_and(|bit| required.item_subclass & bit != 0))
            })
    })
}

pub fn check_cast(world: &World, caster: Entity, spell: &SpellInfo) -> Result<(), CastFailure> {
    let Some(required) = spell
        .equipped_items
        .as_deref()
        .filter(|items| items.item_class >= 0)
    else {
        return Ok(());
    };
    if world.get::<shared::components::Player>(caster).is_none()
        && world
            .get::<crate::followers::AplControlled>(caster)
            .is_none()
    {
        return Ok(());
    }
    let items = world
        .get::<EquippedItems>(caster)
        .ok_or(CastFailure::EquippedItemClass)?;
    let templates = world
        .get_resource::<ItemTemplates>()
        .ok_or(CastFailure::EquippedItemClass)?;
    let matches = |slot| matches_item(items, templates, slot, required);
    // SimC76f8118 action.cpp:1146-1168 enforces native hand flags. TC
    // a352b1fa Player.cpp:26174-26227 otherwise accepts either weapon hand.
    const REQUIRES_MAIN_HAND: u32 = 1 << 10;
    const REQUIRES_OFF_HAND: u32 = 1 << 24;
    let flags = attributes(spell, 3);
    if required.item_class == 2 {
        if flags & REQUIRES_MAIN_HAND != 0 && !matches(EquipmentSlot::MainHand)
            || flags & REQUIRES_OFF_HAND != 0 && !matches(EquipmentSlot::OffHand)
            || !(matches(EquipmentSlot::MainHand) || matches(EquipmentSlot::OffHand))
        {
            return Err(CastFailure::EquippedItemClass);
        }
    } else if !items.items.iter().any(|&(slot, _, _)| matches(slot)) {
        return Err(CastFailure::EquippedItemClass);
    }
    Ok(())
}
