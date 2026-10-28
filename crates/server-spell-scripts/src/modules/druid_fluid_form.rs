//! SimC76f8118 sc_druid.cpp2082–2107,4518,4782,5428: Fluid Form shifts on these foreground actions.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeNativeOnCast(shift)];

fn shift(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth != 0 || !auras::has_aura(world, caster, 449193) {
        return;
    }
    let form = match spell.id {
        1822 | 5221 => 768,
        33917 => 5487,
        _ => return,
    };
    if !auras::has_aura(world, caster, form) {
        spell_cast::cast_triggered(world, caster, form, Some(caster), 1);
    }
}
