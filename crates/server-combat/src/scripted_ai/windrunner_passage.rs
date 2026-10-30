//! WINDRUNNER_TAXIPATH_10308_PASSAGE_LOG_CORRELATED: inferred proximity activation.
use bevy::prelude::*;
use shared::components::{MovementControl, Player, Position};

use super::instance_script::{self, EncounterState};
use crate::maps::InMap;

#[cfg(test)]
#[path = "windrunner_passage_tests.rs"]
mod tests;

pub(crate) const MAP: u32 = 2805;
pub(crate) const PATH: u32 = 10308;
pub(crate) const SOURCE: Vec3 = Vec3::new(5214.27783, 100.708763, 3126.375);
pub(crate) const RADIUS: f32 = 20.0;
const KROLUK: u32 = 3058;

/// WCL jpfFa36xhQkZqr4d fight 6 follows native path 10308 after Kroluk;
/// the 20-yard automatic trigger is an approved inference, not an authored identity.
pub(crate) fn update(world: &mut World) {
    let candidates: Vec<_> = world
        .query_filtered::<(Entity, &Position, &InMap, Option<&MovementControl>), With<Player>>()
        .iter(world)
        .filter_map(|(entity, position, map, control)| {
            (map.map_id == MAP
                && Vec3::new(position.x, position.y, position.z).distance(SOURCE) <= RADIUS
                && control.is_none_or(|control| !control.controlled)
                && crate::spell_cast::targets::is_alive(world, entity))
            .then_some((entity, *map))
        })
        .collect();
    for (owner, map) in candidates {
        let done = instance_script::boss_id_for_encounter(world, map, KROLUK)
            .and_then(|id| instance_script::get_boss_state(world, map, id));
        if done == Some(EncounterState::Done) {
            crate::taxi::start_windrunner_flight(world, owner, map);
        }
    }
}
