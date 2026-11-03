//! Feral finishers restore energy per effective combo point.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::PowerType;

const TALENT: u32 = 158476;
const DRUID_FAMILY: u32 = 7;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PolicyContextRequired(needs_context),
    SpellScriptHook::BeforePolicyPrimaryHit(restore_energy),
];

fn needs_context(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    auras::has_aura(world, caster, TALENT)
        && spell
            .class_options
            .as_ref()
            .is_some_and(|options| options.spell_class_set == DRUID_FAMILY)
        && spell
            .power_costs
            .iter()
            .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
}

fn restore_energy(world: &mut World, ctx: &EffectCtx<'_>, _: Entity) {
    let Some(context) = ctx.spell.cast_policy_context.as_ref() else {
        return;
    };
    if (ctx.depth != 0 && !context.free_spender) || !needs_context(world, ctx.caster, ctx.spell) {
        return;
    }
    let Some(points) = context
        .resources
        .iter()
        .find_map(|(power, amount)| (*power == PowerType::ComboPoints).then_some(*amount))
    else {
        return;
    };
    let per_point = spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0);
    spell_cast::powers::change_power(world, ctx.caster, PowerType::Energy, per_point * points);
}
