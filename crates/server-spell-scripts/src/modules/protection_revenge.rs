//! Revenge Trigger rolls on white impacts and incoming dodges or parries.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    ActiveAuras,
    procs::{self, AuraEvent, PROC_HIT_DODGE, PROC_HIT_PARRY},
};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

const TRIGGER: u32 = 5301;
const BUFF: u32 = 5302;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TRIGGER),
    SpellScriptHook::AfterWhiteImpact(on_white_impact),
    SpellScriptHook::AuraEvent(on_incoming_event),
];

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
