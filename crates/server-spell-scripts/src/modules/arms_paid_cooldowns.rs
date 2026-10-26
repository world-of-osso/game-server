//! Arms Anger Management follows actual paid Rage, not effective free-cast value.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use super::combat;
use combat::spell_cast::CostPool;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterSpellPayment(
    reduce_paid_arms_cooldowns,
)];

const ANGER_MANAGEMENT: u32 = 152278;
const IGNORE_PAIN: u32 = 190456;

fn reduce_paid_arms_cooldowns(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    costs: &[(CostPool, f32)],
) {
    // SimC76f8118 sc_warrior.cpp1450–1455/1519–1527: Ignore Pain is
    // excluded for Arms; native effect1 divides actual displayed Rage spent.
    if world.get::<ActiveSpec>(caster).is_none_or(|spec| spec.0 != 71)
        || spell.id == IGNORE_PAIN
        || !combat::auras::has_aura(world, caster, ANGER_MANAGEMENT)
    {
        return;
    }
    let paid = combat::spell_cast::spent(costs, PowerType::Rage);
    if paid <= 0.0 {
        return;
    }
    let seconds = read_reduction_seconds(world, caster, paid);
    for base in [107574, 167105] {
        let action = combat::auras::overrides::resolve_spell(world, caster, base);
        combat::spell_cast::scripts::reduce_cooldown(world, caster, action, seconds);
    }
}

fn read_reduction_seconds(world: &World, caster: Entity, paid: f32) -> f64 {
    let divisor = script_effect_value(world, caster, ANGER_MANAGEMENT, 0);
    let units = world
        .resource::<combat::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::Rage)
        .expect("native Rage display units")
        .display_modifier;
    assert!(divisor > 0.0 && units > 0, "native Anger Management divisors must be positive");
    f64::from(paid) / f64::from(units) / f64::from(divisor)
}
