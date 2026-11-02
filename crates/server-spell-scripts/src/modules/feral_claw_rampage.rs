//! Selected Feral Claw Rampage earns Ravage on Berserk melee impacts.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::random::Rng;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

const TALENT: u32 = 441835;
const RAVAGE: u32 = 441585;
const BERSERK: u32 = 106951;
const INCARNATION: u32 = 102543;

#[derive(Component, Default)]
struct ClawRampageReady(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(owns_proc),
    SpellScriptHook::AfterPolicyDirectDamage(after_impact),
];

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

// SimC76f8118 sc_druid.cpp2215–2255,4780–4786,4833–4835:
// Shred and Cat Swipe impacts roll native chance and ICD while Berserk is up.
fn after_impact(world: &mut World, ctx: &EffectCtx<'_>, _: Entity, _: bool) {
    if !matches!(ctx.spell.id, 5221 | 106785 | 202028)
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 103)
    {
        return;
    }
    let Some(auras) = world.get::<ActiveAuras>(ctx.caster) else {
        return;
    };
    if !auras
        .iter()
        .any(|a| matches!(a.spell_id, BERSERK | INCARNATION) && a.caster == ctx.caster)
    {
        return;
    }
    let Some(chance) = auras
        .iter()
        .find(|a| a.spell_id == TALENT && a.caster == ctx.caster)
        .and_then(|a| a.effects.iter().find(|effect| effect.index == 0))
        .map(|effect| effect.amount)
    else {
        return;
    };
    let now = spell_cast::now(world);
    if world
        .get::<ClawRampageReady>(ctx.caster)
        .is_some_and(|state| now < state.0)
    {
        return;
    }
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance {
        return;
    }
    let cooldown = world
        .resource::<combat::spell_info::SpellStore>()
        .get(TALENT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Claw Rampage proc options")
        .proc_category_recovery_ms as f64
        / 1000.0;
    world
        .entity_mut(ctx.caster)
        .insert(ClawRampageReady(now + cooldown));
    spell_cast::cast_triggered(world, ctx.caster, RAVAGE, Some(ctx.caster), ctx.depth + 1);
}
