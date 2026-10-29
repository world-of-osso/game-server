//! Thunderlord reduces Demoralizing Shout recovery for actual Thunder Clap hits.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResolvedHits(reduce_shout)];

fn reduce_shout(world: &mut World, ctx: &EffectCtx<'_>) {
    if !matches!(ctx.spell.id, 6343 | 435222)
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 73)
        || !combat::auras::has_aura(world, ctx.caster, 385840)
    {
        return;
    }
    let hits = spell_cast::resolving_targets(world, ctx).map_or(0, |targets| {
        targets
            .iter()
            .filter(|&&target| {
                spell_cast::targets::relation_matches(
                    world,
                    ctx.caster,
                    target,
                    spell_cast::targets::Relation::Enemy,
                )
            })
            .count()
    });
    let cap = spell_cast::scripts::script_effect_value(world, ctx.caster, 385840, 1) as usize;
    let millis = spell_cast::scripts::script_effect_value(world, ctx.caster, 385840, 0);
    // SimC76f8118 sc_warrior.cpp4003–4012,4140–4147 counts selected enemies, including absorbed hits.
    spell_cast::scripts::reduce_cooldown(
        world,
        ctx.caster,
        1160,
        f64::from(millis) * hits.min(cap) as f64 / 1000.0,
    );
}
