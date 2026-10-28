//! Putrefy grants the permanent ghoul Unholy Devotion.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AuraRollAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 1277016).then_some(grant_devotion)
}

fn grant_devotion(world: &mut World, owner: Entity, _: &SpellInfo) {
    if !combat::auras::has_aura(world, owner, 1241858) {
        return;
    }
    let Some(pet) = combat::hunter_pet::summoned_pet(world, owner) else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp3719–3741: each Putrefied ghoul
    // grants an independently expiring native Unholy Devotion stack.
    if !spell_cast::try_cast_triggered(world, owner, 1270491, Some(pet), 1) {
        warn!(?owner, ?pet, "Unholy Devotion application refused");
    }
}
