//! TC HandleCreateAreaTrigger REAL apply/remove: exact aura-effect ownership,
//! attachment to its target, and source CAST/ENEMY entry/exit actions.
use super::catalog::AuraFieldCatalog;
use super::{SpellAreaTrigger, targets};
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason, handlers};
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::{Health, Position};
use std::collections::HashSet;

#[derive(Component, Clone)]
pub(super) struct AuraAreaTrigger {
    target: Entity,
    instance_id: u32,
    effect_index: u8,
    properties: u32,
    created_at: f64,
}

pub(crate) fn on_applied(world: &mut World, target: Entity, aura: &AuraInstance) {
    for effect in aura
        .effects
        .iter()
        .filter(|effect| effect.aura_type == handlers::CREATE_AREATRIGGER)
    {
        let Ok(properties) = u32::try_from(effect.misc_values[0]) else {
            warn!(
                spell = aura.spell_id,
                "Aura395 has negative create-properties"
            );
            continue;
        };
        if let Err(reason) = create_field(world, target, aura, effect.index, properties) {
            warn!(
                spell = aura.spell_id,
                properties, "Aura395 field not created: {reason}"
            );
        }
    }
}

fn create_field(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    effect_index: u8,
    properties: u32,
) -> Result<(), String> {
    let catalog = world
        .get_resource::<AuraFieldCatalog>()
        .ok_or("Aura395 catalog not loaded")?;
    let definition = catalog.definition(properties)?;
    let radius = definition.radius_at(0.0);
    let trigger_spell = definition.trigger_spell;
    let (position, map) = read_attachment(world, aura.caster, target)
        .ok_or("Aura395 target needs a position and the same map copy as its caster")?;
    let now = crate::auras::aura_now(world);
    let binding = AuraAreaTrigger {
        target,
        instance_id: aura.instance_id,
        effect_index,
        properties,
        created_at: now,
    };
    let trigger = build_trigger(aura, trigger_spell, radius, now);
    world.spawn((binding, trigger, position, map));
    Ok(())
}

fn build_trigger(
    aura: &AuraInstance,
    trigger_spell: u32,
    radius: f32,
    now: f64,
) -> SpellAreaTrigger {
    SpellAreaTrigger {
        caster: aura.caster,
        spell_id: aura.spell_id,
        trigger_spell,
        radius,
        expires_at: aura.expires_at,
        period_ms: 0,
        next_tick_at: now,
        relation: targets::Relation::Enemy,
        depth: 1,
        cast_item_level: None,
        inside: HashSet::new(),
    }
}

fn read_attachment(world: &World, caster: Entity, target: Entity) -> Option<(Position, InMap)> {
    let map = *world.get::<InMap>(target)?;
    if world.get::<InMap>(caster) != Some(&map) {
        return None;
    }
    Some((*world.get::<Position>(target)?, map))
}

pub(crate) fn on_removed(world: &mut World, target: Entity, aura: &AuraInstance) {
    let fields: Vec<_> = world
        .query::<(Entity, &AuraAreaTrigger)>()
        .iter(world)
        .filter(|(_, binding)| binding.target == target && binding.instance_id == aura.instance_id)
        .map(|(entity, _)| entity)
        .collect();
    for field in fields {
        remove_field(world, field);
    }
}

pub(super) fn update(world: &mut World, entity: Entity) {
    let Some(binding) = world.get::<AuraAreaTrigger>(entity).cloned() else {
        return;
    };
    let Some(trigger) = world.get::<SpellAreaTrigger>(entity).cloned() else {
        return;
    };
    let now = crate::auras::aura_now(world);
    let Some(position) = read_live_attachment(world, entity, &binding, &trigger, now) else {
        remove_field(world, entity);
        return;
    };
    let radius = world
        .resource::<AuraFieldCatalog>()
        .definition(binding.properties)
        .expect("created field has a supported source definition")
        .radius_at(now - binding.created_at);
    world.entity_mut(entity).insert(position);
    world.get_mut::<SpellAreaTrigger>(entity).unwrap().radius = radius;
    update_actions(world, entity, &trigger);
}

