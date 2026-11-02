//! Improved Adrenaline Rush grants full combo points when its aura starts.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use shared::components::PowerType;

const ADRENALINE_RUSH: u32 = 13750;
const TALENT: u32 = 395422;
const ENERGIZE: u32 = 395424;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::MidAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(ADRENALINE_RUSH).or_default().on_apply = Some(grant_combo_points);
}

fn grant_combo_points(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster != unit || !auras::has_aura(world, unit, TALENT) {
        return;
    }
    // SimC 76f8118 sc_rogue.cpp7283–7296: each new Adrenaline Rush buff
    // grants the maximum combo-point pool; ordinary refreshes do not.
    let (_, maximum) = spell_cast::powers::power_value(world, unit, PowerType::ComboPoints)
        .expect("Improved Adrenaline Rush requires combo points");
    spell_cast::cast_triggered_with_effect0_amount(world, unit, ENERGIZE, Some(unit), 1, maximum);
}
