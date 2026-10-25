//! 27 PERSISTENT_AREA_AURA (`Spell::EffectPersistentAreaAura`): a dynamic object at
//! the effect's destination with the effect's radius (`CalcRadius`) and the spell's
//! duration. Every tick (`DynObjAura::FillTargetMap`) the units within the radius
//! that TargetB selects (28 DEST_DYNOBJ_ENEMY: the caster's enemies) carry the
//! spell's aura, sharing the object's expiry and tick phase
//! (`auras::align_area_aura`); a unit that leaves the area loses it. The object and
//! its auras end with its duration or its caster.

use bevy::prelude::*;
use lightyear::prelude::{NetworkTarget, Replicate};
use shared::components::{DynamicObjectInfo, Position, RAIN_OF_FIRE_SPELL_ID};

use super::effects::EffectCtx;
use super::targets::{self, Relation};
use crate::auras::{AuraApplication, AuraSource, RemoveReason};
use crate::maps::InMap;
use crate::spell_info::SpellEffect;

/// A persistent area aura's `DynamicObject`.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct DynamicObject {
    pub caster: Entity,
    pub spell_id: u32,
    /// The spell's PERSISTENT_AREA_AURA effects.
    pub effect_mask: u32,
    pub radius: f32,
    pub relation: Relation,
    /// Aura time (`auras::aura_now`).
    pub started_at: f64,
    pub expires_at: f64,
    /// Units carrying its aura.
    pub affected: Vec<Entity>,
}

/// TargetB of a persistent area aura: which units its object affects
/// (28 TARGET_DEST_DYNOBJ_ENEMY, 29 TARGET_DEST_DYNOBJ_ALLY, else any).
fn dynobj_relation(effect: &SpellEffect) -> Relation {
    match effect.implicit_targets[1] {
        28 => Relation::Enemy,
        29 => Relation::Ally,
        _ => Relation::Any,
    }
}

pub(crate) fn create(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    let Some(dest) = ctx.dest else {
        debug!(
            "Spell {}: PERSISTENT_AREA_AURA without a destination",
            ctx.spell.id
        );
        return;
    };
    let Some(map) = world.get::<InMap>(ctx.caster).copied() else {
        return;
    };
    let duration_ms = super::duration_ms(world, ctx.caster, ctx.spell).max(0);
    let now = crate::auras::aura_now(world);
    let radius = targets::effect_radius(effect);
    let object = DynamicObject {
        caster: ctx.caster,
        spell_id: ctx.spell.id,
        effect_mask: 1 << effect.index,
        radius,
        relation: dynobj_relation(effect),
        started_at: now,
        expires_at: now + f64::from(duration_ms as u32) / 1000.0,
        affected: Vec::new(),
    };
    let entity = world
        .spawn((
            object,
            Position {
                x: dest.position.x,
                y: dest.position.y,
                z: dest.position.z,
            },
            map,
        ))
        .id();
    if ctx.spell.id == RAIN_OF_FIRE_SPELL_ID {
        world.entity_mut(entity).insert((
            DynamicObjectInfo {
                caster: ctx.caster.to_bits(),
                spell_id: ctx.spell.id,
                radius,
            },
            Replicate::to_clients(NetworkTarget::All),
            crate::interest::NetworkVisibility::default(),
        ));
    }
    info!(
        "Spell {} of {}: dynamic object {entity} at {:?}, radius {}",
        ctx.spell.id, ctx.caster, dest.position, radius
    );
    update_object(world, entity);
}

pub(crate) fn update_dynamic_objects(world: &mut World) {
    let objects: Vec<Entity> = world
        .query_filtered::<Entity, With<DynamicObject>>()
        .iter(world)
        .collect();
    for object in objects {
        update_object(world, object);
    }
}

fn update_object(world: &mut World, entity: Entity) {
    let Some(object) = world.get::<DynamicObject>(entity).cloned() else {
        return;
    };
    let now = crate::auras::aura_now(world);
    let caster_gone = world.get_entity(object.caster).is_err();
    if caster_gone || now >= object.expires_at {
        for unit in &object.affected {
            remove_aura(world, &object, *unit);
        }
        world.entity_mut(entity).despawn();
        return;
    }
    let center = world
        .get::<Position>(entity)
        .map(|pos| Vec3::new(pos.x, pos.y, pos.z))
        .unwrap_or_default();
    let inside = targets::area_units(
        world,
        object.caster,
        center,
        object.radius,
        object.relation,
        0,
    );
    for &unit in inside.iter().filter(|unit| !object.affected.contains(unit)) {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster: object.caster,
                target: unit,
                spell_id: object.spell_id,
                effect_mask: object.effect_mask,
                source: AuraSource::Cast,
            },
        );
        crate::auras::align_area_aura(
            world,
            unit,
            object.caster,
            object.spell_id,
            object.started_at,
            object.expires_at,
        );
    }
    for &unit in object.affected.iter().filter(|unit| !inside.contains(unit)) {
        remove_aura(world, &object, unit);
    }
    if let Some(mut stored) = world.get_mut::<DynamicObject>(entity) {
        stored.affected = inside;
    }
}

fn remove_aura(world: &mut World, object: &DynamicObject, unit: Entity) {
    if world.get_entity(unit).is_err() {
        return;
    }
    crate::auras::remove_auras(world, unit, RemoveReason::Expired, |aura| {
        aura.spell_id == object.spell_id && aura.caster == object.caster
    });
}
