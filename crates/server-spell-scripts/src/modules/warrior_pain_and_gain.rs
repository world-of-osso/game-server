//! Retail 382551 heals about 2% max health before healing modifiers.
//! Two Midnight Warrior logs show 12,007 / 515,794 and 14,048 / 603,478;
//! the native effect's 20 points otherwise produce ten times that amount.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::NamedHealingMultiplier(healing_multiplier)];

fn healing_multiplier(_: &World, _: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if spell.id == 382551 { 0.1 } else { 1.0 }
}
