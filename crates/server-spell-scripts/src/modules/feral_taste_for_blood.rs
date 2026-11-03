//! Taste for Blood adds its native conditional bonus during Tiger's Fury.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

const TALENT: u32 = 384665;
const TIGERS_FURY: u32 = 5217;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

fn multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if !auras::has_aura(world, caster, TIGERS_FURY) {
        return 1.0;
    }
    let Some(talent) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TALENT && aura.caster == caster)
    }) else {
        return 1.0;
    };
    let Some(base) = talent.effects.iter().find(|effect| effect.index == 0) else {
        return 1.0;
    };
    if !auras::spell_mods::class_mask_affects(talent, base, spell) {
        return 1.0;
    }
    // SimC76f8118 sc_druid.cpp14249–14250; native effect1 is the
    // extra percentage, not a replacement for the permanent effect0.
    1.0 + combat::spell_cast::scripts::script_effect_value(world, caster, TALENT, 1) / 100.0
}
