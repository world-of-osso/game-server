//! Icy Talons increases swing speed, not swing damage.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectEnabled(effect_enabled)];

// Retail: Zurâ's same-target, same-AP noncrits at zero and three stacks are
// 13,446 and 13,476 (WoWCombatLog-102526_094013.txt, 11:28:57.583/56.175).
// SimC76f8118 sc_death_knight.cpp17086 models only the speed effect.
fn effect_enabled(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    !(spell == 194879
        && index == 1
        && world.get::<ActiveSpec>(caster).is_some_and(|spec| matches!(spec.0, 250 | 251 | 252)))
}
