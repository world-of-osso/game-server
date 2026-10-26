//! Spirit Bond's authored native percentage stays in the shared aura/mastery engine.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason};
use combat::auras::scripts::AuraScripts;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
];

const SPIRIT_BOND: u32 = 263135;
const SPIRIT_BOND_MODIFIER: u32 = 459722;

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(SPIRIT_BOND).or_default();
    script.on_apply = Some(grant_modifier);
    script.on_remove = Some(remove_modifier);
}

fn grant_modifier(world: &mut World, unit: Entity, aura: &AuraInstance) {
    // SimC76f8118 sc_hunter.cpp1392–1395/2586–2589/7900: selected
    // Spirit Bond includes459722's native percentage. Its native label modifiers
    // update the real mastery effects; never replace mastery stats or damage math.
    if aura.caster != unit {
        return;
    }
    if !spell_cast::try_cast_triggered(world, unit, SPIRIT_BOND_MODIFIER, Some(unit), 1) {
        warn!(?unit, "Native Spirit Bond mastery dependency refused");
    }
}

fn remove_modifier(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    if aura.caster == unit {
        auras::remove_auras(world, unit, RemoveReason::Cancelled, |child| {
            child.spell_id == SPIRIT_BOND_MODIFIER && child.caster == unit
        });
    }
}
