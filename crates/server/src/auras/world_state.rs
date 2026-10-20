//! Server-owned movement permissions and aura teleport origins. Wire/engine projection is separate.
use std::collections::HashMap;

use bevy::prelude::*;
use shared::components::{Player, Position, Rotation};

use super::{ActiveAuras, AuraInstance, RemoveReason, handlers};

use crate::maps::InMap;

/// Movement permissions; kept separate from flight and feather fall.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct WorldAuraState {
    pub water_walk: bool,
    pub hover: bool,
    pub disable_gravity: bool,
    pub tracked_creatures: u32,
    pub movement_force_multiplier: f32,
}

/// The actual map copy and pose at application, keyed by the storing spell ID.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TeleportOrigin {
    pub map: InMap,
    pub position: Position,
    pub yaw: f32,
}

#[derive(Component, Debug, Default)]
pub(crate) struct AuraTeleportOrigins(pub HashMap<u32, TeleportOrigin>);

pub(super) fn reconcile(world: &mut World, unit: Entity) {
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return;
    };
    let player = world.get::<Player>(unit).is_some();
    let tracked_creatures = auras
        .iter()
        .flat_map(|a| &a.effects)
        .filter(|e| player && e.aura_type == handlers::TRACK_CREATURES)
        .filter_map(|e| u32::try_from(e.misc_values[0] - 1).ok())
        .filter_map(|bit| 1u32.checked_shl(bit))
        .fold(0, |mask, flag| mask | flag);
    let movement_force_multiplier = auras
        .iter()
        .flat_map(|a| {
            a.effects
                .iter()
                .filter(|e| e.aura_type == handlers::MOD_MOVEMENT_FORCE_MAGNITUDE)
                .map(|e| 1.0 + e.amount * a.stack_multiplier() / 100.0)
        })
        .product();
    let next = WorldAuraState {
        water_walk: auras.has_type(handlers::WATER_WALK),
        hover: auras.has_type(handlers::HOVER),
        disable_gravity: auras.has_type(handlers::DISABLE_GRAVITY)
            || auras.has_type(297)
            || auras.has_type(298),
        tracked_creatures,
        movement_force_multiplier,
    };
    if world.get::<WorldAuraState>(unit) != Some(&next) {
        world.entity_mut(unit).insert(next);
    }
}

/// REAL-only: refreshes do not overwrite the original teleport point.
pub(super) fn on_applied(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.effects.iter().any(|e| {
        e.aura_type == handlers::MOD_POWER_DISPLAY
            && shared::components::PowerType::from_db(e.misc_values[0]).is_some()
    }) {
        super::remove_auras(world, unit, RemoveReason::Replaced, |other| {
            other.instance_id != aura.instance_id
                && other
                    .effects
                    .iter()
                    .any(|e| e.aura_type == handlers::MOD_POWER_DISPLAY)
        });
    }
    if world.get::<Player>(unit).is_none() || !stores_return_point(aura) {
        return;
    }
    let (Some(map), Some(position), Some(rotation)) = (
        world.get::<InMap>(unit).copied(),
        world.get::<Position>(unit).copied(),
        world.get::<Rotation>(unit).copied(),
    ) else {
        warn!(
            "Cannot store aura{} teleport origin for {unit:?}: missing map/pose",
            aura.spell_id
        );
        return;
    };
    let yaw = rotation.y;
    world
        .entity_mut(unit)
        .entry::<AuraTeleportOrigins>()
        .or_default()
        .into_mut()
        .0
        .insert(aura.spell_id, TeleportOrigin { map, position, yaw });
}

pub(super) fn on_removed(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if stores_return_point(aura)
        && let Some(mut origins) = world.get_mut::<AuraTeleportOrigins>(unit)
    {
        origins.0.remove(&aura.spell_id);
    }
}

fn stores_return_point(aura: &AuraInstance) -> bool {
    aura.effects
        .iter()
        .any(|e| e.aura_type == handlers::STORE_TELEPORT_RETURN_POINT)
}
