//! Nazgrim owns the lifetime and rune-earned strength of Apocalyptic Conquest.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use super::combat;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::{CostPool, script_registry::SpellScriptHook, summon::SummonedCreature};
use combat::spell_info::SpellInfo;

const NAZGRIM: u32 = 444252;
const CONQUEST: u32 = 444763;
const TALENT: u32 = 444052;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonExpired(expire_nazgrim),
    SpellScriptHook::AfterSpellPayment(add_rune_strength),
];

fn expire_nazgrim(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    if summon.spell_id != NAZGRIM {
        return;
    }
    let owner = summon.owner;
    // SimC76f8118 sc_death_knight.cpp5405–5414 expires the owner's buff
    // on the actual Nazgrim's demise, not at a guessed timer.
    auras::remove_auras(world, owner, RemoveReason::Expired, |aura| {
        aura.spell_id == CONQUEST && aura.caster == owner
    });
}

fn add_rune_strength(
    world: &mut World,
    owner: Entity,
    spell: &SpellInfo,
    costs: &[(CostPool, f32)],
) {
    if world
        .get::<ActiveSpec>(owner)
        .is_none_or(|spec| spec.0 != 252)
        || !auras::has_aura(world, owner, TALENT)
    {
        return;
    }
    let Some((instance, current)) = world.get::<ActiveAuras>(owner).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == CONQUEST && aura.caster == owner)
            .and_then(|aura| {
                aura.effects
                    .iter()
                    .find(|effect| effect.index == 0)
                    .map(|effect| (aura.instance_id, effect.amount))
            })
    }) else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp13100–13120 uses Scourge Strike's
    // base rune cost even when free; all other qualifying actions use paid runes.
    let runes = if spell.id == 55090 {
        spell
            .power_costs
            .iter()
            .filter(|cost| cost.power_type == 5)
            .map(|cost| cost.mana_cost as f32)
            .sum()
    } else {
        costs
            .iter()
            .filter_map(|(pool, amount)| {
                (*pool == CostPool::Power(PowerType::Runes)).then_some(*amount)
            })
            .sum()
    };
    if runes > 0.0 {
        let per_rune = combat::spell_cast::scripts::script_effect_value(world, owner, TALENT, 1);
        auras::set_effect_amount(world, owner, instance, 0, current + runes * per_rune);
    }
}
