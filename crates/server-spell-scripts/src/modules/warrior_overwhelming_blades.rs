//! Overwhelming Blades grants target-owned Overwhelmed from real impacts.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterPolicyDirectDamage(apply_overwhelmed)];

fn apply_overwhelmed(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    const OVERWHELMING_BLADES: u32 = 444772;
    const REAP_THE_STORM: u32 = 444775;
    const OVERWHELMED: u32 = 445836;
    if !combat::auras::has_aura(world, ctx.caster, OVERWHELMING_BLADES)
        || !(ctx.spell.id == 50622
            || (ctx.spell.id == 446005
                && combat::auras::has_aura(world, ctx.caster, REAP_THE_STORM)))
    {
        return;
    }
    // SimC76f8118 sc_warrior.cpp:1857–1859,3214–3216: each Reap or
    // main-hand Bladestorm impact adds one native Overwhelmed stack.
    if !combat::spell_cast::try_cast_triggered(
        world,
        ctx.caster,
        OVERWHELMED,
        Some(target),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, ?target, "Overwhelming Blades payload refused");
    }
}
