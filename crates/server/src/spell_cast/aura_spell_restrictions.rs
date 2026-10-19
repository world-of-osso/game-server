//! Native aura references/types; TC Spell.cpp5881–5889/SpellInfo.cpp2510–2517.
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
    check_caster_exclusion(world, caster, spell)?;
    let excluded_type = restrictions.exclude_caster_aura_type;
    if excluded_type != 0
        && world
            .get::<ActiveAuras>(caster)
            .is_some_and(|auras| auras.has_type(excluded_type))
    {
        return Err(CastFailure::CasterAuraState);
    }
    Ok(())
}

pub(super) fn check_caster_exclusion(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let excluded = spell
        .aura_restrictions
        .as_deref()
        .map_or(0, |restrictions| restrictions.exclude_caster_aura_spell);
    if excluded != 0 && crate::auras::has_aura(world, caster, excluded) {
        return Err(CastFailure::CasterAuraState);
    }
    Ok(())
}

pub(super) fn check_target(
    world: &World,
    caster: Entity,
    target: Option<Entity>,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    let (Some(target), Some(restrictions)) = (target, spell.aura_restrictions.as_deref()) else {
        return Ok(());
    };
    check_target_state(world, caster, target, spell)?;
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

// TC SpellInfo.cpp2495–2504; Unit.cpp6163–6188. Only computed states gate casts.
fn check_target_state(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> Result<(), CastFailure> {
    if target_state_exempt(world, caster, target, spell) {
        return Ok(());
    }
    let Some(restrictions) = spell.aura_restrictions.as_ref() else {
        return Ok(());
    };
    let required = crate::auras::aura_state::read_computed_state(
        world,
        target,
        restrictions.target_aura_state,
    );
    let excluded = crate::auras::aura_state::read_computed_state(
        world,
        target,
        restrictions.exclude_target_aura_state,
    );
    if required == Some(false) || excluded == Some(true) {
        return Err(CastFailure::TargetAuraState);
    }
    Ok(())
}

fn target_state_exempt(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> bool {
    let owner_target = world
        .get::<shared::components::UnitSummonedBy>(caster)
        .is_some_and(|owner| owner.0 == target.to_bits());
    let vehicle = world
        .get::<shared::components::Mounted>(caster)
        .is_some_and(|mount| mount.vehicle_id != 0);
    const ABILITY_IGNORE_AURASTATE: u32 = 262;
    let ignores_state = crate::auras::spell_mods::has_aura_type_affecting(
        world,
        caster,
        ABILITY_IGNORE_AURASTATE,
        spell,
    );
    owner_target || vehicle || ignores_state
}
