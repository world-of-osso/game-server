//! SimC76f8118 sc_rogue.cpp1891–1894/2719–2721/8469–8479:
//! a landed combo-point finisher refunds native Energy per effective point.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::PowerType;

const TALENT: u32 = 58423;
const ENERGIZE: u32 = 98440;
const ROGUE_FAMILY: u32 = 8;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PolicyContextRequired(needs_context),
    SpellScriptHook::BeforePolicyPrimaryHit(refund_on_hit),
];

fn selected(world: &World, caster: Entity) -> bool {
    has_talent(world, caster, TALENT)
        || world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(TALENT))
        || auras::has_aura(world, caster, TALENT)
}

fn needs_context(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    let rogue = spell
        .class_options
        .as_ref()
        .is_some_and(|family| family.spell_class_set == ROGUE_FAMILY);
    let spender = spell
        .power_costs
        .iter()
        .any(|cost| cost.power_type == PowerType::ComboPoints as i32);
    rogue && spender && selected(world, caster)
}

fn refund_on_hit(world: &mut World, ctx: &EffectCtx, _target: Entity) {
    if ctx.depth != 0 || !needs_context(world, ctx.caster, ctx.spell) {
        return;
    }
    let Some(points) = ctx.spell.cast_policy_context.as_ref().and_then(|context| {
        context
            .resources
            .iter()
            .find_map(|(power, amount)| (*power == PowerType::ComboPoints).then_some(*amount))
    }) else {
        return;
    };
    if points <= 0.0 {
        return;
    }
    let per_point = script_effect_value(world, ctx.caster, ENERGIZE, 0);
    let spell = world
        .resource::<SpellStore>()
        .get(ENERGIZE)
        .expect("native Relentless Strikes energize")
        .clone();
    let refund = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: ctx.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    // The primary-hit policy runs once after a successful landed action,
    // including AoE and free finishers with effective pre-payment points.
    effects::energize_unit(
        world,
        &refund,
        ctx.caster,
        PowerType::Energy,
        points * per_point,
        false,
    );
}
