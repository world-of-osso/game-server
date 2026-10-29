//! SimC76f8118 sc_monk.cpp2992–3066: Chi Burst launches damage and healing.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 123986).then_some(launch_chi_burst)
}

fn launch_chi_burst(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 268)
    {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    spell_cast::cast_triggered(world, caster, 148135, Some(target), 1);
    spell_cast::cast_triggered(world, caster, 130654, Some(caster), 1);
}
