//! Instrument of Retribution 404752: a nearby group player's death grants Wrath.
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health, Player, Position};

use crate::group::GroupRegistry;
use crate::maps::InMap;
use crate::persistence::ClientId;
use crate::spell_info::SpellStore;

const INSTRUMENT: u32 = 404752;
const WRATH: u32 = 31884;
const RETRIBUTION: u32 = 70;

#[cfg(test)]
#[path = "instrument_of_retribution_tests.rs"]
mod tests;

pub(super) fn on_player_death(world: &mut World, deceased: Entity) {
    let Some((dead_id, dead_position, dead_map)) = world.get::<Player>(deceased).and_then(|_| {
        Some((
            world.get::<ClientId>(deceased)?.0,
            *world.get::<Position>(deceased)?,
            *world.get::<InMap>(deceased)?,
        ))
    }) else {
        return;
    };
    let Some(store) = world.get_resource::<SpellStore>() else {
        return;
    };
    let Some(effect) = store
        .get(INSTRUMENT)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
    else {
        return;
    };
    let Some(radius) = effect.radius[0] else {
        return;
    };
    let duration_ms = (effect.base_points * 1000.0) as u32;
    let Some(groups) = world.get_resource::<GroupRegistry>() else {
        return;
    };
    let candidates: Vec<Entity> = world
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<ClientId>()?.0;
            let position = entity.get::<Position>()?;
            let map = entity.get::<InMap>()?;
            let spec = entity.get::<ActiveSpec>()?;
            let health = entity.get::<Health>()?;
            let distance_squared = (position.x - dead_position.x).powi(2)
                + (position.y - dead_position.y).powi(2)
                + (position.z - dead_position.z).powi(2);
            (entity.id() != deceased
                && entity.contains::<Player>()
                && spec.0 == RETRIBUTION
                && health.current > 0.0
                && map.map_id == dead_map.map_id
                && map.instance_id == dead_map.instance_id
                && distance_squared <= radius.radius.powi(2)
                && groups.is_in_same_group(dead_id, id))
            .then_some(entity.id())
        })
        .collect();
    for paladin in candidates {
        if !crate::spell_cast::caster_knows_spell(world, paladin, INSTRUMENT) {
            continue;
        }
        crate::spell_cast::cast_triggered_with_overrides(
            world,
            paladin,
            WRATH,
            Some(paladin),
            1,
            crate::spell_cast::TriggeredSpellOverrides {
                fixed_aura_duration_ms: Some(duration_ms),
                ..default()
            },
        );
    }
}
