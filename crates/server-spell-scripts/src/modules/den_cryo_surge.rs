//! INFERRED_WCL: Cryo Surge's periodic area damage is not linked in DB2.
//! VxKH96NhkT487pnC/18, G8rCyV2w7L3mtQ1f/4 and 6FJTa3cr2VDhypG1/30
//! show 1239871 at the 1239860 recipient on the source-matched periodic ticks.
//! Retire this inferred link when authoritative current-build behavior is verified.
use super::combat;
use bevy::prelude::*;
use combat::auras::{AuraInstance, scripts::{AuraScript, AuraScripts}};
use combat::spell_cast::{self, script_registry::SpellScriptHook, targets};

const CRYO_SURGE: u32 = 1239860;
const CRYO_SURGE_DAMAGE: u32 = 1239871;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SupportAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(CRYO_SURGE, AuraScript {
        on_periodic: Some(cast_cryo_surge_tick),
        ..Default::default()
    });
}

fn cast_cryo_surge_tick(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let Some(position) = targets::position(world, unit) else {
        warn!(?unit, "Cryo Surge aura target has no position");
        return;
    };
    spell_cast::cast_triggered_at_destination(world, aura.caster, CRYO_SURGE_DAMAGE, position, 1);
}
