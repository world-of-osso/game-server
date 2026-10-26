//! Native action permission shared by follower kits and literal fixture casts.
//! Selected trait metadata wins; named parent projections cover missing DB2 links.
mod form;

use bevy::prelude::*;
use std::collections::HashSet;

use super::CastFailure;
use crate::class_progression::{KnownSpells, TraitLoadout};
use crate::spell_info::SpellStore;

/// Controller selection treats unknown/unavailable endpoints as ordinary fallthrough.
/// Kept in the server library so qualification crates never become its dependency.
pub fn resolve_action(world: &World, actor: Entity, id: u32) -> Result<Option<u32>, String> {
    match resolve_actor_action(world, actor, id) {
        Ok(resolved) => Ok(Some(resolved)),
        Err(CastFailure::UnknownSpell | CastFailure::NotKnown) => Ok(None),
        Err(reason) => Err(format!("native literal action knowledge: {reason:?}")),
    }
}

/// Resolve a known spell, selected visible action or sourced parent action.
/// NPC identity never bypasses this controller-level knowledge check.
pub fn resolve_actor_action(
    world: &World,
    actor: Entity,
    requested: u32,
) -> Result<u32, CastFailure> {
    let known = world
        .get::<KnownSpells>(actor)
        .ok_or_else(|| CastFailure::Unsupported("native action actor lacks KnownSpells".into()))?;
    let resolved = resolve_aliases(world, actor, requested)?;
    let spell = world
        .resource::<SpellStore>()
        .get(resolved)
        .ok_or(CastFailure::UnknownSpell)?;
    if crate::auras::is_passive(spell) {
        return Err(CastFailure::NotKnown);
    }
    if known.contains(resolved) {
        return Ok(resolved);
    }
    for parent in known.iter() {
        if resolve_aliases(world, actor, parent)? == resolved {
            return Ok(resolved);
        }
    }
    Err(CastFailure::UnknownSpell)
}

/// Fixture consumers normalize unavailable knowledge without hiding other errors.
#[cfg(any(test, feature = "test-support"))]
pub fn resolve_optional_actor_action(
    world: &World,
    actor: Entity,
    requested: u32,
) -> Result<Option<u32>, CastFailure> {
    match resolve_actor_action(world, actor, requested) {
        Ok(resolved) => Ok(Some(resolved)),
        Err(CastFailure::UnknownSpell | CastFailure::NotKnown) => Ok(None),
        Err(reason) => Err(reason),
    }
}

fn resolve_aliases(world: &World, actor: Entity, requested: u32) -> Result<u32, CastFailure> {
    let mut current = requested;
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current) {
            return Err(CastFailure::Unsupported(format!(
                "native action alias cycle for spell{requested}"
            )));
        }
        let selected = selected_trait_action(world, actor, current);
        let projected = project_known_parent(world, actor, selected);
        let resolved = form::resolve_action(world, actor, projected)?;
        if resolved == current {
            return Ok(resolved);
        }
        current = resolved;
    }
}

fn selected_trait_action(world: &World, actor: Entity, requested: u32) -> u32 {
    let Some(traits) = world.get::<TraitLoadout>(actor) else {
        return requested;
    };
    traits
        .overrides
        .iter()
        .chain(&traits.visible_actions)
        .find_map(|&(base, action)| (base == requested && base != action).then_some(action))
        .unwrap_or(requested)
}

/// SIMC_KNOWN_PARENT_CAST_ACTION: existing follower_spec_kits.py projections.
/// Elemental114050's native description names1219480 (sc_shaman.cpp11419).
/// Demoniac426115 enables264178 (sc_warlock_init.cpp249–250). Neither pair
/// has an operative TraitDefinition visible/override link in native70448.
fn project_known_parent(world: &World, actor: Entity, requested: u32) -> u32 {
    let Some(known) = world.get::<KnownSpells>(actor) else {
        return requested;
    };
    if !known.contains(requested) {
        return requested;
    }
    match requested {
        // SIMC_DEVASTATION_MANEUVERABILITY_ACTION: sc_evoker.cpp6323–6327.
        // Native433871 effect0 names433874 but supplies no TraitDefinition override.
        357210 if known.contains(433871) => 433874,
        114050 => 1219480,
        426115 => 264178,
        _ => requested,
    }
}
