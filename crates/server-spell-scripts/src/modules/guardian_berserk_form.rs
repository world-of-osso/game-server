//! SimC76f8118 sc_druid.cpp5222–5227: Guardian Berserk autoshifts to Bear.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::BeforeNativeOnCast(shift_to_bear)];

fn shift_to_bear(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth == 0 && matches!(spell.id, 50334 | 102558) && !auras::has_aura(world, caster, 5487) {
        spell_cast::cast_triggered(world, caster, 5487, Some(caster), 1);
    }
}
