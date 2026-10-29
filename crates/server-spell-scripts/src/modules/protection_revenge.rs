//! Revenge Trigger rolls on white impacts and incoming dodges or parries.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    ActiveAuras, AuraInstance,
    procs::{self, AuraEvent, PROC_HIT_DODGE, PROC_HIT_PARRY},
    scripts::AuraScripts,
};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TRIGGER: u32 = 5301;
const BUFF: u32 = 5302;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TRIGGER),
    SpellScriptHook::LateAuraRegistration(register_frothing),
    SpellScriptHook::AfterSpellPayment(refund_frothing_berserker),
    SpellScriptHook::AfterWhiteImpact(on_white_impact),
    SpellScriptHook::AuraEvent(on_incoming_event),
];

fn register_frothing(scripts: &mut AuraScripts) {
    scripts.0.entry(392792).or_default().check_proc = Some(frothing_generic_proc);
}

fn frothing_generic_proc(world: &World, aura: &AuraInstance, _: &AuraEvent) -> bool {
    world
        .get::<shared::components::ActiveSpec>(aura.caster)
        .is_none_or(|spec| spec.0 != 73)
}

fn refund_frothing_berserker(
    world: &mut World,
    actor: Entity,
    spell: &combat::spell_info::SpellInfo,
    costs: &[(spell_cast::CostPool, f32)],
) {
    if spell.id != 6572
        || world
            .get::<shared::components::ActiveSpec>(actor)
            .is_none_or(|spec| spec.0 != 73)
    {
        return;
    }
    let paid: f32 = costs
        .iter()
        .filter_map(|(pool, amount)| {
            (*pool == spell_cast::CostPool::Power(shared::components::PowerType::Rage))
                .then_some(*amount)
        })
        .sum();
    if paid <= 0.0 {
        return;
    }
    let Some(instance) = world.get::<ActiveAuras>(actor).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == 392792 && aura.caster == actor)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    if procs::try_native_policy_proc(world, actor, instance) {
        // SimC76f8118 sc_warrior.cpp6013–6041: refund effect 3 of paid Rage.
        let percent = spell_cast::scripts::script_effect_value(world, actor, 392792, 2);
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            actor,
            392793,
            Some(actor),
            1,
            paid * percent / 100.0,
        );
    }
}

fn on_white_impact(world: &mut World, actor: Entity) {
    // SimC76f8118 sc_warrior.cpp2124–2129: every outgoing white impact.
    roll_revenge(world, actor);
}

fn on_incoming_event(world: &mut World, event: &AuraEvent) {
    // SimC76f8118 sc_warrior.cpp9478–9485: only actual dodges and parries.
    if event.hit_mask & (PROC_HIT_DODGE | PROC_HIT_PARRY) == 0 || !event.can_proc {
        return;
    }
    if let Some(target) = event.target {
        roll_revenge(world, target);
    }
}

fn roll_revenge(world: &mut World, actor: Entity) {
    let Some(instance) = world.get::<ActiveAuras>(actor).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TRIGGER && aura.caster == actor)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    if procs::try_native_policy_proc(world, actor, instance)
        && !spell_cast::try_cast_triggered(world, actor, BUFF, Some(actor), 1)
    {
        warn!(?actor, "Revenge Trigger could not apply its native buff");
    }
}
