//! Selected Crusade1253598: SimC76f8118 sc_paladin.cpp3644–3651,4617–4618,4647–4648.
use super::{ActiveAuras, AuraInstance};
use crate::class_progression::TraitLoadout;
use crate::spell_cast::CostPool;
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

const CRUSADE: u32 = 1253598;
const WRATH: u32 = 31884;
const MAX_STACKS: u8 = 10;

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 70)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == CRUSADE)
        })
}

pub(super) fn maximum_stacks(world: &World, caster: Entity, spell: &SpellInfo) -> Option<u8> {
    (spell.id == WRATH && selected(world, caster)).then_some(MAX_STACKS)
}

/// sc_paladin.cpp3911 parses base Wrath effects with IGNORE_STACKS for every spec.
pub(super) fn ignores_effect_stacks(spell: u32) -> bool {
    spell == WRATH
}

pub(super) fn preserve_refresh_duration(world: &World, target: Entity, new: &mut AuraInstance) {
    if new.spell_id != WRATH || !selected(world, new.caster) {
        return;
    }
    let previous = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras.iter().find(|old| {
            old.spell_id == WRATH && old.caster == new.caster && old.source == new.source
        })
    });
    if let Some(previous) = previous {
        // Buff refresh is disabled; reapplication may add stacks, never time.
        new.duration_ms = previous.duration_ms;
        new.expires_at = previous.expires_at;
    }
}

/// Pure aura-state fold: native passive effect0 owns haste per actual Wrath stack.
pub(super) fn haste_total_pct(auras: &ActiveAuras) -> f32 {
    let Some(wrath) = auras.iter().find(|aura| aura.spell_id == WRATH) else {
        return 0.0;
    };
    let Some(crusade) = auras
        .iter()
        .find(|aura| aura.spell_id == CRUSADE && aura.caster == wrath.caster)
    else {
        return 0.0;
    };
    let per_stack = crusade
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .map_or(0.0, |effect| effect.amount);
    per_stack * f32::from(wrath.stacks)
}

/// hpp1541–1555/1614–1623: actual paid cost, or the free consumer's native base
/// HP equivalent. Triggered Legacy/echo/Tempest actions never call this observer.
pub(crate) fn after_spender(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    costs: &[(CostPool, f32)],
) {
    if !selected(world, caster) {
        return;
    }
    let gain = spender_stack_gain(spell, costs);
    if gain == 0 {
        return;
    }
    let Some(mut auras) = world.get_mut::<ActiveAuras>(caster) else {
        return;
    };
    let Some(wrath) = auras
        .auras
        .iter_mut()
        .find(|aura| aura.spell_id == WRATH && aura.caster == caster)
    else {
        return;
    };
    wrath.stacks = wrath.stacks.saturating_add(gain).min(MAX_STACKS);
    auras.dirty = true;
    super::auras_changed(world, caster);
}

fn spender_stack_gain(spell: &SpellInfo, costs: &[(CostPool, f32)]) -> u8 {
    if spell.id == 427453 {
        // hpp1618–1622: the pinned reference's Hammer of Light always grants9.
        return 9;
    }
    if !matches!(spell.id, 85256 | 383328 | 53385 | 85673) {
        return 0;
    }
    let paid: f32 = costs
        .iter()
        .filter_map(|(pool, amount)| {
            (*pool == CostPool::Power(PowerType::HolyPower)).then_some(*amount)
        })
        .sum();
    if paid > 0.0 {
        return paid as u8;
    }
    let free =
        spell
            .cast_charge_mods
            .iter()
            .flatten()
            .any(|modifier| match modifier.aura_spell_id {
                408458 => true,
                326733 => spell.id == 53385,
                327510 => spell.id == 85673,
                _ => false,
            });
    if !free {
        return 0;
    }
    spell
        .power_costs
        .iter()
        .find(|cost| cost.power_type == PowerType::HolyPower as i32)
        .expect("Crusade consumer requires native Holy Power cost")
        .mana_cost
        .max(0) as u8
}
