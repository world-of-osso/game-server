//! SimC76f8118 sc_evoker.cpp9926–9939/10841–10849: real combat owns Siege.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{AuraInstance, RemoveReason, has_aura};
use combat::spell_cast::script_registry::SpellScriptHook;

const TALENT: u32 = 441246;
const SIEGE: u32 = 441248;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::CombatStatusChanged(combat_changed),
    SpellScriptHook::MidAuraRegistration(register),
];

fn combat_changed(world: &mut World, unit: Entity, active: bool) {
    if active {
        if has_aura(world, unit, TALENT)
            && !has_aura(world, unit, SIEGE)
            && !combat::spell_cast::try_cast_triggered(world, unit, SIEGE, Some(unit), 1)
        {
            warn!(?unit, "Native Unrelenting Siege grant refused");
        }
    } else {
        combat::auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
            aura.spell_id == SIEGE && aura.caster == unit
        });
    }
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(SIEGE).or_default().on_periodic = Some(grow_stack);
    scripts.0.entry(TALENT).or_default().on_remove = Some(remove_talent);
}

fn grow_stack(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index == 1 && aura.caster == unit && aura.stacks < aura.max_stacks {
        combat::auras::set_self_cast_stacks(world, unit, SIEGE, aura.stacks + 1);
    }
}

fn remove_talent(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    if unit == aura.caster {
        combat::auras::remove_auras(world, unit, RemoveReason::Cancelled, |aura| {
            aura.spell_id == SIEGE && aura.caster == unit
        });
    }
}
