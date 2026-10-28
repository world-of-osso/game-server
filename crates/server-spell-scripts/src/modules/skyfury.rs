//! Skyfury repeats the originating hand's white swing.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraInstance,
    procs::{AuraEvent, PROC_OFF_HAND_WEAPON_SWING},
    scripts::{AuraScript, AuraScripts},
};
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        462854,
        AuraScript {
            on_damage_proc: Some(repeat),
            ..Default::default()
        },
    );
}

fn repeat(world: &mut World, unit: Entity, _: &AuraInstance, event: &AuraEvent) {
    let Some(target) = event.target else { return };
    let hand = if event.actor_mask[0] & PROC_OFF_HAND_WEAPON_SWING != 0 {
        combat::melee::Hand::Off
    } else {
        combat::melee::Hand::Main
    };
    // SimC 76f8118 unique_gear.cpp 3215-3241: repeat the same hand without a miss or timer reset.
    combat::combat::repeat_melee_swing(world, unit, target, hand);
}
