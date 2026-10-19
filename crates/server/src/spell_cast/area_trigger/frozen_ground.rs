//! Native387149/26505/30679: fixed destination, source scale and CAST387150/ENEMY.
//! The applied aura owns its periodic clock; entry never invents a damage tick.
use super::catalog::AuraFieldCatalog;
use super::{SpellAreaTrigger, aura_owned};
use crate::maps::InMap;
use crate::spell_cast::effects::EffectCtx;
use bevy::prelude::*;
use shared::components::Position;
use std::collections::HashSet;

pub(super) const SPELL: u32 = 387149;
pub(super) const CREATE_PROPERTIES: i32 = 26505;

#[derive(Component, Clone, Copy)]
pub(super) struct FrozenGround {
    created_at: f64,
}

pub(super) fn create(world: &mut World, ctx: &EffectCtx, destination: Vec3) -> Result<(), String> {
    let map = *world.get::<InMap>(ctx.caster).ok_or("caster has no map")?;
    let now = crate::auras::aura_now(world);
    let trigger = build_trigger(world, ctx, now)?;
    let entity = world
        .spawn((
            FrozenGround { created_at: now },
            trigger,
            Position {
                x: destination.x,
                y: destination.y,
                z: destination.z,
            },
            map,
        ))
        .id();
    update(world, entity);
    Ok(())
}

fn build_trigger(world: &World, ctx: &EffectCtx, now: f64) -> Result<SpellAreaTrigger, String> {
    let definition = world
        .get_resource::<AuraFieldCatalog>()
        .ok_or("source field catalog not loaded")?
        .definition(CREATE_PROPERTIES as u32)?;
    let duration = crate::spell_cast::duration_ms(world, ctx.caster, ctx.spell);
    Ok(SpellAreaTrigger {
        caster: ctx.caster,
        spell_id: ctx.spell.id,
        trigger_spell: definition.trigger_spell,
        radius: definition.radius_at(0.0),
        expires_at: (duration >= 0).then_some(now + f64::from(duration) / 1000.0),
        period_ms: 0,
        next_tick_at: now,
        relation: definition.relation,
        depth: ctx.depth + 1,
        cast_item_level: ctx.spell.cast_item_level,
        inside: HashSet::new(),
    })
}

pub(super) fn radius_at(world: &World, entity: Entity, at: f64) -> Option<f32> {
    let binding = world.get::<FrozenGround>(entity)?;
    Some(
        world
            .resource::<AuraFieldCatalog>()
            .definition(CREATE_PROPERTIES as u32)
            .expect("created Frozen Ground has supported source definition")
            .radius_at(at - binding.created_at),
    )
}

pub(super) fn update(world: &mut World, entity: Entity) {
    let binding = *world
        .get::<FrozenGround>(entity)
        .expect("Frozen Ground binding");
    let trigger = world
        .get::<SpellAreaTrigger>(entity)
        .expect("Frozen Ground trigger")
        .clone();
    let now = crate::auras::aura_now(world);
    let expired = trigger.expires_at.is_some_and(|expiry| now >= expiry);
    let caster_map = world.get::<InMap>(trigger.caster);
    if expired || caster_map.is_none() || caster_map != world.get::<InMap>(entity) {
        remove(world, entity, &trigger);
        return;
    }
    let radius = world
        .resource::<AuraFieldCatalog>()
        .definition(CREATE_PROPERTIES as u32)
        .expect("created Frozen Ground has supported source definition")
        .radius_at(now - binding.created_at);
    world.get_mut::<SpellAreaTrigger>(entity).unwrap().radius = radius;
    aura_owned::update_actions(world, entity, &trigger);
}

fn remove(world: &mut World, entity: Entity, trigger: &SpellAreaTrigger) {
    for &unit in &trigger.inside {
        aura_owned::undo_action(world, trigger, unit);
    }
    world.entity_mut(entity).despawn();
}
