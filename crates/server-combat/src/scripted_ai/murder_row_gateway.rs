//! MURDER_ROW_DEMONIC_GATEWAY_LOG_CORRELATED: inferred automatic proximity edge.
use bevy::prelude::*;
use shared::components::{MovementControl, Player, Position};

use super::instance_script::{self, EncounterState};
use crate::maps::{InMap, TeleportOptions, WorldLocation};

#[cfg(test)]
#[path = "murder_row_gateway_tests.rs"]
mod tests;

pub(crate) const MAP: u32 = 2813;
const ZAEN_ENCOUNTER: u32 = 3102;
pub(crate) const SOURCE: Vec3 = Vec3::new(8878.12, 61.60762, 4777.14);
// The 20 yd activation radius is inferred, using the Uldaman/Deios proximity precedent.
pub(crate) const RADIUS: f32 = 20.0;

// Endpoints LOG_CORRELATED; flight duration unsourced (≤3.70 s, Thalecgos);
// TC no-ops without JumpChargeParams 1272. The approved edge is a near teleport.
pub(crate) fn landing() -> WorldLocation {
    WorldLocation {
        map_id: MAP,
        position: Vec3::new(8872.60, -4857.73, 100.26132),
        orientation: 4.6035,
    }
}

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
        let done = instance_script::boss_id_for_encounter(world, map, ZAEN_ENCOUNTER)
            .and_then(|id| instance_script::get_boss_state(world, map, id));
        if done != Some(EncounterState::Done) {
            continue;
        }
        let followers: Vec<_> = world
            .query::<(Entity, &crate::followers::Follower, &InMap)>()
            .iter(world)
            .filter_map(|(unit, follower, current)| {
                (follower.owner == owner
                    && *current == map
                    && crate::spell_cast::targets::is_alive(world, unit))
                .then_some(unit)
            })
            .collect();
        let destination = landing();
        if !crate::maps::teleport_to(world, owner, destination, TeleportOptions::default()) {
            error!(
                ?owner,
                ?map,
                "MURDER_ROW_DEMONIC_GATEWAY_LOG_CORRELATED teleport refused"
            );
            continue;
        }
        for follower in followers {
            crate::maps::transfer::teleport_creature_with_companions(world, follower, &destination);
        }
        info!(?owner, ?map, "MURDER_ROW_DEMONIC_GATEWAY_LOG_CORRELATED");
    }
}
