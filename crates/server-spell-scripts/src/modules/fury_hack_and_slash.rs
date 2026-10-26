//! Hack and Slash is spent after both delayed native Raging/Crushing weapon hits.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterCounterLaunch(consume_after_weapons)];

const HACK_AND_SLASH: u32 = 383873;

fn consume_after_weapons(world: &mut World, caster: Entity, spell: u32) {
    // SimC76f8118 sc_warrior.cpp5061/5227 decrements after MH and OH.
    // Native85288/335097 queue their OH last; consuming at parent launch
    // would remove the native20% before either delayed weapon hit.
    if !matches!(spell, 85384 | 335100) {
        return;
    }
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == HACK_AND_SLASH && aura.caster == caster)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        combat::auras::spend_stack(world, caster, instance);
    }
}
