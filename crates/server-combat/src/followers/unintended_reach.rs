//! Reach of hostile secondary targets, for follower pull-position selection.
use std::collections::HashSet;

use super::Follower;
use crate::class_progression::KnownSpells;
use crate::spell_cast::guardian::GuardianActor;
use crate::spell_cast::script_registry::{DeclaredReach, SpellScriptRegistry};
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::targets;
use crate::spell_info::SpellStore;
use bevy::prelude::*;

/// One entry per party member and currently owned pet, centered on that unit.
/// The second radius is centered on that unit's explicit combat target.
pub fn party_unintended_reach(world: &mut World, party: &[Entity]) -> Vec<(Entity, f32, f32)> {
    let units = party_and_pets(world, party);
    units
        .into_iter()
        .map(|unit| {
            let reach = unit_reach(world, unit);
            (unit, reach.caster_radius, reach.target_radius)
        })
        .collect()
}

fn party_and_pets(world: &mut World, party: &[Entity]) -> Vec<Entity> {
    let owners: HashSet<_> = party.iter().copied().collect();
    let mut units = party.to_vec();
    let pets: Vec<_> = crate::system_states::with_query::<
        (
            Entity,
            Option<&crate::hunter_pet::HunterPet>,
            Option<&SummonedCreature>,
        ),
        Or<(With<crate::hunter_pet::HunterPet>, With<SummonedCreature>)>,
        _,
    >(world, |world, query| {
        query
            .iter(world)
            .filter(|(unit, pet, summon)| {
                let owner = pet
                    .map(|pet| pet.owner)
                    .or_else(|| summon.map(|summon| summon.owner));
                owner.is_some_and(|owner| {
                    owners.contains(&owner) && targets::same_map(world, owner, *unit)
                })
            })
            .map(|(unit, _, _)| unit)
            .collect()
    });
    for pet in pets {
        if !units.contains(&pet) {
            units.push(pet);
        }
    }
    units
}

/// Reach of one spell around its caster and explicit target. Missing spells or
/// target entities return `None`.
pub fn spell_unintended_reach(
    world: &World,
    caster: Entity,
    spell_id: u32,
    target: Entity,
) -> Option<(f32, f32)> {
    let reach = action_reach(world, caster, spell_id, target, true)?;
    Some((reach.caster_radius, reach.target_radius))
}

/// Reach of one action at `target`; `chain: false` admits only its first chain target.
pub(super) fn action_reach(
    world: &World,
    caster: Entity,
    spell_id: u32,
    target: Entity,
    chain: bool,
) -> Option<DeclaredReach> {
    world.get_entity(target).ok()?;
    let mut spell = world.resource::<SpellStore>().get(spell_id)?.clone();
    spell.cast_single_target |= !chain;
    let registry = *world.resource::<SpellScriptRegistry>();
    Some(spell_reach(
        world,
        caster,
        &spell,
        Some(target),
        registry,
        0,
    ))
}

fn unit_reach(world: &mut World, unit: Entity) -> DeclaredReach {
    let registry = *world.resource::<SpellScriptRegistry>();
    let mut reach = DeclaredReach::default();
    for spell in unit_spells(world, unit)
        .into_iter()
        .chain(registry.additional_reach_actions(world, unit))
    {
        let Some(spell) = world.resource::<SpellStore>().get(spell) else {
            continue;
        };
        // No primary exists yet: take the maximum over possible targets.
        let spell = spell_reach(world, unit, spell, None, registry, 0);
        // A beam runs from the unit to its own target; with no target yet, this
        // bounds it only by its width around each anchor.
        reach.caster_radius = reach
            .caster_radius
            .max(spell.caster_radius)
            .max(spell.beam_half_width);
        reach.target_radius = reach
            .target_radius
            .max(spell.target_radius)
            .max(spell.beam_half_width);
    }
    let fields = attached_field_reach(world, unit);
    reach.caster_radius = reach.caster_radius.max(fields.caster_radius);
    reach.target_radius = reach.target_radius.max(fields.target_radius);
    reach
}

/// Active hostile fields attached to a unit move with it to the fight position.
/// Fields fixed in place cover the same ground wherever the party fights, so
/// they add no reach around the fight position.
fn attached_field_reach(world: &mut World, unit: Entity) -> DeclaredReach {
    use crate::spell_cast::area_trigger;
    let map = world.get::<crate::maps::InMap>(unit).copied();
    let now = crate::auras::aura_now(world);
    let fields: Vec<_> = world
        .query::<(Entity, &area_trigger::SpellAreaTrigger, &crate::maps::InMap)>()
        .iter(world)
        .filter(|(_, field, field_map)| {
            field.caster == unit
                && field.relation == targets::Relation::Enemy
                && field.expires_at.is_none_or(|expiry| expiry > now)
                && Some(**field_map) == map
        })
        .map(|(entity, _, _)| entity)
        .collect();
    let mut reach = DeclaredReach::default();
    for field in fields {
        let Some(attached) = area_trigger::attached_unit(world, field) else {
            continue;
        };
        let radius = area_trigger::maximum_field_radius(world, field)
            .expect("active hostile area requires its native maximum radius");
        if attached == unit {
            reach.caster_radius = reach.caster_radius.max(radius);
        } else {
            reach.target_radius = reach.target_radius.max(radius);
        }
    }
    reach
}

