//! Fury MID2 two-piece extends Recklessness on Raging and Crushing Blow.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterForegroundSpecial(after_cast)];

const SET_BONUS: u32 = 1296645;
const RECKLESSNESS: u32 = 1719;

#[derive(Component, Default)]
struct RecklessnessExtensions(u32);

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 72)
        || !combat::auras::has_aura(world, caster, SET_BONUS)
    {
        return;
    }
    // SimC76f8118 sc_warrior.cpp7166–7167 resets the cap on a hard cast.
    if spell.id == RECKLESSNESS {
        world
            .entity_mut(caster)
            .insert(RecklessnessExtensions::default());
        return;
    }
    if !matches!(spell.id, 85288 | 335097) || !combat::auras::has_aura(world, caster, RECKLESSNESS)
    {
        return;
    }
    let cap = script_effect_value(world, caster, SET_BONUS, 2) as u32;
    let duration_ms = script_effect_value(world, caster, SET_BONUS, 1) as u32;
    let mut entity = world.entity_mut(caster);
    let mut counter = entity.entry::<RecklessnessExtensions>().or_default();
    if counter.0 >= cap {
        return;
    }
    counter.0 += 1;
    // SimC76f8118 sc_warrior.cpp5053–5058/5219–5224 extends once per cast.
    combat::auras::extend_owned_duration(world, caster, caster, &[RECKLESSNESS], duration_ms);
}
