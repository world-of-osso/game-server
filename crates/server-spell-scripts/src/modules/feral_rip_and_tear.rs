//! Rip and Tear snapshots the remaining source-side Rip damage into Tear.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, periodic_live_amount};
use combat::spell_cast::{
    self,
    effects::{DamageInput, EffectCtx},
    script_registry::{ContextHitHook, SpellScriptHook},
};
use combat::spell_info::SpellStore;

const RIP: u32 = 1079;
const TALENT: u32 = 391347;
const TEAR: u32 = 391356;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ContextHit(hit)];

fn hit(spell: u32) -> Option<ContextHitHook> {
    (spell == RIP).then_some(apply_tear)
}

// SimC76f8118 sc_druid.cpp4640–4647,4707–4720: use the newly applied
// Rip's fractional remaining tick count and unmitigated, non-crit source tick.
fn apply_tear(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let Some(percent) = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|a| a.spell_id == TALENT && a.caster == ctx.caster)
            .and_then(|a| a.effects.iter().find(|effect| effect.index == 0))
            .map(|effect| effect.amount)
    }) else {
        return;
    };
    let Some(rip) = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|a| a.spell_id == RIP && a.caster == ctx.caster)
    }) else {
        return;
    };
    let effect = rip
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.period_ms > 0)
        .expect("native Rip periodic effect");
    let expiry = rip.expires_at.expect("native Rip duration");
    let period = f64::from(effect.period_ms) / 1000.0;
    let ticks = (1.0 + (expiry - effect.next_tick_at()) / period).max(0.0) as f32;
    let native = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Rip effect");
    let amount = periodic_live_amount(world, ctx.caster, ctx.spell, effect);
    let source = combat::spell_cast::effects::damage_source_amount(
        world,
        ctx,
        target,
        &DamageInput {
            amount,
            crit: false,
            periodic: true,
            resolved_source: false,
            effect: Some(native),
            bonus_caster: ctx.caster,
        },
    );
    let tear = world
        .resource::<SpellStore>()
        .get(TEAR)
        .expect("native Tear");
    let interval = tear
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Tear tick")
        .aura_period_ms;
    let duration = tear
        .misc
        .as_ref()
        .and_then(|misc| misc.duration.as_ref())
        .expect("native Tear duration")
        .duration_ms;
    let per_tick = source * ticks * (percent / 100.0) * interval as f32 / duration as f32;
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        TEAR,
        Some(target),
        ctx.depth + 1,
        per_tick,
    );
}
