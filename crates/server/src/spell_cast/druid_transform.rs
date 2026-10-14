//! TC a352b1fa spell_druid.cpp271–295/319/1336/1934/2263/2821.
//! Native BeforeCast: switch to the script's form only when not already in it.
use bevy::prelude::*;

use super::CastFailure;
use crate::auras::ActiveAuras;
use crate::auras::shapeshift::current_form;
use crate::spell_info::SpellInfo;

const CAT: u32 = 768;
const BEAR: u32 = 5487;

fn required_form(spell_id: u32) -> Option<(i32, u32)> {
    match spell_id {
        5215 | 252216 => Some((1, CAT)),
        99 | 50334 | 106898 => Some((5, BEAR)),
        _ => None,
    }
}

pub(super) fn before_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    depth: u8,
) -> Result<(), CastFailure> {
    let Some((form, child)) = required_form(spell.id) else {
        return Ok(());
    };
    let current = world
        .get::<ActiveAuras>(caster)
        .and_then(current_form)
        .map(|(_, form)| form);
    if current == Some(form) {
        return Ok(());
    }
    if !super::try_cast_triggered(world, caster, child, Some(caster), depth + 1) {
        return Err(CastFailure::Unsupported(format!(
            "native Druid form child {child} could not be cast for {}",
            spell.id
        )));
    }
    Ok(())
}
