//! TC `InstanceScript::LoadDoorData`/`UpdateDoorState` for `OpenWhenDone` doors: the
//! door is ACTIVE (open, no collision) once its encounter is DONE, else READY.
use bevy::prelude::*;

use super::instance_script::{self, EncounterState};
use crate::maps::InMap;

struct Door {
    map_id: u32,
    entry: u32,
    encounter: u32,
}

const DOORS: &[Door] = &[
    // TC a352b1fa instance_darkflame_cleft.cpp doorData: GO_CATACOMBS_GATE, The Candle King.
    Door {
        map_id: 2651,
        entry: 430177,
        encounter: 2787,
    },
    // WAXBEARD_GATE_UNLOCK_LOG_CORRELATED: TC has no door data for 11DU_CATACOMBS_GATE01.
    // WCL WBYDm1N6JTZzCFnd fight 3 has no player past WoW y=-11.4 before Ol' Waxbeard's
    // kill and pull 5 at y=-123 beyond the gate. The exact retail trigger is unsourced.
    Door {
        map_id: 2651,
        entry: 420154,
        encounter: 2829,
    },
];

/// `AddDoor` on copy fill and `UpdateDoorState` on every boss state change.
pub(crate) fn update_doors(world: &mut World, map: InMap) {
    if !DOORS.iter().any(|door| door.map_id == map.map_id) {
        return;
    }
    let objects: Vec<(Entity, u32)> = world
        .query::<(Entity, &InMap, &shared::protocol::GameObjectInfo)>()
        .iter(world)
        .filter(|(_, copy, _)| **copy == map)
        .map(|(object, _, info)| (object, info.entry))
        .collect();
    for (object, entry) in objects {
        let doors: Vec<u32> = DOORS
            .iter()
            .filter(|door| door.map_id == map.map_id && door.entry == entry)
            .map(|door| door.encounter)
            .collect();
        if doors.is_empty() {
            continue;
        }
        let open = doors
            .into_iter()
            .all(|encounter| is_done(world, map, encounter));
        let state = if open {
            crate::game_objects::GO_STATE_ACTIVE
        } else {
            crate::game_objects::GO_STATE_READY
        };
        crate::game_objects::set_go_state(world, object, state);
    }
}

fn is_done(world: &mut World, map: InMap, encounter: u32) -> bool {
    instance_script::boss_id_for_encounter(world, map, encounter)
        .and_then(|boss_id| instance_script::get_boss_state(world, map, boss_id))
        == Some(EncounterState::Done)
}
