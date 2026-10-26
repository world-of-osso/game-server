//! SimC76f8118 sc_demon_hunter.cpp3339–3348,8567–8581: native Fury buff producers.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(after_cast),
    SpellScriptHook::AfterCounterLaunch(after_launch),
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
    let mut auras = world
        .get_mut::<ActiveAuras>(caster)
        .expect("applied native beam aura");
    let beam = auras
        .get_mut(aura.instance_id)
        .expect("native beam instance");
    let regen = beam
        .effects
        .iter_mut()
        .find(|effect| {
            effect.aura_type == combat::auras::handlers::MOD_POWER_REGEN
                && effect.misc_values[0] == 17
        })
        .expect("native beam Fury regeneration effect");
    regen.amount = amount;
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
}
