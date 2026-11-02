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
/// target entities return `None`; radii do not depend on the target's identity.
pub fn spell_unintended_reach(
    world: &World,
    caster: Entity,
    spell_id: u32,
    target: Entity,
) -> Option<(f32, f32)> {
    world.get_entity(target).ok()?;
    let spell = world.resource::<SpellStore>().get(spell_id)?;
    let registry = *world.resource::<SpellScriptRegistry>();
    let reach = spell_reach(world, caster, spell, registry, 0);
    Some((reach.caster_radius, reach.target_radius))
}

pub(super) fn spell_reach_without_chain(
    world: &World,
    caster: Entity,
    spell_id: u32,
    target: Entity,
) -> Option<(f32, f32)> {
    world.get_entity(target).ok()?;
    let mut spell = world.resource::<SpellStore>().get(spell_id)?.clone();
    spell.cast_single_target = true;
    let registry = *world.resource::<SpellScriptRegistry>();
    let reach = spell_reach(world, caster, &spell, registry, 0);
    Some((reach.caster_radius, reach.target_radius))
}

fn unit_reach(world: &mut World, unit: Entity) -> DeclaredReach {
    let registry = *world.resource::<SpellScriptRegistry>();
    let mut reach = DeclaredReach::default();
    for spell in unit_spells(world, unit)
        .into_iter()
        .chain(registry.additional_reach_actions(world, unit))
    {
        // Radii are target-independent; the actor supplies a valid entity for
        // this maximum over possible actions when no primary target exists.
        let Some((caster_radius, target_radius)) = spell_unintended_reach(world, unit, spell, unit)
        else {
            continue;
        };
        reach.caster_radius = reach.caster_radius.max(caster_radius);
        reach.target_radius = reach.target_radius.max(target_radius);
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
    registry: SpellScriptRegistry,
    depth: u8,
) -> DeclaredReach {
    let mut result = registry.declared_reach(world, caster, spell);
    if depth < crate::spell_cast::effects::MAX_TRIGGER_DEPTH {
        for rank in 1..=4 {
            let Some(release) = crate::spell_cast::script_registry::empower_release_preview(
                world, caster, spell.id, rank,
            ) else {
                break;
            };
            let reach = spell_reach(world, caster, &release, registry, depth + 1);
            result.caster_radius = result.caster_radius.max(reach.caster_radius);
            result.target_radius = result.target_radius.max(reach.target_radius);
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
        let child = spell_reach(world, caster, child, registry, depth + 1);
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
        }
    }
    if depth < crate::spell_cast::effects::MAX_TRIGGER_DEPTH {
        for child_id in registry.declared_children(world, caster, spell) {
            let Some(child) = world.resource::<SpellStore>().get(child_id) else {
                continue;
            };
            let child = spell_reach(world, caster, child, registry, depth + 1);
            result.caster_radius = result.caster_radius.max(child.caster_radius);
            result.target_radius = result.target_radius.max(child.target_radius);
        }
        let parent = result;
        for (child_id, caster_range, splash_radius) in
            crate::spell_cast::scripts::reach_children(world, caster, spell)
        {
            let Some(child) = world.resource::<SpellStore>().get(child_id) else {
                continue;
            };
            let child = spell_reach(world, caster, child, registry, depth + 1);
            result.caster_radius = result.caster_radius.max(child.caster_radius);
            if caster_range > 0.0 {
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
            }
        }
    }
    result
}