fn unit_spells(world: &World, unit: Entity) -> Vec<u32> {
    if world.get::<Follower>(unit).is_some() {
        return super::action_spells::available(world, unit)
            .expect("follower available action resolution failed");
    }
    let known = world
        .get::<KnownSpells>(unit)
        .into_iter()
        .flat_map(KnownSpells::iter);
    let pet = world
        .get::<crate::pet_control::PetCharm>(unit)
        .into_iter()
        .flat_map(crate::pet_control::PetCharm::known_spells);
    let guardian = world
        .get::<GuardianActor>(unit)
        .into_iter()
        .flat_map(|guardian| guardian.spells.iter().copied());
    known.chain(pet).chain(guardian).collect()
}

fn spell_reach(
    world: &World,
    caster: Entity,
    spell: &crate::spell_info::SpellInfo,
    primary: Option<Entity>,
    registry: SpellScriptRegistry,
    depth: u8,
) -> DeclaredReach {
    if let Some(owned) = registry.owned_reach(world, caster, spell) {
        return owned;
    }
    let mut result = registry.declared_reach(world, caster, spell);
    if depth < crate::spell_cast::effects::MAX_TRIGGER_DEPTH {
        for rank in 1..=4 {
            let Some(release) = crate::spell_cast::script_registry::empower_release_preview(
                world, caster, spell.id, rank,
            ) else {
                break;
            };
            let reach = spell_reach(world, caster, &release, primary, registry, depth + 1);
            merge_reach(&mut result, reach);
        }
    }
    if let Some(radius) =
        crate::spell_cast::scripts::hostile_ground_pulse_radius(world, caster, spell.id)
    {
        // Named ground casts may use either caster or target as their destination.
        result.caster_radius = result.caster_radius.max(radius);
        result.target_radius = result.target_radius.max(radius);
    }
    for effect in &spell.effects {
        let (caster_radius, target_radius) =
            targets::unintended_effect_reach(world, caster, spell, effect);
        result.caster_radius = result.caster_radius.max(caster_radius);
        result.target_radius = result.target_radius.max(target_radius);
        // Spell modifier auras reference the modified spell; they do not cast it.
        if effect.trigger_spell == 0
            || depth >= crate::spell_cast::effects::MAX_TRIGGER_DEPTH
            || (effect.effect == 6
                && matches!(
                    effect.aura,
                    crate::auras::handlers::ADD_FLAT_MODIFIER
                        | crate::auras::handlers::ADD_PCT_MODIFIER
                        | crate::auras::handlers::ADD_FLAT_MODIFIER_BY_SPELL_LABEL
                        | crate::auras::handlers::ADD_PCT_MODIFIER_BY_SPELL_LABEL
                ))
        {
            continue;
        }
        let Some(child) = world.resource::<SpellStore>().get(effect.trigger_spell) else {
            continue;
        };
        let child = spell_reach(world, caster, child, primary, registry, depth + 1);
        result.caster_radius = result.caster_radius.max(child.caster_radius);
        if effect.implicit_targets.contains(&1) {
            result.caster_radius = result.caster_radius.max(child.target_radius);
        } else {
            result.caster_radius = result
                .caster_radius
                .max(caster_radius + child.target_radius);
            result.target_radius = result
                .target_radius
                .max(target_radius + child.target_radius);
            extend_beam(
                &mut result,
                caster_radius,
                target_radius,
                child.beam_half_width,
            );
        }
    }
    if depth < crate::spell_cast::effects::MAX_TRIGGER_DEPTH {
        for child_id in registry.declared_children(world, caster, spell) {
            let Some(child) = world.resource::<SpellStore>().get(child_id) else {
                continue;
            };
            let child = spell_reach(world, caster, child, primary, registry, depth + 1);
            merge_reach(&mut result, child);
        }
        let parent = result;
        for (child_id, caster_range, splash_radius) in
            crate::spell_cast::scripts::reach_children(world, caster, spell, primary)
        {
            let Some(child) = world.resource::<SpellStore>().get(child_id) else {
                continue;
            };
            // A ranged payload may land on another unit, which is its own primary.
            let child_primary = if caster_range > 0.0 { None } else { primary };
            let child = spell_reach(world, caster, child, child_primary, registry, depth + 1);
            result.caster_radius = result.caster_radius.max(child.caster_radius);
            if caster_range > 0.0 {
                extend_beam(&mut result, caster_range, 0.0, child.beam_half_width);
                result.caster_radius = result
                    .caster_radius
                    .max(caster_range + child.target_radius.max(splash_radius));
            } else {
                let extension = child.target_radius.max(splash_radius);
                if parent.caster_radius > 0.0 {
                    result.caster_radius =
                        result.caster_radius.max(parent.caster_radius + extension);
                }
                result.target_radius = result.target_radius.max(parent.target_radius + extension);
                extend_beam(
                    &mut result,
                    parent.caster_radius,
                    parent.target_radius,
                    child.beam_half_width,
                );
            }
        }
    }
    result
}

fn merge_reach(result: &mut DeclaredReach, reach: DeclaredReach) {
    result.caster_radius = result.caster_radius.max(reach.caster_radius);
    result.target_radius = result.target_radius.max(reach.target_radius);
    result.beam_half_width = result.beam_half_width.max(reach.beam_half_width);
}

/// A child beam runs from the caster to each parent recipient. Recipients within
/// `caster_radius` of the caster keep it inside `caster_radius + width`; the primary
/// and recipients within `target_radius` of it keep every beam point within
/// `target_radius + width` of the caster-to-primary beam.
fn extend_beam(result: &mut DeclaredReach, caster_radius: f32, target_radius: f32, width: f32) {
    if width <= 0.0 {
        return;
    }
    if caster_radius > 0.0 {
        result.caster_radius = result.caster_radius.max(caster_radius + width);
    }
    if target_radius > 0.0 {
        result.target_radius = result.target_radius.max(target_radius + width);
    }
    result.beam_half_width = result.beam_half_width.max(target_radius + width);
}
