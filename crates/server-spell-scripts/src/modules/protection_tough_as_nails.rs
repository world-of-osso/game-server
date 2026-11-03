//! SimC76f8118 sc_warrior.cpp6425–6455,9494–9502: blocks retaliate.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    ActiveAuras,
    procs::{self, AuraEvent, PROC_HIT_BLOCK},
};
use combat::spell_cast::{self, script_registry::SpellScriptHook, targets};

const TALENT: u32 = 385888;
const DAMAGE: u32 = 385890;
const CANNOT_CRIT: u32 = 0x2000_0000;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT),
    SpellScriptHook::AuraEvent(on_block),
];

fn on_block(world: &mut World, event: &AuraEvent) {
    let Some(defender) = event.target else {
        return;
    };
    if !event.can_proc
        || event.periodic
        || event.hit_mask & PROC_HIT_BLOCK == 0
        || !targets::relation_matches(world, defender, event.actor, targets::Relation::Enemy)
    {
        return;
    }
    let Some(instance) = world.get::<ActiveAuras>(defender).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TALENT && aura.caster == defender)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    if !procs::try_native_policy_proc(world, defender, instance) {
        return;
    }
    let Some(mut damage) = spell_cast::load_triggered_spell(world, DAMAGE, 1) else {
        warn!(?defender, "Tough as Nails damage unavailable");
        return;
    };
    // SimC currently disables critical-block doubling and ordinary crits.
    damage
        .misc
        .as_mut()
        .expect("Tough as Nails metadata")
        .attributes[2] |= CANNOT_CRIT;
    if !spell_cast::launch_triggered_spell(world, defender, &damage, Some(event.actor), 1) {
        warn!(?defender, attacker = ?event.actor, "Tough as Nails retaliation refused");
    }
}
