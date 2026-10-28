//! The pinned Unholy rotation has no legacy Festering Wound RP proc.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::script_registry::SpellScriptHook;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(197147).or_default().check_proc = Some(check_proc);
}

fn check_proc(world: &World, aura: &AuraInstance, _: &AuraEvent) -> bool {
    // SimC76f8118 sc_death_knight.cpp10411–10503 grants ready ghouls from
    // Festering Strike and Scythe; it has no 197147/195757 wound producer.
    world
        .get::<ActiveSpec>(aura.caster)
        .is_none_or(|spec| spec.0 != 252)
}
