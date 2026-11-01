//! Bullseye only earns its critical strike buff against low-health targets.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::script_effect_value};
use shared::components::Health;

const BULLSEYE: u32 = 204089;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(BULLSEYE).or_default().check_proc = Some(target_below_threshold);
}

fn target_below_threshold(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if event.actor != aura.caster {
        return false;
    }
    let Some(health) = event.target.and_then(|target| world.get::<Health>(target)) else {
        return false;
    };
    // SimC76f8118 sc_hunter.cpp8552–8579: an eligible hit procs only
    // below the native Bullseye effect1 health percentage (DB2 70448: 20%).
    let threshold = script_effect_value(world, aura.caster, BULLSEYE, 0);
    health.current * 100.0 < health.max * threshold
}
