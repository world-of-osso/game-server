//! SimC76f8118 sc_druid.cpp:3254, 6852, 11194–11196.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 202345;
const BUFF: u32 = 279709;

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::SuccessfulCastRelationships(after_cast)];

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 78674 | 191034)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(TALENT))
    {
        return;
    }
    let existing = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == BUFF && aura.caster == caster)
            .map(|aura| (aura.stacks, aura.max_stacks))
    });
    if let Some((stacks, maximum)) = existing {
        auras::set_self_cast_stacks(world, caster, BUFF, (stacks + 1).min(maximum));
    } else {
        spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
    }
}

pub(super) fn on_eclipse(world: &mut World, caster: Entity) {
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == BUFF && aura.caster == caster
    });
}
