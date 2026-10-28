//! Reaping's selected execute bonus and Dark Transformation buff.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::EffectCtx,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::SpellInfo;
use shared::components::Health;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::TargetContextDamageMultiplier(execute_bonus),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 1233448).then_some(apply_reaping)
}

fn apply_reaping(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, 377514) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp8776–8778: Dark Transformation
    // grants the selected native Reaping buff.
    combat::auras::apply_aura(
        world,
        &combat::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: 1242654,
            effect_mask: 1,
            source: combat::auras::AuraSource::Cast,
        },
    );
}

fn execute_bonus(world: &World, ctx: &EffectCtx<'_>, target: Entity, _: bool) -> f32 {
    // SimC76f8118 sc_death_knight.cpp9821–9833/10427–10430/12549–12552:
    // Reaping uses the live recipient's health for these actual damage actions.
    if !matches!(
        ctx.spell.id,
        47632 | 1242172 | 1242178 | 85948 | 458128 | 55090
    ) || !combat::auras::has_aura(world, ctx.caster, 377514)
    {
        return 1.0;
    }
    let Some(health) = world.get::<Health>(target) else {
        return 1.0;
    };
    let threshold = script_effect_value(world, ctx.caster, 377514, 1);
    if health.current * 100.0 >= health.max * threshold {
        return 1.0;
    }
    1.0 + script_effect_value(world, ctx.caster, 377514, 0) / 100.0
}