fn read_live_attachment(
    world: &World,
    entity: Entity,
    binding: &AuraAreaTrigger,
    trigger: &SpellAreaTrigger,
    now: f64,
) -> Option<Position> {
    let expired = trigger.expires_at.is_some_and(|expiry| now >= expiry);
    if expired || !has_bound_effect(world, binding) {
        return None;
    }
    let (position, map) = read_attachment(world, trigger.caster, binding.target)?;
    (world.get::<InMap>(entity) == Some(&map)).then_some(position)
}

fn has_bound_effect(world: &World, binding: &AuraAreaTrigger) -> bool {
    world
        .get::<ActiveAuras>(binding.target)
        .and_then(|auras| auras.get(binding.instance_id))
        .is_some_and(|aura| {
            aura.effects.iter().any(|effect| {
                effect.index == binding.effect_index
                    && effect.aura_type == handlers::CREATE_AREATRIGGER
            })
        })
}

/// TC IsWithinDist adds the unit's source combat reach; the trigger has no unit reach.
pub(crate) fn collision_radius(world: &World, entity: Entity, unit: Entity) -> Option<f32> {
    let radius = world.get::<SpellAreaTrigger>(entity)?.radius;
    let reach = crate::scripted_ai::model_reach::combat_reach(world, unit)?;
    Some(radius + reach)
}

fn eligible_units(
    world: &mut World,
    entity: Entity,
    trigger: &SpellAreaTrigger,
) -> HashSet<Entity> {
    let center = *world
        .get::<Position>(entity)
        .expect("attached field position");
    let map = *world.get::<InMap>(entity).expect("attached field map");
    let candidates = living_units_in_map(world, map);
    candidates
        .into_iter()
        .filter_map(|(unit, position)| {
            if !targets::relation_matches(world, trigger.caster, unit, trigger.relation) {
                return None;
            }
            let Some(radius) = collision_radius(world, entity, unit) else {
                warn!(
                    ?unit,
                    "Aura395 enemy collision requires source model combat reach"
                );
                return None;
            };
            let at = Vec3::new(position.x, position.y, position.z);
            let origin = Vec3::new(center.x, center.y, center.z);
            (at.distance_squared(origin) < radius * radius).then_some(unit)
        })
        .collect()
}

fn living_units_in_map(world: &mut World, map: InMap) -> Vec<(Entity, Position)> {
    world
        .query::<(Entity, &Position, &Health, &InMap)>()
        .iter(world)
        .filter(|(_, _, health, copy)| health.current > 0.0 && **copy == map)
        .map(|(unit, position, _, _)| (unit, *position))
        .collect()
}

fn update_actions(world: &mut World, entity: Entity, previous: &SpellAreaTrigger) {
    let inside = eligible_units(world, entity, previous);
    for &unit in previous.inside.difference(&inside) {
        undo_action(world, previous, unit);
    }
    for &unit in inside.difference(&previous.inside) {
        crate::spell_cast::cast_triggered(
            world,
            previous.caster,
            previous.trigger_spell,
            Some(unit),
            previous.depth,
        );
    }
    if let Some(mut stored) = world.get_mut::<SpellAreaTrigger>(entity) {
        stored.inside = inside;
    }
}

fn undo_action(world: &mut World, trigger: &SpellAreaTrigger, target: Entity) {
    crate::auras::remove_auras(world, target, RemoveReason::Cancelled, |aura| {
        aura.spell_id == trigger.trigger_spell && aura.caster == trigger.caster
    });
}

fn remove_field(world: &mut World, entity: Entity) {
    let Some(trigger) = world.get::<SpellAreaTrigger>(entity).cloned() else {
        return;
    };
    for &unit in &trigger.inside {
        undo_action(world, &trigger, unit);
    }
    world.entity_mut(entity).despawn();
}
