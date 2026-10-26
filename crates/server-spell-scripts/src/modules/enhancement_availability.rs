//! SimC76f8118 sc_shaman.cpp8463–8470/8861–8868: selected talents replace foreground actions.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::{ActiveSpec, TraitLoadout};
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ActionAvailable(available)];

fn available(world: &World, caster: Entity, spell: u32) -> bool {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 263)
    {
        return true;
    }
    let replacements: &[u32] = match spell {
        188389 => &[470057],
        384352 => &[114051, 378270],
        _ => return true,
    };
    !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| replacements.contains(&talent.spell_id))
    })
}
