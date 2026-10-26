//! Native Gravedigger finisher relationships.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{effects::EffectCtx, queued_triggers, script_registry::SpellScriptHook};
use shared::components::PowerType;

const DISPATCH: u32 = 2098;
const GRAVEDIGGER_TWO: u32 = 1265862;
const SCOUNDREL_STRIKE: u32 = 1265918;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResourceOnlyContext(&[DISPATCH]),
    SpellScriptHook::ResolvedDamage(after_hit),
];

fn after_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if ctx.spell.id != DISPATCH
        || ctx.depth != 0
        || damage <= 0.0
        || !auras::has_aura(world, ctx.caster, GRAVEDIGGER_TWO)
    {
        return;
    }
    let points = ctx
        .spell
        .cast_policy_context
        .as_ref()
        .expect("native Dispatch retains effective points")
        .resources
        .iter()
        .find_map(|(power, amount)| (*power == PowerType::ComboPoints).then_some(*amount))
        .expect("native Dispatch has combo-point context");
    let threshold =
        combat::spell_cast::scripts::script_effect_value(world, ctx.caster, GRAVEDIGGER_TWO, 0);
    if points >= threshold {
        // SimC76f8118 sc_rogue.cpp3794/8879–8893: qualifying landed
        // Dispatch launches the native child at its original target after 200ms.
        queued_triggers::schedule_full_trigger(world, ctx, target, SCOUNDREL_STRIKE, 200);
    }
}
