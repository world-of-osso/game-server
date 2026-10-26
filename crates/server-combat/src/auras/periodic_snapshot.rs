//! Source-qualified persistent factors, separate from AP/SP and damage amounts.
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;

/// Application metadata may retain an already-applied cast factor. This does
/// not claim a fresh/live producer for those spells or modify their damage.
pub(crate) fn capture_multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> Option<f32> {
    if effect.aura == super::PERIODIC_DAMAGE_AURA
        && crate::spell_cast::script_registry::uses_persistent_cast_multiplier(world, spell.id)
    {
        return Some(spell.cast_damage_multiplier.unwrap_or(1.0));
    }
    multiplier(world, caster, spell, effect)
}

/// SimC76f8118 sc_druid.cpp2021–2030,4518–4568: native Feral bleeds
/// retain their application-time persistent factors until the next refresh.
pub(crate) fn multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> Option<f32> {
    let qualified = matches!(spell.id, 1079 | 155722 | 155625);
    let periodic = effect.index == 0 && effect.aura == super::PERIODIC_DAMAGE_AURA;
    if !(qualified && periodic) {
        return None;
    }
    let factor = super::spell_mods::snapshot_periodic_multiplier(world, caster, spell)
        * spell.cast_damage_multiplier.unwrap_or(1.0);
    Some(if spell.id == 155722 {
        crate::spell_cast::scripts::rake_bleed_snapshot(world, caster, factor)
    } else {
        factor
    })
}
