//! Native aura references/types; TC Spell.cpp5881–5883/SpellInfo.cpp2510–2517.
use bevy::prelude::*;

use super::CastFailure;
use crate::auras::ActiveAuras;
use crate::spell_info::SpellInfo;

pub(super) fn check_caster(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let Some(restrictions) = spell.aura_restrictions.as_deref() else {
        return Ok(());
    };
    let required = restrictions.caster_aura_spell;
    if required != 0 && !crate::auras::has_aura(world, caster, required) {
        return Err(CastFailure::CasterAuraState);
    }
    let excluded = restrictions.exclude_caster_aura_spell;
    if excluded != 0 && crate::auras::has_aura(world, caster, excluded) {
        return Err(CastFailure::CasterAuraState);
    }
    Ok(())
}

pub(super) fn check_target(
    world: &World,
    target: Option<Entity>,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let (Some(target), Some(restrictions)) = (target, spell.aura_restrictions.as_deref()) else {
        return Ok(());
    };
    let excluded = restrictions.exclude_target_aura_spell;
    if excluded != 0 && crate::auras::has_aura(world, target, excluded) {
        return Err(CastFailure::TargetAuraState);
    }
    let auras = world.get::<ActiveAuras>(target);
    let required_type = restrictions.target_aura_type;
    if required_type != 0 && !auras.is_some_and(|auras| auras.has_type(required_type)) {
        return Err(CastFailure::TargetAuraState);
    }
    let excluded_type = restrictions.exclude_target_aura_type;
    if excluded_type != 0 && auras.is_some_and(|auras| auras.has_type(excluded_type)) {
        return Err(CastFailure::TargetAuraState);
    }
    Ok(())
}
