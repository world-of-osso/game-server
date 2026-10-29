//! Fel Devastation's cast heal precedes its channel ticks.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 212084).then_some(heal_on_cast)
}

fn heal_on_cast(world: &mut World, caster: Entity, _: &SpellInfo) {
    if combat::auras::has_aura(world, caster, 320639) {
        // SimC76f8118 sc_demon_hunter.cpp4166–4175 executes the native heal on cast.
        spell_cast::cast_triggered(world, caster, 212106, Some(caster), 1);
    }
}
