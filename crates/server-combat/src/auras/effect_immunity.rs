//! TC a352b1fa SpellInfo.cpp:3555/3804, Unit.cpp:7934:
//! aura37 MiscValue is a SpellEffects ID, independent of school/mechanic/positivity.
use super::{ActiveAuras, handlers};

const ATTR0_NO_IMMUNITIES: u32 = 0x2000_0000;
/// TrinityCore a352b1fa DBCEnums.h:2418.
const EFFECT_NO_IMMUNITY: u32 = 0x0000_0001;
use crate::spell_info::{SpellEffect, SpellInfo};
use bevy::prelude::*;

pub(crate) fn blocks_effect(
    world: &World,
    target: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> bool {
    // SPELL_ATTR0_NO_IMMUNITIES and SpellEffectAttributes::NoImmunity (DBCEnums.h).
    if super::attributes(spell, 0) & ATTR0_NO_IMMUNITIES != 0
        || effect.attributes & EFFECT_NO_IMMUNITY != 0
    {
        return false;
    }
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .flat_map(|aura| &aura.effects)
            .any(|immunity| match immunity.aura_type {
                handlers::EFFECT_IMMUNITY => {
                    immunity.misc_values[0] > 0 && immunity.misc_values[0] as u32 == effect.effect
                }
                handlers::MECHANIC_IMMUNITY_MASK => immunity
                    .immunity
                    .as_ref()
                    .is_some_and(|definition| definition.effects.contains(&effect.effect)),
                _ => false,
            })
    })
}
