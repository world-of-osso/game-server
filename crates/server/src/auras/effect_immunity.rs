//! TC a352b1fa SpellInfo.cpp:3555/3804, Unit.cpp:7934:
//! aura37 MiscValue is a SpellEffects ID, independent of school/mechanic/positivity.
use super::{ActiveAuras, handlers};
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;

pub(crate) fn blocks_effect(
    world: &World,
    target: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> bool {
    // SPELL_ATTR0_NO_IMMUNITIES and SpellEffectAttributes::NoImmunity (DBCEnums.h).
    if super::attributes(spell, 0) & 0x2000_0000 != 0 || effect.attributes & 1 != 0 {
        return false;
    }
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().flat_map(|aura| &aura.effects).any(|immunity| {
            immunity.aura_type == handlers::EFFECT_IMMUNITY
                && immunity.misc_values[0] > 0
                && immunity.misc_values[0] as u32 == effect.effect
        })
    })
}
