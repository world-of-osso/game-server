//! Actual runic spenders extend the active pet transformation.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AuraRollAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 47541 | 1242174 | 207317 | 383269).then_some(extend_transformation)
}

fn extend_transformation(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, 1235391) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp13565–13569: real Coil/Epidemic
    // executions extend active Dark Transformation by native Eternal Agony.
    let milliseconds = script_effect_value(world, caster, 390268, 0) as u32;
    combat::auras::extend_owned_duration(world, caster, caster, &[1235391], milliseconds);
    if let Some(pet) = combat::hunter_pet::summoned_pet(world, caster) {
        combat::auras::extend_owned_duration(world, caster, pet, &[1233448], milliseconds);
    } else {
        warn!(
            ?caster,
            "active Dark Transformation has no controlled ghoul"
        );
    }
}
