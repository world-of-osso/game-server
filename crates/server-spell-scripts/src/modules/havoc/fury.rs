//! SimC76f8118 sc_demon_hunter.cpp3339–3348,8567–8581: native Fury buff producers.
use super::{combat, havoc::has_talent};
use bevy::prelude::*;
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
