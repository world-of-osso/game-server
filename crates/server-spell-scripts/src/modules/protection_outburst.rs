//! Protection's paid Rage earns and spends Violent Outburst.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::{self, CostPool, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::{ActiveSpec, PowerType};

const TALENT: u32 = 386477;
const SEEING_RED: u32 = 386486;
const OUTBURST: u32 = 386478;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterPayment(track_rage),
    SpellScriptHook::AuraRollAfterCast(shield_slam_hook),
];

fn track_rage(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 73)
        || !auras::has_aura(world, caster, TALENT)
    {
        return;
    }
    let paid = spell_cast::spent(costs, PowerType::Rage);
    if paid <= 0.0 {
        return;
    }
    let units = world
        .resource::<spell_cast::powers::PowerTypeTable>()
        .get(PowerType::Rage)
        .expect("native Rage units")
        .display_modifier;
    let threshold = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    assert!(
        units > 0 && threshold > 0.0,
        "Violent Outburst requires native Rage threshold"
    );
    let old = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SEEING_RED && aura.caster == caster)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    let total = old + paid / units as f32;
    // SimC76f8118 sc_warrior.cpp1480–1502 preserves excess paid Rage.
    if total >= threshold {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == SEEING_RED && aura.caster == caster
        });
        spell_cast::cast_triggered(world, caster, OUTBURST, Some(caster), 1);
    }
    let remainder = if total >= threshold {
        total - threshold
    } else {
        total
    };
    if remainder > 0.0 {
        if !auras::has_aura(world, caster, SEEING_RED) {
            spell_cast::cast_triggered(world, caster, SEEING_RED, Some(caster), 1);
        }
        let instance = world
            .get::<ActiveAuras>(caster)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == SEEING_RED && aura.caster == caster)
            })
            .expect("Seeing Red application")
            .instance_id;
        auras::set_effect_amount(world, caster, instance, 0, remainder);
    }
}

fn shield_slam_hook(spell: u32) -> Option<spell_cast::scripts::AfterCastHook> {
    (spell == 23922).then_some(spend_outburst)
}

fn spend_outburst(world: &mut World, caster: Entity, _: &SpellInfo) {
    // SimC76f8118 sc_warrior.cpp6282–6286: Shield Slam spends the earned
    // buff and grants a free native Ignore Pain, including its normal cap.
    if !auras::has_aura(world, caster, OUTBURST) {
        return;
    }
    spell_cast::cast_triggered(world, caster, 190456, Some(caster), 1);
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == OUTBURST && aura.caster == caster
    });
}
