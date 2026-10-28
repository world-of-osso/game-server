//! Midnight two-piece Magus damage receives the four-piece execute bonus.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature,
};
use shared::components::Health;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetContextDamageMultiplier(
    execute_bonus,
)];

fn execute_bonus(world: &World, ctx: &EffectCtx<'_>, target: Entity, periodic: bool) -> f32 {
    let effect = match (ctx.spell.id, periodic) {
        (1297055 | 1297086, false) => 0,
        (1297091, true) => 1,
        _ => return 1.0,
    };
    let Some(summon) = world.get::<SummonedCreature>(ctx.caster) else {
        return 1.0;
    };
    if !combat::auras::has_aura(world, summon.owner, 1296655) {
        return 1.0;
    }
    let Some(health) = world.get::<Health>(target) else {
        return 1.0;
    };
    let threshold =
        combat::spell_cast::scripts::script_effect_value(world, summon.owner, 1296655, 2);
    if health.current * 100.0 > health.max * threshold {
        return 1.0;
    }
    // SimC76f8118 sc_death_knight.cpp4456–4464/4513–4519: the selected
    // four-piece applies its separate native bonuses at or below effect3 health.
    1.0 + combat::spell_cast::scripts::script_effect_value(world, summon.owner, 1296655, effect)
        / 100.0
}
