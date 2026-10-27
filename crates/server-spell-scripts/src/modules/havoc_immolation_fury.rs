//! SimC76f8118 sc_demon_hunter.cpp4651–4677,4784–4800: Havoc Immolation Fury.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::spell_cast::script_registry::SpellScriptHook;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectEnabled(effect_enabled)];

fn effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    if world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(577) {
        return true;
    }
    match (spell, index) {
        (258920 | 258922, 2) => false,
        (258922, 1) => world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(320374)),
        _ => true,
    }
}
