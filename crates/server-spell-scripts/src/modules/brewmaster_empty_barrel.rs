//! SimC76f8118 sc_monk.cpp2613–2616: Keg Smash consumes Empty Barrel.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, RemoveReason};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook};
use combat::spell_info::SpellInfo;

const EMPTY_BARREL: u32 = 1265307;
const EMPTY_BARREL_DAMAGE: u32 = 1265133;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 121253).then_some(consume_empty_barrel)
}

fn consume_empty_barrel(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !auras::has_aura(world, caster, EMPTY_BARREL) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        error!(
            ?caster,
            "Keg Smash cannot consume Empty Barrel without a primary target"
        );
        return;
    };
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == EMPTY_BARREL && aura.caster == caster
    });
    spell_cast::cast_triggered(world, caster, EMPTY_BARREL_DAMAGE, Some(target), 1);
}
