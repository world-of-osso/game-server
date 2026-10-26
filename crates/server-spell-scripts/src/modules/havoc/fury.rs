//! SimC76f8118 sc_demon_hunter.cpp3339–3348,8567–8581: native Fury buff producers.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::AfterCounterLaunch(after_launch),
    SpellScriptHook::OwnsPolicyProc(|spell| spell == 197125),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        198013 | 452497 => Some(grant_student),
        _ => None,
    }
}

pub(super) fn apply_blind_fury(world: &mut World, caster: Entity, aura: &AuraInstance) {
    if aura.caster != caster || !has_talent(world, caster, 203550) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp9986–9993 uses talent effect3/50
    // every100ms: exactly its native raw amount per5s. Eye/Gaze already
    // carries the real aura85 Fury-regeneration effect; populate that amount.
    let amount = combat::spell_cast::scripts::script_effect_value(world, caster, 203550, 2);
    let regen = aura
        .effects
        .iter()
        .find(|effect| {
            effect.aura_type == combat::auras::handlers::MOD_POWER_REGEN
                && effect.misc_values[0] == 17
        })
        .expect("native beam Fury regeneration effect");
    assert!(
        combat::auras::set_effect_amount(world, caster, aura.instance_id, regen.index, amount),
        "applied native beam instance"
    );
}

fn grant_student(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if has_talent(world, caster, 452412) {
        combat::spell_cast::cast_triggered(world, caster, 453239, Some(caster), 1);
    }
}

fn after_launch(world: &mut World, caster: Entity, spell: u32) {
    // Native198813 is a queued triggered child, so it never runs the ordinary
    // foreground AfterCast stage. This callback follows its actual launch.
    if spell == 198813 && has_talent(world, caster, 389688) {
        combat::spell_cast::cast_triggered(world, caster, 389890, Some(caster), 1);
    }
    if spell == 201428 {
        roll_annihilation_refund(world, caster);
    }
}

fn roll_annihilation_refund(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_demon_hunter.cpp7472–7510,7673–7685: the real
    // Annihilation off-hand execution rolls Chaos Strike's native refund.
    // Own197125's event so broad metadata cannot spend its ICD without a payload.
    // The native helper takes the live aura instance, not its spell ID.
    let instance = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == 197125)
                .map(|aura| aura.instance_id)
        });
    if let Some(instance) = instance
        && combat::auras::procs::try_native_policy_proc(world, caster, instance)
    {
        combat::spell_cast::cast_triggered(world, caster, 193840, Some(caster), 1);
    }
}
