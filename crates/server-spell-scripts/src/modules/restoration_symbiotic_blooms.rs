//! Symbiotic Blooms healing proc; SimC76f8118 marks its RNG NYI.
//! WCL mR86tKbdCqF1A2NY fight4 actor4: 257/7156 isolated eligible
//! heals matched an application within 100ms (95% Wilson CI 3.18–4.05%).
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

const THRIVING_GROWTH: u32 = 439_528;
const BLOOM: u32 = 439_530;
const OBSERVED_CHANCE: f32 = 257.0 / 7156.0;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AsynchronousAuras(&[BLOOM]),
    SpellScriptHook::ResolvedHeal(grow_bloom),
];

fn grow_bloom(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    raw: f32,
    _effective: f32,
    _critical: bool,
    _periodic: bool,
) {
    if raw <= 0.0
        || !matches!(ctx.spell.id, 48438 | 8936 | 81269)
        || !combat::auras::has_aura(world, ctx.caster, THRIVING_GROWTH)
    {
        return;
    }
    if combat::random::random::<f32>() < OBSERVED_CHANCE {
        combat::spell_cast::cast_triggered(world, ctx.caster, BLOOM, Some(target), ctx.depth + 1);
    }
}
