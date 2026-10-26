//! Slayer Reap executions retain native damage, chance and internal cooldown.
use bevy::prelude::*;

use super::combat;
use combat::auras::{ActiveAuras, procs};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterSpenderProc(after_execute),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

const REAP_TALENT: u32 = 444775;
const REAP_DAMAGE: u32 = 446005;
const IMMINENT_DEMISE: u32 = 444769;
const SUDDEN_DEATH: u32 = 52437;

fn owns_proc(spell: u32) -> bool {
    spell == REAP_TALENT
}

fn after_execute(world: &mut World, caster: Entity, spell: &SpellInfo, target: Option<Entity>) {
    // SimC76f8118 sc_warrior.cpp4347–4359/4547–4559: only an SD Execute
    // may roll selected Imminent Demise's Reap. This stage precedes the existing
    // SD consumer; the shared native Reap aura owns its500ms ICD (cpp7980–7981).
    if !matches!(spell.id, 163201 | 281000 | 280735) {
        return;
    }
    let Some(instance) = read_reap_instance(world, caster) else {
        return;
    };
    if !combat::auras::has_aura(world, caster, SUDDEN_DEATH)
        || !combat::auras::has_aura(world, caster, IMMINENT_DEMISE)
    {
        return;
    }
    let chance = script_effect_value(world, caster, IMMINENT_DEMISE, 1);
    if procs::try_policy_proc(world, caster, instance, chance)
        && !combat::spell_cast::try_cast_triggered(world, caster, REAP_DAMAGE, target, 1)
    {
        warn!(?caster, "Native Reap the Storm payload refused");
    }
}

fn read_reap_instance(world: &World, caster: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == REAP_TALENT && aura.caster == caster)
        .map(|aura| aura.instance_id)
}
