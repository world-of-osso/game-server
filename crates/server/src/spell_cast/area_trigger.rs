//! Spell-created area triggers, distinct from static map/quest triggers.
//! TC a352b1fa Spell::EffectCreateAreaTrigger: one object at the destination
//! lasting CalcDuration; effect179 retains its explicit DB2 trigger binding,
//! except the finite Frozen Ground source-template binding.
//! Aura395 uses exact aura-effect ownership and supported source template actions.
//! Missing or unsupported source definitions never acquire guessed geometry.
mod aura_owned;
mod catalog;
mod frozen_ground;
mod vault_rune;

pub(crate) use aura_owned::{on_applied as on_aura_applied, on_removed as on_aura_removed};
pub(crate) use catalog::load_catalog as load_aura_field_catalog;

use super::effects::EffectCtx;
use super::targets::{self, Relation};
use crate::maps::InMap;
use crate::spell_info::{SpellEffect, SpellStore};
use bevy::prelude::*;
use shared::components::Position;
use std::collections::HashSet;

pub const EFFECT_CREATE_AREATRIGGER: u32 = 179;

/// Source-bound spheres add native unit reach; other effect179 keeps its spell radius.
pub(crate) fn avoidance_radius(world: &World, entity: Entity, unit: Entity) -> Option<f32> {
    if world.get::<aura_owned::AuraAreaTrigger>(entity).is_some()
        || world.get::<frozen_ground::FrozenGround>(entity).is_some()
    {
        return aura_owned::collision_radius(world, entity, unit);
    }
    Some(world.get::<SpellAreaTrigger>(entity)?.radius)
}

#[derive(Component, Debug, Clone)]
pub struct SpellAreaTrigger {
    pub caster: Entity,
    pub spell_id: u32,
    pub trigger_spell: u32,
    pub radius: f32,
    pub expires_at: Option<f64>,
    pub period_ms: u32,
    pub next_tick_at: f64,
    pub relation: Relation,
    pub depth: u8,
    pub cast_item_level: Option<u16>,
    pub inside: HashSet<Entity>,
}

pub(super) fn create_area_trigger(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    let Some(dest) = ctx.dest else {
        warn!(
            "Spell {}: area trigger {} has no destination",
            ctx.spell.id, effect.misc_values[0]
        );
        return;
    };
    if effect.misc_values[0] == crate::scripted_ai::kings_rest::GUST_SLASH_CREATE_PROPERTIES {
        let duration = super::duration_ms(world, ctx.caster, ctx.spell);
        crate::scripted_ai::kings_rest::create_gust_slash(
            world,
            ctx.caster,
            dest.position,
            dest.orientation,
            duration,
        );
        return;
    }
    if ctx.spell.id == frozen_ground::SPELL
        && effect.misc_values[0] == frozen_ground::CREATE_PROPERTIES
    {
        if let Err(reason) = frozen_ground::create(world, ctx, dest.position) {
            warn!(spell = ctx.spell.id, "Frozen Ground not created: {reason}");
        }
        return;
    }
    let Some(map) = world.get::<InMap>(ctx.caster).copied() else {
        return;
    };
    let now = crate::auras::aura_now(world);
    let duration = super::duration_ms(world, ctx.caster, ctx.spell);
    let radius = effect.radius.iter().flatten().next().map_or(0.0, |radius| {
        if radius.radius > 0.0 {
            radius.radius
        } else {
            radius.radius_max
        }
    });
    let relation = trigger_relation(world, effect);
    if effect.trigger_spell == 0 {
        warn!(
            "Spell {}: area trigger {} has no trigger binding; lifecycle only",
            ctx.spell.id, effect.misc_values[0]
        );
    }
    if radius <= 0.0 {
        warn!(
            "Spell {}: area trigger {} has no spell radius; area actions disabled",
            ctx.spell.id, effect.misc_values[0]
        );
    }
    let entity = world
        .spawn((
            SpellAreaTrigger {
                caster: ctx.caster,
                spell_id: ctx.spell.id,
                trigger_spell: effect.trigger_spell,
                radius,
                expires_at: (duration >= 0).then_some(now + f64::from(duration) / 1000.0),
                period_ms: effect.aura_period_ms,
                next_tick_at: now + f64::from(effect.aura_period_ms) / 1000.0,
                relation,
                depth: ctx.depth + 1,
                cast_item_level: ctx.spell.cast_item_level,
                inside: HashSet::new(),
            },
            Position {
                x: dest.position.x,
                y: dest.position.y,
                z: dest.position.z,
            },
            map,
        ))
        .id();
    update_trigger(world, entity);
}

fn trigger_relation(world: &World, effect: &SpellEffect) -> Relation {
    if let Some(spell) = world.resource::<SpellStore>().get(effect.trigger_spell) {
        if let Some(relation) = targets::explicit_requirement(spell.effects.iter()) {
            return relation;
        }
    }
    match effect.implicit_targets {
        [_, 16] | [16, _] | [53, _] => Relation::Enemy,
        [_, 31] | [31, _] | [132, _] => Relation::Ally,
        _ => Relation::Any,
    }
}

pub(super) fn update_area_triggers(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<SpellAreaTrigger>>()
        .iter(world)
        .collect();
    for entity in entities {
        update_trigger(world, entity);
    }
}

fn update_trigger(world: &mut World, entity: Entity) {
    if world.get::<frozen_ground::FrozenGround>(entity).is_some() {
        frozen_ground::update(world, entity);
        return;
    }
    if world.get::<aura_owned::AuraAreaTrigger>(entity).is_some() {
        aura_owned::update(world, entity);
        return;
    }
    let Some(trigger) = world.get::<SpellAreaTrigger>(entity).cloned() else {
        return;
    };
    let now = crate::auras::aura_now(world);
    let owner_map = world.get::<InMap>(trigger.caster);
    let trigger_map = world.get::<InMap>(entity);
    if trigger.expires_at.is_some_and(|expiry| now >= expiry)
        || owner_map.is_none()
        || owner_map != trigger_map
    {
        debug!("Despawn area trigger {entity}: spell {}", trigger.spell_id);
        world.entity_mut(entity).despawn();
        return;
    }
    if trigger.radius <= 0.0 || trigger.trigger_spell == 0 {
        return;
    }
    let position = world
        .get::<Position>(entity)
        .expect("area trigger position");
    let center = Vec3::new(position.x, position.y, position.z);
    let inside = targets::area_units(
        world,
        trigger.caster,
        center,
        trigger.radius,
        trigger.relation,
        0,
    );
    apply_trigger_spells(world, &trigger, &inside, now);
    if let Some(mut stored) = world.get_mut::<SpellAreaTrigger>(entity) {
        stored.inside = inside.into_iter().collect();
        if trigger.period_ms > 0 && now >= trigger.next_tick_at {
            stored.next_tick_at = now + f64::from(trigger.period_ms) / 1000.0;
        }
    }
}

fn apply_trigger_spells(
    world: &mut World,
    trigger: &SpellAreaTrigger,
    inside: &[Entity],
    now: f64,
) {
    let periodic = trigger.period_ms > 0;
    if periodic && now < trigger.next_tick_at {
        return;
    }
    for &unit in inside {
        if periodic || !trigger.inside.contains(&unit) {
            super::cast_triggered_with_item_level(
                world,
                trigger.caster,
                trigger.trigger_spell,
                Some(unit),
                trigger.depth,
                trigger.cast_item_level,
            );
        }
    }
}
