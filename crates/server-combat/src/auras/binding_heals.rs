//! TC a352b1fa spell_priest.cpp859–889 and
//! sql/old/11.x/world/25101_2026_01_14/2025_12_30_01_world.sql.
use bevy::prelude::*;

use super::AuraInstance;
use super::procs::AuraEvent;
use crate::spell_info::SpellStore;

pub(super) const BINDING_HEALS: u32 = 368_275;
const BINDING_HEAL: u32 = 368_276;
const PRIEST_FAMILY: u32 = 6;
const BINDING_SOURCE_MASK: u32 = 0x1800;

pub(super) fn check_heal(world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    // The explicit spell_proc row supplies the family mask and heal-only hit
    // phase; these are absent from the native aura's zero effect class mask.
    event.target.is_some_and(|target| target != event.actor)
        && event.heal.is_some()
        && event
            .spell_id
            .and_then(|id| world.resource::<SpellStore>().get(id))
            .and_then(|spell| spell.class_options.as_ref())
            .is_some_and(|options| {
                options.spell_class_set == PRIEST_FAMILY
                    && options.spell_class_mask[0] & BINDING_SOURCE_MASK != 0
            })
}

pub(super) fn transfer_heal(
    world: &mut World,
    _unit: Entity,
    aura: &AuraInstance,
    event: &AuraEvent,
) {
    let Some(effect) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return;
    };
    let Some(heal) = event.heal else {
        return;
    };
    // TC CalculatePct(uint32 GetHeal, amount) truncates before the child cast.
    let amount = (f64::from(heal) * f64::from(effect.amount) / 100.0) as u32;
    // Native368276 has no cost, cast time or recovery category. Its self target
    // and real heal effect execute through the shared triggered cast path.
    crate::spell_cast::cast_triggered_with_effect0_amount(
        world,
        event.actor,
        BINDING_HEAL,
        Some(event.actor),
        1,
        amount as f32,
    );
}
