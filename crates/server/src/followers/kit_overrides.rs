//! Resolve follower player-kit actions before eligibility/radius/cost checks.
//! SimC76f8118 sc_druid.cpp4837,10671: Cat Swipe applies source48629's332
//! override metadata. This is not an invented aura or a learned-spell grant.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, handlers, overrides, shapeshift, spell_mods};
use crate::spell_cast::CastFailure;
use crate::spell_info::{SpellInfo, SpellStore};

const FORM_OVERRIDE_SOURCES: &[(i32, u32)] = &[(1, 48629)];

pub(super) fn resolve_action(world: &World, unit: Entity, base: u32) -> Result<u32, CastFailure> {
    let form = world
        .get::<ActiveAuras>(unit)
        .and_then(shapeshift::current_form)
        .map(|(_, form)| form);
    let source = FORM_OVERRIDE_SOURCES
        .iter()
        .find(|(required, _)| Some(*required) == form)
        .map(|(_, source)| *source);
    let resolved = match source {
        Some(source) => resolve_from_source(world, base, source)?,
        None => base,
    };
    Ok(overrides::resolve_spell(world, unit, resolved))
}

fn resolve_from_source(world: &World, base: u32, source: u32) -> Result<u32, CastFailure> {
    let store = world.resource::<SpellStore>();
    let spell = store.get(base).ok_or(CastFailure::UnknownSpell)?;
    let source = store.get(source).ok_or_else(|| {
        CastFailure::Unsupported(format!(
            "native follower form override source {source} is missing"
        ))
    })?;
    let Some(replacement) = source_replacement(source, spell) else {
        return Ok(base);
    };
    if store.get(replacement).is_none() {
        return Err(CastFailure::UnknownSpell);
    }
    Ok(replacement)
}

fn source_replacement(source: &SpellInfo, spell: &SpellInfo) -> Option<u32> {
    let class_set = source
        .class_options
        .as_ref()
        .map_or(0, |options| options.spell_class_set);
    source
        .effects
        .iter()
        .filter(|effect| effect.aura == handlers::OVERRIDE_ACTIONBAR_SPELLS)
        .find_map(|effect| {
            let matches = if effect.misc_values[0] != 0 {
                effect.misc_values[0] as u32 == spell.id
            } else {
                spell_mods::class_mask_matches(class_set, &effect.spell_class_mask, spell)
            };
            matches.then_some(effect.base_points as u32)
        })
}
