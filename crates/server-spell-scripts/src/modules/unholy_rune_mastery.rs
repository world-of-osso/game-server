//! Blood and Unholy rune actions can earn native Rune Mastery Strength.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, CostPool, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, PowerType};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeNativeOnCast(on_unholy_cast),
    SpellScriptHook::AfterSpellPayment(on_blood_payment),
];

fn on_unholy_cast(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth != 0
        || world
            .get::<ActiveSpec>(caster)
            .is_none_or(|spec| spec.0 != 252)
        || !combat::auras::has_aura(world, caster, 374574)
    {
        return;
    }
    let spends_runes = spell.power_costs.iter().any(|cost| {
        cost.power_type == 5
            && cost.mana_cost > 0
            && (cost.required_aura_spell_id == 0
                || combat::auras::has_aura(world, caster, cost.required_aura_spell_id))
    });
    if spends_runes {
        roll_mastery(world, caster);
    }
}

fn on_blood_payment(world: &mut World, caster: Entity, _: &SpellInfo, costs: &[(CostPool, f32)]) {
    if world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
        && combat::auras::has_aura(world, caster, 374574)
        && costs
            .iter()
            .any(|(pool, _)| *pool == CostPool::Power(PowerType::Runes))
    {
        roll_mastery(world, caster);
    }
}

// SimC76f8118 sc_death_knight.cpp13081–13098/15831–15833 uses
// the observed 15% chance once per rune-cost action, including free casts.
fn roll_mastery(world: &mut World, caster: Entity) {
    if combat::random::random::<f32>() < 0.15
        && !spell_cast::try_cast_triggered(world, caster, 374585, Some(caster), 1)
    {
        warn!(?caster, "native Rune Mastery buff refused");
    }
}
