//! Feral Primal Fury grants one combo point for a critical builder cast.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::{ActiveSpec, PowerType};

const TALENT: u32 = 159286;
const ENERGIZE: u32 = 16953;
const RAKE: u32 = 1822;
const ENERGIZE_EFFECT: u32 = 30;

#[derive(Component)]
struct CriticalBuilder(u32);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::ResolvedDamage(record_critical_builder),
    SpellScriptHook::AfterResolvedHits(award_critical_builder),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TALENT).or_default().check_proc = Some(non_feral_only);
}

fn non_feral_only(world: &World, aura: &AuraInstance, _: &AuraEvent) -> bool {
    !world
        .get::<ActiveSpec>(aura.caster)
        .is_some_and(|spec| spec.0 == 103)
}

// SimC76f8118 sc_druid.cpp3728–3759: one reward per critical foreground
// combo generator; Rake specifically requires the primary hit to crit (4607–4613).
fn record_critical_builder(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
    amount: f32,
) {
    if ctx.depth != 0
        || !critical
        || amount <= 0.0
        || !auras::has_aura(world, ctx.caster, TALENT)
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 103)
        || (ctx.spell.id == RAKE && ctx.spell.cast_primary_target != Some(target))
        || !ctx.spell.effects.iter().any(|effect| {
            effect.effect == ENERGIZE_EFFECT
                && effect.misc_values[0] == PowerType::ComboPoints as i32
                && effect.base_points > 0.0
        })
    {
        return;
    }
    world
        .entity_mut(ctx.caster)
        .insert(CriticalBuilder(ctx.spell.id));
}

fn award_critical_builder(world: &mut World, ctx: &EffectCtx<'_>) {
    if ctx.depth != 0
        || !world
            .get::<CriticalBuilder>(ctx.caster)
            .is_some_and(|state| state.0 == ctx.spell.id)
    {
        return;
    }
    world.entity_mut(ctx.caster).remove::<CriticalBuilder>();
    spell_cast::cast_triggered(world, ctx.caster, ENERGIZE, Some(ctx.caster), 1);
}
