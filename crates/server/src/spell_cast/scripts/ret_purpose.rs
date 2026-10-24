//! SIMC_RET_DIVINE_PURPOSE_TRIGGER: selected408459 -> native223819.
//! TC a352b1fa spell_paladin.cpp549–579 supplies HolyPower cost/amount semantics,
//! but registers223817. SimC76f8118 sc_paladin.hpp1721–1726 binds selected408459.
use bevy::prelude::*;
use shared::components::PowerType;

use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::random as rand;
use crate::spell_cast::CostPool;
use crate::spell_info::SpellInfo;

const PURPOSE_TALENT: u32 = 408459;
const DIVINE_PURPOSE: u32 = 223819;

fn selected_chance(world: &World, caster: Entity) -> Option<f32> {
    let selected = world
        .get::<TraitLoadout>(caster)?
        .active_talent_spells()
        .any(|spell| spell.spell_id == PURPOSE_TALENT);
    if !selected {
        return None;
    }
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == PURPOSE_TALENT)?;
    Some(
        aura.effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native408459 retains its chance effect0")
            .amount,
    )
}

pub(in crate::spell_cast) fn roll_selected_ret_divine_purpose(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) {
    let Some(chance) = selected_chance(world, caster) else {
        return;
    };
    let costs = match crate::spell_cast::applicable_costs(world, caster, spell) {
        Ok(costs) => costs,
        Err(reason) => {
            warn!(
                ?caster,
                spell = spell.id,
                ?reason,
                "Ret Divine Purpose cost predicate failed"
            );
            return;
        }
    };
    // TC Spell.cpp8088–8098: pool presence qualifies even at zero final cost.
    let has_holy_power_cost = costs
        .iter()
        .any(|(pool, _, _)| *pool == CostPool::Power(PowerType::HolyPower));
    if !has_holy_power_cost || rand::random::<f32>() * 100.0 >= chance {
        return;
    }
    if !crate::spell_cast::try_cast_triggered(world, caster, DIVINE_PURPOSE, Some(caster), 1) {
        warn!(?caster, "Ret native Divine Purpose223819 refused");
    }
}
