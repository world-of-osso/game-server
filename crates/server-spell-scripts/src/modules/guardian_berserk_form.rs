//! SimC76f8118 sc_druid.cpp5222–5227: Guardian Berserk autoshifts to Bear.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeNativeOnCast(shift_to_bear),
    SpellScriptHook::AfterForegroundSpecial(reset_incarnation_cooldowns),
];

// SimC76f8118 sc_druid.cpp5222–5243 resets these on Incarnation as well as Berserk.
fn reset_incarnation_cooldowns(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == 102558 {
        for id in [33917, 77758, 6795] {
            spell_cast::scripts::reduce_cooldown(world, caster, id, f64::MAX);
        }
    }
}

fn shift_to_bear(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if depth == 0 && matches!(spell.id, 50334 | 102558) && !auras::has_aura(world, caster, 5487) {
        spell_cast::cast_triggered(world, caster, 5487, Some(caster), 1);
    }
}
