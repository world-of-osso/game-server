//! Equipped-item admission for ordinary player and class-follower casts.
use bevy::prelude::*;
use shared::protocol::EquipmentSlot;

use super::{CastFailure, attributes};
use crate::item_templates::ItemTemplate;
use crate::spell_info::{SpellEquippedItems, SpellInfo};
use crate::stats::EquippedItems;

fn matches_item(item: &ItemTemplate, required: &SpellEquippedItems) -> bool {
    i32::from(item.scaling.class_id) == required.item_class
        && (required.item_subclass == 0
            || 1_u32
                .checked_shl(u32::from(item.scaling.subclass_id))
                .is_some_and(|bit| required.item_subclass & bit != 0))
}

pub(crate) fn check_items<'a>(
    items: &[(EquipmentSlot, &'a ItemTemplate)],
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let Some(required) = spell
        .equipped_items
        .as_deref()
        .filter(|items| items.item_class >= 0)
    else {
        return Ok(());
    };
    let matches = |slot| {
        items
            .iter()
            .any(|(item_slot, item)| *item_slot == slot && matches_item(item, required))
    };
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
    } else if !items.iter().any(|(_, item)| matches_item(item, required)) {
        return Err(CastFailure::EquippedItemClass);
    }
    Ok(())
}

pub fn check_cast(world: &World, caster: Entity, spell: &SpellInfo) -> Result<(), CastFailure> {
    if spell
        .equipped_items
        .as_deref()
        .is_none_or(|items| items.item_class < 0)
    {
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
        .get_resource::<crate::item_templates::ItemTemplates>()
        .ok_or(CastFailure::EquippedItemClass)?;
    let equipped: Vec<_> = items
        .items
        .iter()
        .filter_map(|&(slot, source, id)| templates.get(source, id).map(|item| (slot, item)))
        .collect();
    check_items(&equipped, spell)
}
