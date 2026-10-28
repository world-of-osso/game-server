//! Unholy rune spenders can earn native Rune Mastery Strength.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeNativeOnCast(on_cast)];

fn on_cast(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
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
    // SimC76f8118 sc_death_knight.cpp13081–13098/15831–15833 uses
    // the observed 15% chance once per rune-cost action, including free casts.
    if spends_runes
        && combat::random::random::<f32>() < 0.15
        && !spell_cast::try_cast_triggered(world, caster, 374585, Some(caster), 1)
    {
        warn!(?caster, "native Rune Mastery buff refused");
    }
}
