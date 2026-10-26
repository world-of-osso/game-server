//! Mortal Wounds is distinct from the shared Execute-only Deep Wounds passive.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterPolicyDirectDamage(
    apply_critical_mortal_wounds,
)];

const MORTAL_WOUNDS: u32 = 1261062;
const DEEP_WOUNDS: u32 = 262115;
const CRITICAL_PRODUCERS: &[u32] = &[12294, 1464];

fn apply_critical_mortal_wounds(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
) {
    // SimC76f8118 sc_warrior.cpp3153–3154/3518–3519: selected Mortal
    // Wounds applies native Deep Wounds only on Mortal Strike/Slam crits,
    // including background Unhinged Mortal Strikes. Execute retains its
    // separate native1261060 producer (cpp4248–4249); Fury is unchanged.
    let eligible_hit = critical && CRITICAL_PRODUCERS.contains(&ctx.spell.id);
    if !eligible_hit {
        return;
    }
    let arms = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 71);
    if !arms || !combat::auras::has_aura(world, ctx.caster, MORTAL_WOUNDS) {
        return;
    }
    if !combat::spell_cast::try_cast_triggered(
        world,
        ctx.caster,
        DEEP_WOUNDS,
        Some(target),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, ?target, "Native Arms Mortal Wounds payload refused");
    }
}
