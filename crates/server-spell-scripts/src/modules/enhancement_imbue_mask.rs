//! SimC76f8118 sc_shaman.cpp11455–11459: Enhancement selects only effect2.
use super::combat::spell_cast::script_registry::SpellScriptHook;
use bevy::prelude::*;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectEnabled(enabled)];

fn enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    spell != 462796
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 263)
        || index == 1
}
