//! Coil of Devastation deposits actual Coil damage into its native rolling DoT.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value,
};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(coil_impact)];

fn coil_impact(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, amount: f32) {
    if !matches!(ctx.spell.id, 47632 | 1242172 | 1242178)
        || amount <= 0.0
        || !combat::auras::has_aura(world, ctx.caster, 390270)
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp9803–9810/9852–9854: deposit a
    // selected native percentage of the real hit into the rolling residual.
    let ticks = {
        let dot = world
            .resource::<SpellStore>()
            .get(390271)
            .expect("native Coil of Devastation DoT");
        let duration = dot
            .misc
            .as_ref()
            .and_then(|misc| misc.duration)
            .expect("native Coil duration")
            .duration_ms;
        let period = dot
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Coil periodic effect")
            .aura_period_ms;
        assert!(
            duration > 0 && period > 0,
            "native Coil duration and period must be positive"
        );
        duration as f32 / period as f32
    };
    let fraction = script_effect_value(world, ctx.caster, 390270, 0) / 100.0;
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        390271,
        Some(target),
        ctx.depth + 1,
        amount * fraction / ticks,
    );
}
