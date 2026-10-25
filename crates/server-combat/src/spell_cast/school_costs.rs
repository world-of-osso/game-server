//! TC a352b1fa SpellInfo.cpp:4185 school costs and4242 global mana multiplier.

/// Aura423 changes only mana, after school and SpellModOp cost modifiers.
/// TC truncates the scaled cost to integer before its existing sign check.
pub(super) fn modify_mana_cost(world: &World, caster: Entity, amount: f32) -> f32 {
    use crate::auras::{ActiveAuras, handlers::MOD_MANA_COST_PCT};
    world
        .get::<ActiveAuras>(caster)
        .filter(|auras| auras.has_type(MOD_MANA_COST_PCT))
        .map_or(amount, |auras| {
            (amount * (1.0 + auras.modifier(MOD_MANA_COST_PCT) / 100.0)).trunc()
        })
}
use super::*;

pub(super) fn modify_school_cost(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    power: i32,
    amount: f32,
) -> f32 {
    let Some(power_mask) = u32::try_from(power)
        .ok()
        .and_then(|power| 1u32.checked_shl(power))
    else {
        return amount;
    };
    let Some(auras) = world.get::<crate::auras::ActiveAuras>(caster) else {
        return amount;
    };
    auras.iter().fold(amount, |amount, aura| {
        aura.effects
            .iter()
            .filter(|effect| {
                effect.aura_type == crate::auras::handlers::MOD_POWER_COST_SCHOOL_PCT
                    && effect.misc_values[0] as u32 & spell.school_mask() != 0
                    && effect.misc_values[1] as u32 & power_mask != 0
            })
            .fold(amount, |amount, effect| {
                amount * (1.0 + effect.amount * aura.stack_multiplier() / 100.0)
            })
    })
}
