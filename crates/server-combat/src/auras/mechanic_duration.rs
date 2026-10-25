//! TC a352b1fa Object.cpp1725: target mechanic durations use the stronger negative rule.
use bevy::prelude::*;

use super::{ActiveAuras, AuraApplication, AuraEffectState, attributes, handlers, is_aura_effect};
use crate::spell_info::SpellInfo;

const NO_TARGET_DURATION_MOD: u32 = 0x2;

pub(super) fn positive_source_gap(effect: &AuraEffectState) -> bool {
    effect.aura_type == handlers::MECHANIC_DURATION_MOD && effect.amount > 0.0
}

fn mechanic_bit(mechanic: u32) -> u64 {
    assert!(
        mechanic < u64::BITS,
        "Native mechanic {mechanic} exceeds its 64-bit mask"
    );
    if mechanic == 0 { 0 } else { 1u64 << mechanic }
}

fn applied_mechanic_mask(spell: &SpellInfo, effect_mask: u32) -> u64 {
    let category = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.mechanic);
    spell
        .effects
        .iter()
        .filter(|effect| is_aura_effect(effect) && effect_mask & (1 << effect.index) != 0)
        .fold(mechanic_bit(category), |mask, effect| {
            mask | mechanic_bit(effect.mechanic)
        })
}

pub(super) fn invalid_mechanic_source_gap(effect: &AuraEffectState) -> bool {
    let kind = matches!(
        effect.aura_type,
        handlers::MECHANIC_DURATION_MOD | handlers::MECHANIC_DURATION_MOD_NOT_STACK
    );
    kind && !(0..u64::BITS as i32).contains(&effect.misc_values[0])
}

fn matches_duration_mechanic(effect: &AuraEffectState, mechanic_mask: u64) -> bool {
    if !matches!(
        effect.aura_type,
        handlers::MECHANIC_DURATION_MOD | handlers::MECHANIC_DURATION_MOD_NOT_STACK
    ) {
        return false;
    }
    if invalid_mechanic_source_gap(effect) {
        return false;
    }
    mechanic_mask & mechanic_bit(effect.misc_values[0] as u32) != 0
}

fn matching_duration_modifiers(auras: &ActiveAuras, mechanic_mask: u64) -> i32 {
    let effects = auras
        .iter()
        .flat_map(|aura| {
            aura.effects
                .iter()
                .map(move |effect| (aura.stack_multiplier(), effect))
        })
        .filter(|(_, effect)| matches_duration_mechanic(effect, mechanic_mask));
    let (total, strongest) = effects.fold(
        (0.0_f32, 0.0_f32),
        |(total, strongest), (stacks, effect)| {
            let amount = effect.amount * stacks;
            if effect.aura_type == handlers::MECHANIC_DURATION_MOD {
                (total + amount, strongest)
            } else {
                (total, strongest.min(amount))
            }
        },
    );
    (total as i32).min(strongest as i32)
}

fn matching_dispel_modifier(auras: &ActiveAuras, dispel: u32) -> i32 {
    let total: f32 = auras
        .iter()
        .flat_map(|aura| {
            aura.effects
                .iter()
                .filter(move |effect| {
                    effect.aura_type == handlers::MOD_AURA_DURATION_BY_DISPEL
                        && effect.misc_values[0] == dispel as i32
                })
                .map(move |effect| effect.amount * aura.stack_multiplier())
        })
        .sum();
    // Object.cpp1765 selects min(sum245, strongest negative246); unsupported246
    // remains a named gap rather than acquiring invented positive-duration rules.
    (total as i32).min(0)
}

fn add_percent(duration: u32, percent: i32) -> u32 {
    // Util.h AddPct truncates the integer delta, not the final multiplied duration.
    let delta = (duration as f32 * percent as f32 / 100.0) as i64;
    (i64::from(duration) + delta).max(0) as u32
}

pub(super) fn modify_duration(
    world: &World,
    application: &AuraApplication,
    spell: &SpellInfo,
    duration: u32,
) -> u32 {
    if attributes(spell, 7) & NO_TARGET_DURATION_MOD != 0 {
        return duration;
    }
    if !super::is_harmful(
        world,
        spell,
        application.caster,
        application.target,
        application.effect_mask,
    ) {
        return duration;
    }
    let Some(auras) = world.get::<ActiveAuras>(application.target) else {
        return duration;
    };
    let mask = applied_mechanic_mask(spell, application.effect_mask);
    let percent = matching_duration_modifiers(auras, mask);
    let duration = add_percent(duration, percent);
    let dispel = spell
        .categories
        .as_ref()
        .map_or(0, |category| category.dispel_type);
    add_percent(duration, matching_dispel_modifier(auras, dispel))
}
