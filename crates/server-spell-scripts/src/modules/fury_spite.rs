//! Spite's selected native rank values are expressed in tenths of a percent.
use bevy::prelude::*;

use super::combat;
use combat::auras::AuraInstance;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

const SPITE: u32 = 383852;

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    scripts.0.entry(SPITE).or_default().on_apply = Some(bind_spite);
}

fn bind_spite(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit {
        return;
    }
    // SimC76f8118 sc_warrior.cpp7894–7899 and the native tooltip divide
    // each selected rank value by10; preserve the rank curves and masks.
    for index in [0, 1] {
        let amount =
            combat::spell_cast::scripts::script_effect_value(world, unit, SPITE, index) / 10.0;
        combat::auras::set_effect_amount(world, unit, aura.instance_id, index, amount);
    }
}
