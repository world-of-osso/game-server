//! Fleeing cast admission, TC a352b1fa `Spell::CheckCasterAuras` (Spell.cpp:6964-7052),
//! `SpellInfo::SpellCancelsAuraEffect` / `CanSpellEffectProvideImmunityAgainstAuraEffect`
//! (SpellInfo.cpp:3869-3921) and `_LoadImmunityInfo`'s allowed mechanics (3613-3650).
//! Immunity sources modelled: aura77 mechanics, aura39 schools and aura147
//! definition mechanics/schools/auras. Special-ID aura77 masks, dispel and
//! apply-aura school immunity are not.
use bevy::prelude::*;

use super::{ActiveAuras, attributes, handlers};
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};

const ATTR0_NO_IMMUNITIES: u32 = 0x2000_0000;
const ATTR1_IMMUNITY_PURGES_EFFECT: u32 = 0x0000_8000;
const ATTR2_NO_SCHOOL_IMMUNITIES: u32 = 0x0400_0000;
const ATTR5_ALLOW_WHILE_STUNNED: u32 = 0x0000_0008;
const ATTR5_ALLOW_WHILE_FLEEING: u32 = 0x0002_0000;
const ATTR5_ALLOW_WHILE_CONFUSED: u32 = 0x0004_0000;
const EFFECT_NO_IMMUNITY: u32 = 0x0000_0001;
const MECHANIC_DISORIENTED: u32 = 2;
const MECHANIC_FEAR: u32 = 5;
const MECHANIC_SLEEP: u32 = 10;
const MECHANIC_STUN: u32 = 12;
const MECHANIC_FREEZE: u32 = 13;
const MECHANIC_KNOCKOUT: u32 = 14;
const MECHANIC_HORROR: u32 = 24;
const BARKSKIN: u32 = 22812;
const DISPERSION: u32 = 47585;
const LICHBORNE: u32 = 49039;

fn bit(mechanic: u32) -> u64 {
    if mechanic == 0 || mechanic >= u64::BITS {
        0
    } else {
        1 << mechanic
    }
}

fn spell_mechanic(spell: &SpellInfo) -> u32 {
    spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.mechanic)
}

/// `SpellInfo::GetAllEffectsMechanicMask`.
fn all_effects_mechanic_mask(spell: &SpellInfo) -> u64 {
    spell
        .effects
        .iter()
        .filter(|effect| effect.effect != 0)
        .fold(bit(spell_mechanic(spell)), |mask, effect| {
            mask | bit(effect.mechanic)
        })
}

/// One effect's `ImmunityInfo::MechanicImmuneMask`.
fn effect_mechanic_immunity(effect: &SpellEffect) -> u64 {
    match effect.aura {
        handlers::MECHANIC_IMMUNITY => match u32::try_from(effect.misc_values[0]) {
            Ok(mechanic) => bit(mechanic),
            Err(_) => 0,
        },
        handlers::MECHANIC_IMMUNITY_MASK => effect
            .immunity
            .as_ref()
            .map_or(0, |definition| definition.mechanics_mask),
        _ => 0,
    }
}

fn effect_school_immunity(effect: &SpellEffect) -> u32 {
    match effect.aura {
        handlers::SCHOOL_IMMUNITY => effect.misc_values[0] as u32,
        handlers::MECHANIC_IMMUNITY_MASK => effect
            .immunity
            .as_ref()
            .map_or(0, |definition| definition.school_mask),
        _ => 0,
    }
}

/// `SpellInfo::GetAllowedMechanicMask`.
fn allowed_mechanic_mask(spell: &SpellInfo) -> u64 {
    let mut mask = spell
        .effects
        .iter()
        .fold(0, |mask, effect| mask | effect_mechanic_immunity(effect));
    if attributes(spell, 5) & ATTR5_ALLOW_WHILE_STUNNED != 0 {
        mask |= match spell.id {
            BARKSKIN | DISPERSION => {
                bit(MECHANIC_STUN)
                    | bit(MECHANIC_FREEZE)
                    | bit(MECHANIC_KNOCKOUT)
                    | bit(MECHANIC_SLEEP)
            }
            LICHBORNE => 0,
            _ => bit(MECHANIC_STUN),
        };
    }
    if attributes(spell, 5) & ATTR5_ALLOW_WHILE_CONFUSED != 0 {
        mask |= bit(MECHANIC_DISORIENTED);
    }
    if attributes(spell, 5) & ATTR5_ALLOW_WHILE_FLEEING != 0 {
        mask |= bit(MECHANIC_FEAR)
            | if matches!(spell.id, BARKSKIN | DISPERSION) {
                bit(MECHANIC_HORROR)
            } else {
                0
            };
    }
    mask
}

/// `CanSpellEffectProvideImmunityAgainstAuraEffect` for the modelled immunity sources.
fn effect_provides_immunity(
    effect: &SpellEffect,
    aura_spell: &SpellInfo,
    fear: &SpellEffect,
) -> bool {
    let schools = effect_school_immunity(effect);
    if attributes(aura_spell, 2) & ATTR2_NO_SCHOOL_IMMUNITIES == 0
        && schools & aura_spell.school_mask() != 0
    {
        return true;
    }
    let mechanics = effect_mechanic_immunity(effect);
    if mechanics & (bit(spell_mechanic(aura_spell)) | bit(fear.mechanic)) != 0 {
        return true;
    }
    effect.aura == handlers::MECHANIC_IMMUNITY_MASK
        && effect
            .immunity
            .as_ref()
            .is_some_and(|definition| definition.auras.contains(&fear.aura))
}

/// `SpellInfo::SpellCancelsAuraEffect`.
fn cancels(spell: &SpellInfo, aura_spell: &SpellInfo, fear: &SpellEffect) -> bool {
    attributes(spell, 1) & ATTR1_IMMUNITY_PURGES_EFFECT != 0
        && attributes(aura_spell, 0) & ATTR0_NO_IMMUNITIES == 0
        && fear.attributes & EFFECT_NO_IMMUNITY == 0
        && spell
            .effects
            .iter()
            .any(|effect| effect_provides_immunity(effect, aura_spell, fear))
}

/// Whether the caster's fear auras forbid casting `spell` (`SPELL_FAILED_FLEEING`).
pub(super) fn blocks_cast(world: &World, auras: &ActiveAuras, spell: &SpellInfo) -> bool {
    let store = world.resource::<SpellStore>();
    let allow_while_fleeing = attributes(spell, 5) & ATTR5_ALLOW_WHILE_FLEEING != 0;
    let allowed = allowed_mechanic_mask(spell);
    auras.iter().any(|aura| {
        let Some(aura_spell) = store.get(aura.spell_id) else {
            return false;
        };
        aura.effects
            .iter()
            .filter(|effect| effect.aura_type == handlers::MOD_FEAR)
            .filter_map(|state| {
                aura_spell
                    .effects
                    .iter()
                    .find(|effect| effect.index == state.index)
            })
            .any(|fear| {
                if allow_while_fleeing {
                    let mask = all_effects_mechanic_mask(aura_spell);
                    mask != 0 && mask & allowed == 0
                } else {
                    !cancels(spell, aura_spell, fear)
                }
            })
    })
}
