//! Blade Rush launches its native attack and energy aura.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const BLADE_RUSH: u32 = 271877;
const ATTACK: u32 = 271881;
const ENERGIZE: u32 = 271896;
const BLADE_FLURRY: u32 = 13877;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResolvedHits(launch_blade_rush),
    SpellScriptHook::TargetContextDamageMultiplier(scale_attack),
];

fn launch_blade_rush(world: &mut World, ctx: &EffectCtx<'_>) {
    if ctx.spell.id != BLADE_RUSH || ctx.depth != 0 {
        return;
    }
    let Some(target) = ctx.primary_target() else {
        return;
    };
    // SimC 76f8118 sc_rogue.cpp4008–4051: the cast launches the attack;
    // the attack also starts the energy-regeneration aura.
    if spell_cast::try_cast_triggered(world, ctx.caster, ATTACK, Some(target), ctx.depth + 1) {
        spell_cast::cast_triggered(world, ctx.caster, ENERGIZE, Some(ctx.caster), ctx.depth + 1);
    }
}

fn scale_attack(world: &World, ctx: &EffectCtx<'_>, target: Entity, _: bool) -> f32 {
    if ctx.spell.id != ATTACK {
        return 1.0;
    }
    let multiplier = if ctx.primary_target() == Some(target) {
        spell_cast::scripts::script_effect_value(world, ctx.caster, ATTACK, 1) / 100.0
    } else if auras::has_aura(world, ctx.caster, BLADE_FLURRY) {
        1.0 + spell_cast::scripts::script_effect_value(world, ctx.caster, BLADE_RUSH, 0) / 100.0
    } else {
        1.0
    };
    let count = spell_cast::resolving_targets(world, ctx).map_or(1, |targets| targets.len());
    let cap = spell_cast::scripts::script_effect_value(world, ctx.caster, ATTACK, 2);
    multiplier * (cap / count as f32).min(1.0).sqrt()
}
