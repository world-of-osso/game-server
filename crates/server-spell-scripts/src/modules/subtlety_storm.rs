//! SimC76f8118 sc_rogue.cpp5817–5838/5872–5881: Shuriken Storm
//! and its clone snapshot the native rank-two stealth-stance bonus.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, handlers};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

const RANK: u32 = 1279401;
pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::PreparePolicySpell(prepare_storm)];

fn prepare_storm(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if spell.id == 197835 {
        spell.cast_damage_multiplier =
            Some(spell.cast_damage_multiplier.unwrap_or(1.0) * stance_multiplier(world, caster));
    }
}

pub(super) fn stance_multiplier(world: &World, caster: Entity) -> f32 {
    let stealth = world
        .get::<ActiveAuras>(caster)
        .is_some_and(|auras| auras.has_type(handlers::MOD_STEALTH));
    let rogue_stance = [115191, 115192, 185313]
        .iter()
        .any(|&spell| auras::has_aura(world, caster, spell));
    if has_talent(world, caster, RANK) && (stealth || rogue_stance) {
        1.0 + script_effect_value(world, caster, RANK, 0) / 100.0
    } else {
        1.0
    }
}
