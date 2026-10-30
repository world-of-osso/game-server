//! Reach of hostile secondary targets, for follower pull-position selection.
use std::collections::HashSet;

use bevy::prelude::*;
use shared::components::Level;

use super::{Follower, FollowerCatalog};
use crate::class_progression::KnownSpells;
use crate::spell_cast::guardian::GuardianActor;
use crate::spell_cast::script_registry::{DeclaredReach, SpellScriptRegistry};
use crate::spell_cast::summon::SummonedCreature;
use crate::spell_cast::targets;
use crate::spell_info::SpellStore;

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
    let pets: Vec<_> = world
        .query::<(
            Entity,
            Option<&crate::hunter_pet::HunterPet>,
            Option<&SummonedCreature>,
        )>()
        .iter(world)
        .filter(|(_, pet, summon)| {
            pet.is_some_and(|pet| owners.contains(&pet.owner))
                || summon.is_some_and(|summon| owners.contains(&summon.owner))
        })
        .map(|(unit, _, _)| unit)
        .collect();
    for pet in pets {
        if !units.contains(&pet) {
            units.push(pet);
        }
    }
    units
}

fn unit_reach(world: &World, unit: Entity) -> DeclaredReach {
    let store = world.resource::<SpellStore>();
    let registry = *world.resource::<SpellScriptRegistry>();
    let mut reach = DeclaredReach::default();
    for spell in unit_spells(world, unit) {
        let Some(spell) = store.get(spell) else {
            continue;
        };
        let current = spell_reach(world, unit, spell, registry, 0);
        reach.caster_radius = reach.caster_radius.max(current.caster_radius);
        reach.target_radius = reach.target_radius.max(current.target_radius);
    }
    reach
}

fn unit_spells(world: &World, unit: Entity) -> Vec<u32> {
    if let Some(follower) = world.get::<Follower>(unit) {
        let level = world.get::<Level>(unit).expect("follower level").0;
        let catalog = world.resource::<FollowerCatalog>();
        let spells: Vec<_> = catalog
            .kits
            .iter()
            .filter(|entry| entry.available(follower.profile.spec_id, level))
            .map(|entry| entry.spell_id)
            .collect();
        if !spells.is_empty() {
            return spells;
        }
        return [
            follower.profile.attack_spell,
            follower.profile.cooldown_spell,
            follower.profile.heal_spell,
            follower.profile.taunt_spell,
            follower.profile.form_spell,
        ]
        .into_iter()
        .filter(|&spell| spell != 0)
        .collect();
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
    for effect in &spell.effects {
        let (caster_radius, target_radius) =
            targets::unintended_effect_reach(world, caster, spell, effect);
        result.caster_radius = result.caster_radius.max(caster_radius);
        result.target_radius = result.target_radius.max(target_radius);
        if effect.trigger_spell == 0 || depth >= crate::spell_cast::effects::MAX_TRIGGER_DEPTH {
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
    result
}
