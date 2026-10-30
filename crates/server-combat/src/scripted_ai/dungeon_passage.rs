//! LOG_CORRELATED dungeon passages with inferred proximity activation.
use bevy::prelude::*;
use shared::components::{MovementControl, Player, Position};

use super::instance_script::{self, EncounterState};
use crate::maps::InMap;

#[cfg(test)]
#[path = "dungeon_passage_tests.rs"]
mod tests;

pub(crate) const RADIUS: f32 = 20.0;

#[derive(Clone, Copy)]
pub(crate) enum PassageKind {
    Taxi(u32),
    Drop { landing: Vec3 },
}

pub(crate) struct Passage {
    pub map: u32,
    pub kind: PassageKind,
    pub encounter: u32,
    pub source: Vec3,
}

// Native TaxiPath node 0; WCL X4tT9P73BvRrGxyM fight 1 and
// jpfFa36xhQkZqr4d fight 6 correlate the routes to player movement.
// The automatic trigger and radius are inferred, not authored identities.
pub(crate) const PASSAGES: &[Passage] = &[
    Passage {
        map: 2805,
        kind: PassageKind::Taxi(10301),
        encounter: 3056,
        source: Vec3::new(5346.42871, 94.4079361, 3304.83521),
    },
    Passage {
        map: 2805,
        kind: PassageKind::Taxi(10303),
        encounter: 3057,
        source: Vec3::new(5048.79346, 107.856323, 3150.79175),
    },
    Passage {
        map: 2805,
        kind: PassageKind::Taxi(10308),
        encounter: 3058,
        source: Vec3::new(5214.27783, 100.708763, 3126.375),
    },
    // NEXUS_POINT_TAXIPATH_PASSAGE_LOG_CORRELATED: five WCL player paths match 11073.
    Passage {
        map: 2915,
        kind: PassageKind::Taxi(11073),
        encounter: 3332,
        source: Vec3::new(482.428833, 163.467117, 10.8732643),
    },
    // FLOODGATE_DROP_LOG_CORRELATED: WCL 67c3WanVygd4ZvCt/1,
    // XCmNaPcWdFA4kJ8D/32, and hjF7fyTxVNdBnWbJ/2.
    Passage {
        map: 2773,
        kind: PassageKind::Drop {
            landing: Vec3::new(1524.25, 275.31427, 2746.25),
        },
        encounter: 3053,
        source: Vec3::new(1529.167, 334.6686, 2746.0),
    },
];

/// Start a passage only after the encounter preceding it.
pub(crate) fn update(world: &mut World) {
    let candidates: Vec<_> = world
        .query_filtered::<(Entity, &Position, &InMap, Option<&MovementControl>), With<Player>>()
        .iter(world)
        .filter_map(|(entity, position, map, control)| {
            (PASSAGES.iter().any(|passage| passage.map == map.map_id)
                && control.is_none_or(|control| !control.controlled)
                && world
                    .get::<crate::navigation::collision_drop::CollisionDropMovement>(entity)
                    .is_none()
                && crate::spell_cast::targets::is_alive(world, entity))
            .then_some((entity, *map, Vec3::new(position.x, position.y, position.z)))
        })
        .collect();
    for (owner, map, position) in candidates {
        for passage in PASSAGES {
            if passage.map != map.map_id || position.distance(passage.source) > RADIUS {
                continue;
            }
            let done = instance_script::boss_id_for_encounter(world, map, passage.encounter)
                .and_then(|id| instance_script::get_boss_state(world, map, id));
            if done == Some(EncounterState::Done) {
                match passage.kind {
                    PassageKind::Taxi(path) => {
                        crate::taxi::start_dungeon_passage_flight(world, owner, map, path)
                    }
                    PassageKind::Drop { landing } => {
                        start_party_drop(world, owner, map, passage.source, landing)
                    }
                }
                break;
            }
        }
    }
}

fn start_party_drop(world: &mut World, owner: Entity, map: InMap, source: Vec3, landing: Vec3) {
    let followers: Vec<_> = world
        .query::<(Entity, &crate::followers::Follower, &InMap)>()
        .iter(world)
        .filter_map(|(entity, follower, current)| {
            (follower.owner == owner && *current == map).then_some(entity)
        })
        .collect();
    let mut riders = vec![owner];
    riders.extend(followers);
    riders.retain(|&rider| crate::spell_cast::targets::is_alive(world, rider));
    let pets: Vec<_> = riders
        .iter()
        .flat_map(|&rider| crate::maps::transfer::owned_companions(world, rider))
        .collect();
    riders.extend(pets);
    riders.retain(|&rider| {
        world.get::<InMap>(rider) == Some(&map)
            && crate::spell_cast::targets::is_alive(world, rider)
    });
    if riders.iter().any(|&rider| {
        world.get::<Position>(rider).is_none()
            || world
                .get::<crate::navigation::collision_drop::CollisionDropMovement>(rider)
                .is_some()
            || world
                .get::<MovementControl>(rider)
                .is_some_and(|control| control.controlled)
    }) {
        return;
    }
    for rider in riders {
        crate::navigation::collision_drop::start_passage_drop(world, rider, map, source, landing);
    }
}
