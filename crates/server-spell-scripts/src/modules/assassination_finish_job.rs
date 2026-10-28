//! Finish the Job follows a real Deathmark application.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraInstance;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterAuraApplication(grant_buff)];

fn grant_buff(world: &mut World, _: Entity, aura: &AuraInstance) {
    let owner = aura.caster;
    if aura.spell_id != 360194 || !combat::auras::has_aura(world, owner, 1249809) {
        return;
    }
    // SimC76f8118 sc_rogue.cpp4208–4217: actual Deathmark impact grants
    // the native Finish the Job direct, periodic and auto-attack buff.
    if !spell_cast::try_cast_triggered(world, owner, 1249810, Some(owner), 1) {
        warn!(?owner, "Finish the Job application refused");
    }
}
