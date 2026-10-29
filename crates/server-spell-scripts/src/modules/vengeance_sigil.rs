//! Vengeance Sigil of Flame energizes its caster on execution.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 204596).then_some(energize)
}

fn energize(world: &mut World, caster: Entity, _: &SpellInfo) {
    if world.get::<ActiveSpec>(caster).is_some_and(|spec| spec.0 == 581) {
        // SimC 76f8118 sc_demon_hunter.cpp:4469-4473,11262 executes the Vengeance energize on cast.
        spell_cast::cast_triggered(world, caster, 389787, Some(caster), 1);
    }
}
