//! Keeper's Seething Core: item-scaled haste and the two-stack bonus.
use crate::auras::{self, AuraInstance, scripts::AuraScripts};
use bevy::prelude::*;

const DRIVER: u32 = 1292065;
const FOCUS: u32 = 1295582;

pub(super) fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(FOCUS).or_default();
    script.on_apply = Some(update_haste);
    script.on_refresh = Some(update_haste);
    script.on_stack_loss = Some(stack_lost);
}

fn stack_lost(world: &mut World, unit: Entity, aura: &AuraInstance, _: u8) {
    if aura.stacks > 0 {
        update_haste(world, unit, aura);
    }
}

fn update_haste(world: &mut World, unit: Entity, aura: &AuraInstance) {
    // SimC76f8118 unique_gear_midnight.cpp4050–4074: the driver's item-scaled
    // value feeds the buff, with its second-effect multiplier at two stacks.
    let level = aura
        .cast_item_level
        .expect("Keeper's Seething Core item level");
    let base = super::native_amount(world, unit, DRIVER, 0, Some(level));
    let bonus = super::native_amount(world, unit, DRIVER, 1, Some(level)) / 100.0;
    let amount = base * if aura.stacks >= 2 { 1.0 + bonus } else { 1.0 };
    assert!(auras::set_effect_amount(
        world,
        unit,
        aura.instance_id,
        0,
        amount
    ));
}
