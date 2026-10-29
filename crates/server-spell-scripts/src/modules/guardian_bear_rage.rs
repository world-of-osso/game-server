//! SimC76f8118 sc_druid.cpp12480–12500: Bear Form's native proc earns hit-taken Rage.
use bevy::prelude::*;
use super::combat;
use combat::auras::{AuraInstance, procs::AuraEvent, scripts::{AuraScript, AuraScripts}};
use combat::spell_cast::{powers, script_registry::SpellScriptHook};
use shared::components::PowerType;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::LateAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(1178, AuraScript { on_damage_proc: Some(earn_rage), ..Default::default() });
}

fn earn_rage(world: &mut World, unit: Entity, aura: &AuraInstance, _: &AuraEvent) {
    let amount = aura.effects.iter().find(|effect| effect.index == 2)
        .expect("Bear Form passive requires its hit-taken Rage effect").amount;
    powers::change_power(world, unit, PowerType::Rage, amount * 10.0);
}
