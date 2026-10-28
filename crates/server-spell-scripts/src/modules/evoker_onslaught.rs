//! SimC76f8118 sc_evoker.cpp9916–9923: Onslaught grants Burnout on combat entry.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::CombatStatusChanged(combat_changed)];

fn combat_changed(world: &mut World, unit: Entity, active: bool) {
    if !active || !combat::auras::has_aura(world, unit, 441245) {
        return;
    }
    if !combat::spell_cast::try_cast_triggered(world, unit, 375802, Some(unit), 1) {
        warn!(?unit, "Onslaught Burnout grant refused");
    }
}
