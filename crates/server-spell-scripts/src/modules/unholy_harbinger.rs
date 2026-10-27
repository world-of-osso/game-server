//! Harbinger of Doom reduces Putrefy recharge when Scourge spends a ghoul.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{script_registry::SpellScriptHook, scripts::{reduce_cooldown, script_effect_value}};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeNativeOnCast(on_cast)];

fn on_cast(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth != 0 || spell.id != 55090 || !combat::auras::has_aura(world, caster, 276023)
        || !combat::auras::has_aura(world, caster, 1254252) { return; }
    // SimC76f8118 sc_death_knight.cpp12597–12610: consuming an earned
    // ghoul reduces the native Putrefy recharge by Harbinger effect3.
    let milliseconds = script_effect_value(world, caster, 276023, 2);
    reduce_cooldown(world, caster, 1247378, f64::from(milliseconds) / 1000.0);
}
