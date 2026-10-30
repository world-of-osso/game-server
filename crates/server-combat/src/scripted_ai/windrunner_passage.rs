//! WINDRUNNER_TAXIPATH_PASSAGE_LOG_CORRELATED: inferred proximity activation.
use bevy::prelude::*;
use shared::components::{MovementControl, Player, Position};

use super::instance_script::{self, EncounterState};
use crate::maps::InMap;

#[cfg(test)]
#[path = "windrunner_passage_tests.rs"]
mod tests;

pub(crate) const MAP: u32 = 2805;
pub(crate) const RADIUS: f32 = 20.0;

pub(crate) struct Passage {
    pub path: u32,
    pub encounter: u32,
    pub source: Vec3,
}

// Native TaxiPath node 0; WCL X4tT9P73BvRrGxyM fight 1 and
// jpfFa36xhQkZqr4d fight 6 correlate the routes to player movement.
// The automatic trigger and radius are inferred, not authored identities.
pub(crate) const PASSAGES: &[Passage] = &[
    Passage {
        path: 10301,
        encounter: 3056,
        source: Vec3::new(5346.42871, 94.4079361, 3304.83521),
    },
    Passage {
        path: 10303,
        encounter: 3057,
        source: Vec3::new(5048.79346, 107.856323, 3150.79175),
    },
    Passage {
        path: 10308,
        encounter: 3058,
        source: Vec3::new(5214.27783, 100.708763, 3126.375),
    },
];

/// Start a party flight only after the encounter preceding its passage.
pub(crate) fn update(world: &mut World) {
    let candidates: Vec<_> = world
        .query_filtered::<(Entity, &Position, &InMap, Option<&MovementControl>), With<Player>>()
        .iter(world)
        .filter_map(|(entity, position, map, control)| {
            (map.map_id == MAP
                && control.is_none_or(|control| !control.controlled)
                && crate::spell_cast::targets::is_alive(world, entity))
            .then_some((entity, *map, Vec3::new(position.x, position.y, position.z)))
        })
        .collect();
    for (owner, map, position) in candidates {
        for passage in PASSAGES {
            if position.distance(passage.source) > RADIUS {
                continue;
            }
            let done = instance_script::boss_id_for_encounter(world, map, passage.encounter)
                .and_then(|id| instance_script::get_boss_state(world, map, id));
            if done == Some(EncounterState::Done) {
                crate::taxi::start_windrunner_flight(world, owner, map, passage.path);
                break;
            }
        }
    }
}
