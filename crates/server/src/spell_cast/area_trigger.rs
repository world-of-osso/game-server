//! Source-backed spell-created area triggers, distinct from static map/quest triggers.
//! Effect179 selects supported destination definitions by features, never spell IDs.
//! Aura395 retains exact aura-effect ownership and supported source template actions.
//! Missing or unsupported definitions never acquire guessed geometry.
mod aura_owned;
mod catalog;
mod destination_fields;
mod vault_rune;

pub(crate) use aura_owned::{on_applied as on_aura_applied, on_removed as on_aura_removed};
pub(crate) use catalog::load_catalog as load_aura_field_catalog;

use super::effects::EffectCtx;
use super::targets::{self, Relation};
use crate::spell_info::SpellEffect;
use bevy::prelude::*;
use std::collections::HashSet;

pub const EFFECT_CREATE_AREATRIGGER: u32 = 179;

/// Source-bound spheres add native reach; only authored native curves advance in time.
pub(crate) fn avoidance_radius_at(
    world: &World,
    entity: Entity,
    unit: Entity,
    at: f64,
) -> Option<f32> {
    let radius = if world
        .get::<destination_fields::DestinationField>(entity)
        .is_some()
    {
        destination_fields::radius_at(world, entity, at)?
    } else {
        aura_owned::radius_at(world, entity, at)?
    };
    Some(radius + crate::scripted_ai::model_reach::combat_reach(world, unit)?)
}

#[derive(Component, Debug, Clone)]
pub struct SpellAreaTrigger {
    pub caster: Entity,
    pub spell_id: u32,
    pub trigger_spell: u32,
    pub radius: f32,
    pub expires_at: Option<f64>,
    pub relation: Relation,
    pub depth: u8,
    pub cast_item_level: Option<u16>,
    pub inside: HashSet<Entity>,
}

pub(super) fn create_area_trigger(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect) {
    super::efflorescence::remove_previous_area_trigger(world, ctx.caster, ctx.spell.id);
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
    let result = u32::try_from(effect.misc_values[0])
        .map_err(|_| "negative create-properties".to_owned())
        .and_then(|properties| destination_fields::create(world, ctx, dest.position, properties));
    if let Err(reason) = result {
        warn!(
            spell = ctx.spell.id,
            properties = effect.misc_values[0],
            "Source destination field not created: {reason}"
        );
    }
}

pub(super) fn update_area_triggers(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<SpellAreaTrigger>>()
        .iter(world)
        .collect();
    for entity in entities {
        if world
            .get::<destination_fields::DestinationField>(entity)
            .is_some()
        {
            destination_fields::update(world, entity);
        } else {
            aura_owned::update(world, entity);
        }
    }
}
