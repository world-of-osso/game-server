//! Sudden Ambush rolls its native per-combo-point chance on landed finishers.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

const TALENT: u32 = 384667;
const BUFF: u32 = 1244483;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::PolicyContextRequired(needs_context),
    SpellScriptHook::BeforePolicyPrimaryHit(roll_on_hit),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(TALENT).or_default().check_proc = Some(block_generic_proc);
}

fn block_generic_proc(_: &World, _: &AuraInstance, _: &AuraEvent) -> bool {
    false
}

fn needs_context(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    if !auras::has_aura(world, caster, TALENT)
        || !spell
            .power_costs
            .iter()
            .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
    {
        return false;
    }
    world
        .resource::<SpellStore>()
        .get(TALENT)
        .and_then(|source| source.aura_options.as_ref())
        .and_then(|options| options.proc_family)
        .is_some_and(|(family, mask)| auras::procs::matches_explicit_family(spell, family, mask))
}

fn roll_on_hit(world: &mut World, ctx: &EffectCtx<'_>, _: Entity) {
    let Some(context) = ctx.spell.cast_policy_context.as_ref() else {
        return;
    };
    if (ctx.depth != 0 && !context.free_spender) || !needs_context(world, ctx.caster, ctx.spell) {
        return;
    }
    let Some(points) = context
        .resources
        .iter()
        .find_map(|(power, points)| (*power == PowerType::ComboPoints).then_some(*points))
    else {
        return;
    };
    let chance = spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0) * points;
    // SimC76f8118 sc_druid.cpp3817–3844; WCL g1dGBjmAtcHYXaM9 fight15
    // also records grants from free Convoke Bites with retained effective points.
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, ctx.caster, BUFF, Some(ctx.caster), ctx.depth + 1);
    }
}
