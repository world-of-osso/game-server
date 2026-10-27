//! SimC76f8118 Ret878–881/cpp73–74: Wake grants Purpose through Aurora's native ICD.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, procs};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const AURORA: u32 = 439760;
const WAKE: u32 = 255937;
const PURPOSE: u32 = 408458;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    // After the sourced Dawnlight CounterAfterCast, matching Wake.execute order.
    SpellScriptHook::AuraRollAfterCast(hook),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

fn hook(spell: u32) -> Option<AfterCastHook> {
    (spell == WAKE).then_some(grant_after_wake)
}

fn owns_proc(spell: u32) -> bool {
    // Broad native DUMMY hints must not spend Aurora's ICD without its producer.
    spell == AURORA
}

fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 70)
        && world.get::<TraitLoadout>(caster).is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == AURORA)
        })
}

fn grant_after_wake(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == AURORA && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    let Some(instance) = instance else {
        warn!(?caster, "selected Aurora requires its native passive aura");
        return;
    };
    if procs::start_policy_proc(world, caster, instance).is_none() {
        return;
    }
    let depth = spell_cast::next_trigger_depth(world);
    if !spell_cast::try_cast_triggered(world, caster, PURPOSE, Some(caster), depth) {
        warn!(?caster, "Aurora native Divine Purpose grant refused");
    }
}
