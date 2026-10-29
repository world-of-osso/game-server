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
    SpellScriptHook::AfterResolvedHits(grow_on_cast),
];

fn grow_on_cast(world: &mut World, ctx: &EffectCtx<'_>) {
    use combat::random::seq::SliceRandom;
    const TALENT: u32 = 440_118;
    if !matches!(ctx.spell.id, 18_562 | 48_438)
        || !combat::auras::has_aura(world, ctx.caster, TALENT)
    {
        return;
    }
    let Some(targets) = combat::spell_cast::resolving_targets(world, ctx) else {
        return;
    };
    let candidates: Vec<_> = targets
        .iter()
        .copied()
        .filter(|target| {
            combat::spell_cast::targets::relation_matches(
                world,
                ctx.caster,
                *target,
                combat::spell_cast::targets::Relation::Ally,
            )
        })
        .collect();
    let Some(&target) = candidates.choose(&mut combat::random::thread_rng()) else {
        return;
    };
    let duration = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 1)
        .max(0.0) as u32;
    combat::spell_cast::cast_triggered_with_overrides(
        world,
        ctx.caster,
        BLOOM,
        Some(target),
        ctx.depth + 1,
        combat::spell_cast::TriggeredSpellOverrides {
            fixed_aura_duration_ms: Some(duration),
            ..Default::default()
        },
    );
}

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
