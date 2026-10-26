//! Improved Shadow Bolt also affects Shadowbolt Volley.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const IMPROVED_SHADOW_BOLT: u32 = 453080;
const SHADOWBOLT_VOLLEY: u32 = 453176;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::TargetDamageMultiplier(volley_multiplier)];

// SimC 76f8118 sc_warlock_init.cpp:85–86 adds Volley to effect 2's affect list.
fn volley_multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != SHADOWBOLT_VOLLEY
        || !world.get::<ActiveAuras>(caster).is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.caster == caster && aura.spell_id == IMPROVED_SHADOW_BOLT)
        })
    {
        return 1.0;
    }
    1.0 + script_effect_value(world, caster, IMPROVED_SHADOW_BOLT, 1) / 100.0
}
