//! TC a352b1fa SpellInfo::CheckShapeshift and Spell::CheckCast's aura275 bypass.
use bevy::prelude::*;

use super::{CastFailure, attributes};
use crate::auras::ActiveAuras;
use crate::auras::handlers::MOD_IGNORE_SHAPESHIFT;
use crate::auras::shapeshift::{ShapeshiftForms, current_form};
use crate::spell_info::SpellInfo;

const FORM_STANCE: u32 = 0x0000_0001;
const FORM_ONLY_SHAPESHIFT_SPELLS: u32 = 0x0000_0400;
const NOT_SHAPESHIFTED: u32 = 0x0001_0000;
const ALLOW_CASTER_FORM: u32 = 0x0008_0000;

pub(super) fn check_cast(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if crate::auras::spell_mods::has_aura_type_affecting(
        world,
        caster,
        MOD_IGNORE_SHAPESHIFT,
        spell,
    ) {
        return Ok(());
    }
    let form = world
        .get::<ActiveAuras>(caster)
        .and_then(current_form)
        .map_or(0, |(_, form)| form);
    let stance = u32::try_from(form - 1)
        .ok()
        .and_then(|bit| 1u64.checked_shl(bit))
        .unwrap_or(0);
    let (required, excluded) = spell.shapeshift.as_ref().map_or((0, 0), |row| {
        (
            combine_mask(row.shapeshift_mask),
            combine_mask(row.shapeshift_exclude),
        )
    });
    if stance & excluded != 0 {
        return Err(CastFailure::NotShapeshift);
    }
    if stance & required != 0 {
        return Ok(());
    }
    check_unmatched_form(world, spell, form, required)
}

fn combine_mask(words: [u32; 2]) -> u64 {
    u64::from(words[0]) | (u64::from(words[1]) << 32)
}

fn check_unmatched_form(
    world: &World,
    spell: &SpellInfo,
    form: i32,
    required: u64,
) -> Result<(), CastFailure> {
    let flags = if form > 0 {
        let flags = world
            .get_resource::<ShapeshiftForms>()
            .and_then(|forms| forms.flags(form));
        let Some(flags) = flags else {
            // Pinned TC logs the missing native row and explicitly permits this cast.
            error!(
                form,
                spell_id = spell.id,
                "CheckShapeshift: unknown shapeshift form"
            );
            return Ok(());
        };
        flags
    } else {
        FORM_STANCE
    };
    check_flags(spell, flags, required)
}

fn check_flags(spell: &SpellInfo, flags: u32, required: u64) -> Result<(), CastFailure> {
    let shifted = flags & FORM_STANCE == 0;
    if shifted {
        let forbidden = attributes(spell, 0) & NOT_SHAPESHIFTED != 0;
        let only_form_spells = flags & FORM_ONLY_SHAPESHIFT_SPELLS != 0;
        if forbidden || only_form_spells {
            return Err(CastFailure::NotShapeshift);
        }
        if required != 0 {
            return Err(CastFailure::OnlyShapeshift);
        }
    } else if required != 0 && attributes(spell, 2) & ALLOW_CASTER_FORM == 0 {
        return Err(CastFailure::OnlyShapeshift);
    }
    Ok(())
}
