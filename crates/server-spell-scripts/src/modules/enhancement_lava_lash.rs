//! SimC76f8118 sc_shaman.cpp5160–5220: offhand Flametongue empowers Lava Lash.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    if spell.id != 60103
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 263)
        || !combat::auras::has_aura(world, caster, 319778)
    {
        return 1.0;
    }
    // Enhancement applies native Flametongue to its equipped offhand.
    1.0 + combat::spell_cast::scripts::script_effect_value(world, caster, 60103, 1) / 100.0
}
