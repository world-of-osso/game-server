//! SimC76f8118 sc_druid.cpp4868–4871: Tiger's Fury triggers Savage Fury.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 449645;
const BUFF: u32 = 449646;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 5217).then_some(grant)
}

fn grant(world: &mut World, caster: Entity, _: &SpellInfo) {
    if auras::has_aura(world, caster, TALENT) {
        spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
    }
}
