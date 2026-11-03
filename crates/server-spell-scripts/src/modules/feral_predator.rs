//! Predator increases eligible generator direct damage by a share of melee haste.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraStatMods, spell_mods};
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use combat::stats::CharacterStats;

const PREDATOR: u32 = 449864;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::TargetContextDamageMultiplier(multiplier)];

fn multiplier(world: &World, ctx: &EffectCtx<'_>, _: Entity, periodic: bool) -> f32 {
    if periodic {
        return 1.0;
    }
    let Some(aura) = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == PREDATOR && aura.caster == ctx.caster)
    }) else {
        return 1.0;
    };
    let Some(mask) = aura.effects.iter().find(|effect| effect.index == 0) else {
        return 1.0;
    };
    if !spell_mods::class_mask_affects(aura, mask, ctx.spell) {
        return 1.0;
    }
    let haste = world
        .get::<CharacterStats>(ctx.caster)
        .expect("Predator caster stats")
        .haste_pct;
    let speed = world.get::<AuraStatMods>(ctx.caster).map_or(1.0, |mods| {
        mods.attack_cast_time_mult * mods.attack_time_mult * mods.melee_ranged_attack_time_mult
    });
    let melee_haste = (1.0 + haste / 100.0) / speed - 1.0;
    let percent = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, PREDATOR, 1);
    // Retail 70448 spelltext202021; SimC76f8118 sc_druid.cpp11293–11301.
    1.0 + melee_haste * percent / 100.0
}
