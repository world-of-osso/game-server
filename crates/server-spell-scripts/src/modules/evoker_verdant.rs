//! SimC76f8118 sc_evoker.cpp4519–4557: Verdant Embrace's selected target receives its native heal.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHit(effect_hit)];

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    (spell == 360995 && index == 0).then_some(heal)
}

fn heal(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    combat::spell_cast::cast_triggered(world, caster, 361195, Some(target), depth + 1);
}
