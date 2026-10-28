//! Divine Purpose spender procs from selected native talents.
//! SimC76f8118 sc_paladin.hpp1721–1726 and sc_paladin.cpp4342 select
//! 408458 for Ret and 223819 for Protection; native talents do not link the buffs.
use bevy::prelude::*;
use shared::components::PowerType;

use super::combat;
use combat::auras::ActiveAuras;
use combat::class_progression::TraitLoadout;
use combat::random as rand;
use combat::spell_cast::CostPool;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterSpenderStacks(
    roll_selected_divine_purpose,
)];

const PURPOSE: &[(u32, u32)] = &[(408459, 408458), (223817, 223819)];

fn selected_purpose(world: &World, caster: Entity) -> Option<(f32, u32)> {
    let traits = world.get::<TraitLoadout>(caster)?;
    let auras = world.get::<ActiveAuras>(caster)?;
    for &(talent, buff) in PURPOSE {
        if traits.active_talent_spells().any(|spell| spell.spell_id == talent) {
            let aura = auras.iter().find(|aura| aura.spell_id == talent)?;
            let chance = aura.effects.iter().find(|effect| effect.index == 0)
                .expect("Divine Purpose talent retains chance effect0").amount;
            return Some((chance, buff));
        }
    }
    None
}

fn roll_selected_divine_purpose(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some((chance, buff)) = selected_purpose(world, caster) else {
        return;
    };
    let costs = match combat::spell_cast::applicable_costs(world, caster, spell) {
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
    if !combat::spell_cast::try_cast_triggered(world, caster, buff, Some(caster), 1) {
        warn!(?caster, buff, "native Divine Purpose buff refused");
    }
}
