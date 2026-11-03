//! Sentinel stack initialization and decay from SimC76f8118.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{CostPool, script_registry::SpellScriptHook};
use shared::components::PowerType;

const SENTINEL: u32 = 389539;
const RIGHTEOUS_PROTECTOR: u32 = 204074;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::AfterAuraApplication(initialize),
    SpellScriptHook::AfterSpellPayment(delay_on_payment),
];

#[derive(Component)]
struct SentinelDecay {
    instance: u32,
    due: f64,
    cooldown_until: f64,
    cooldown: f64,
}

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(SENTINEL).or_default();
    script.on_periodic = Some(decay);
    script.on_remove = Some(remove);
}

// SimC76f8118 sc_paladin_protection.cpp:16–35,48–59,736–749:
// Sentinel starts at its native maximum; its separate decay timer starts after
// max(native modified duration − 15s, 1s), with its own native-effect cooldown.
fn initialize(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.spell_id != SENTINEL || aura.caster != unit {
        return;
    }
    assert!(auras::set_owned_stacks(
        world,
        unit,
        unit,
        SENTINEL,
        aura.max_stacks
    ));
    let reduction = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == unit && aura.spell_id == RIGHTEOUS_PROTECTOR)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 1))
        .map_or(0.0, |effect| effect.amount.abs() / 100.0);
    let cooldown = f64::from(combat::spell_cast::scripts::script_effect_value(
        world, unit, SENTINEL, 13,
    )) / 1000.0
        * f64::from(1.0 - reduction);
    let now = auras::aura_now(world);
    world.entity_mut(unit).insert(SentinelDecay {
        instance: aura.instance_id,
        due: now + (f64::from(aura.duration_ms) / 1000.0 - 15.0).max(1.0),
        cooldown_until: now + cooldown,
        cooldown,
    });
}

// sc_paladin.hpp:1651–1657: each paid foreground Holy Power spender
// extends the active decay timer by one second; free spenders do not.
fn delay_on_payment(
    world: &mut World,
    unit: Entity,
    _spell: &combat::spell_info::SpellInfo,
    costs: &[(CostPool, f32)],
) {
    if combat::spell_cast::spent(costs, PowerType::HolyPower) > 0.0 {
        if let Some(mut decay) = world.get_mut::<SentinelDecay>(unit) {
            decay.due += 1.0;
        }
    }
}

// sc_paladin_protection.cpp:772–785: the next 1s timer is triggered only
// when the independent decay-buff cooldown permits it.
fn decay(world: &mut World, unit: Entity, aura: &AuraInstance, effect: u8) {
    if effect != 9 {
        return;
    }
    let now = auras::aura_now(world);
    let Some(state) = world.get::<SentinelDecay>(unit) else {
        return;
    };
    if state.instance != aura.instance_id || now + 0.000_001 < state.due {
        return;
    }
    let can_retrigger = now + 0.000_001 >= state.cooldown_until;
    let cooldown = state.cooldown;
    auras::spend_stack(world, unit, aura.instance_id);
    if can_retrigger && auras::has_aura(world, unit, SENTINEL) {
        if let Some(mut state) = world.get_mut::<SentinelDecay>(unit) {
            state.due = now + 1.0;
            state.cooldown_until = now + cooldown;
        }
    } else {
        world.entity_mut(unit).remove::<SentinelDecay>();
    }
}

fn remove(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    if world
        .get::<SentinelDecay>(unit)
        .is_some_and(|state| state.instance == aura.instance_id)
    {
        world.entity_mut(unit).remove::<SentinelDecay>();
    }
}

#[cfg(test)]
#[path = "../paladin_sentinel_tests.rs"]
mod tests;
