//! SimC76f8118 sc_evoker.cpp5541–5548: Eye of Infinity amplifies Surge's primary.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != 359077
        || spell.cast_primary_target != Some(target)
        || !combat::auras::has_aura(world, caster, 411165)
    {
        return 1.0;
    }
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, 411165, 0);
    1.0 + percent / 100.0
}
