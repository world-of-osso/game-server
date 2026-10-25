//! Passive auras that need equipped armor: TC a352b1fa Player::HandlePassiveSpellLearn
//! (Player.cpp3104–3117) applies them only while HasItemFitToSpellRequirements holds,
//! and ApplyItemDependentAuras / RemoveItemDependentAurasAndCasts (8297–8317,
//! 26258–26281) re-check on equipment changes. Only players are checked. Implemented:
//! the ATTR8_REQUIRES_EQUIPPED_INV_TYPES armor branch (26217–26228), where every armor
//! slot must hold an item of the required class and subclass (Item::IsFitToSpellRequirements,
//! Item.cpp1477–1509). Weapon and other armor requirements still apply unconditionally.
use bevy::prelude::*;
use shared::components::Player;
use shared::protocol::EquipmentSlot;

use super::attributes;
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::spell_info::{SpellEquippedItems, SpellInfo};
use crate::stats::EquippedItems;

const ITEM_CLASS_ARMOR: i32 = 4;
const SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES: u32 = 0x0010_0000;
/// Player.cpp26220: HEAD, SHOULDERS, CHEST, WAIST, LEGS, FEET, WRISTS, HANDS.
const ARMOR_SLOTS: [EquipmentSlot; 8] = [
    EquipmentSlot::Head,
    EquipmentSlot::Shoulder,
    EquipmentSlot::Chest,
    EquipmentSlot::Waist,
    EquipmentSlot::Legs,
    EquipmentSlot::Feet,
    EquipmentSlot::Wrist,
    EquipmentSlot::Hands,
];

/// Whether `unit` may hold the passive `spell` given its equipped items.
pub(super) fn passive_items_fit(world: &World, unit: Entity, spell: &SpellInfo) -> bool {
    let Some(required) = spell.equipped_items.as_deref() else {
        return true;
    };
    let all_armor_slots = required.item_class == ITEM_CLASS_ARMOR
        && attributes(spell, 8) & SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES != 0;
    if !all_armor_slots || world.get::<Player>(unit).is_none() {
        return true;
    }
    let Some(equipped) = world.get::<EquippedItems>(unit) else {
        return false;
    };
    ARMOR_SLOTS.iter().all(|slot| {
        equipped
            .items
            .iter()
            .find(|(equipped_slot, ..)| equipped_slot == slot)
            .is_some_and(|&(_, source, item_id)| {
                let template = world
                    .get_resource::<ItemTemplates>()
                    .expect("equipped items require definition catalogs")
                    .get(source, item_id)
                    .unwrap_or_else(|| {
                        panic!("equipped item {item_id} has no {source:?} definition")
                    });
                fits(template, required)
            })
    })
}

/// Item::IsFitToSpellRequirements: same class; a zero subclass mask allows any subclass.
fn fits(template: &ItemTemplate, required: &SpellEquippedItems) -> bool {
    i32::from(template.scaling.class_id) == required.item_class
        && (required.item_subclass == 0
            || required.item_subclass & (1 << template.scaling.subclass_id) != 0)
}
