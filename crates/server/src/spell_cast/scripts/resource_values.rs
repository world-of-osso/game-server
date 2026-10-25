//! Source-qualified effective resource values, separate from actual payment.
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason, spell_mods};
use crate::spell_info::SpellInfo;
use bevy::prelude::*;
use shared::components::PowerType;

struct MaximumValueRule {
    aura: u32,
    resource: PowerType,
    match_effect: u8,
}

const RULES: &[MaximumValueRule] = &[
    // SimC76f8118 sc_rogue.cpp2021–2027 uses consume_cp_max for an action
    // affected by Fang Strike;1926 consumes that one use after execution.
    // The native aura effect supplies the affected family mask, not an action-ID list.
    MaximumValueRule {
        aura: 1301405,
        resource: PowerType::ComboPoints,
        match_effect: 0,
    },
];

fn matches(world: &World, caster: Entity, spell: &SpellInfo, rule: &MaximumValueRule) -> bool {
    if !spell
        .power_costs
        .iter()
        .any(|cost| cost.power_type == rule.resource as i32)
    {
        return false;
    }
    let now = super::super::now(world);
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == rule.aura
                && aura.caster == caster
                && aura.expires_at.is_none_or(|expiry| expiry > now)
                && affects(aura, spell, rule.match_effect)
        })
    })
}

fn affects(aura: &AuraInstance, spell: &SpellInfo, index: u8) -> bool {
    aura.effects
        .iter()
        .any(|effect| effect.index == index && spell_mods::class_mask_affects(aura, effect, spell))
}

pub(in crate::spell_cast) fn effective_value(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    resource: PowerType,
    paid: f32,
) -> f32 {
    let maximum = RULES
        .iter()
        .any(|rule| rule.resource == resource && matches(world, caster, spell, rule));
    if maximum {
        return super::super::powers::power_value(world, caster, resource)
            .expect("effective maximum resource requires the native pool")
            .1;
    }
    paid
}

pub(in crate::spell_cast) fn consume_sources(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let sources: Vec<_> = RULES
        .iter()
        .filter(|rule| matches(world, caster, spell, rule))
        .map(|rule| rule.aura)
        .collect();
    for source in sources {
        crate::auras::remove_aura(world, caster, source, RemoveReason::Consumed);
    }
}
