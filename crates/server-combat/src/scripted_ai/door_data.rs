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
    // AVANOXX_SPIKE_WALL_OPEN_LOG_CORRELATED: WCL qLNnp4x613mv2Ctz fight 28
    // player 9 casts at (-398.47, -109.6) before Avanoxx ends, then at
    // (-590.77, -68.63) 21.558s after it ends, beyond the wall at x=-521.234.
    // WCL coordinates are (engine z, engine x); the exact door trigger is unknown.
    Door {
        map_id: 2660,
        entry: 455376,
        encounter: 2926,
    },
    Door {
        map_id: 2660,
        entry: 455377,
        encounter: 2926,
    },
    Door {
        map_id: 2660,
        entry: 455378,
        encounter: 2926,
    },
    // AVANOXX_WEB_BARRIER_OPEN_LOG_CORRELATED: WCL XCmNaPcWdFA4kJ8D fight 9,
    // pull 4 actor 91 crosses within 2 yd of GO 467576 before Anub'zekt starts,
    // after Avanoxx ends. The exact retail trigger is unknown.
    Door {
        map_id: 2660,
        entry: 467576,
        encounter: 2926,
    },
    // ANUBZEKT_SPIKE_WALL_OPEN_LOG_CORRELATED: WCL XCmNaPcWdFA4kJ8D fight 9
    // pull 5 has casts near GO 455375 during Anub'zekt; pull 7 has a player
    // beyond it 15.318s after the encounter ends. The exact trigger is unknown.
    Door {
        map_id: 2660,
        entry: 455375,
        encounter: 2906,
    },
    // TC 15e0ed76 instance_the_stonevault.cpp doorData: both paths open when E.D.N.A. is Done.
    Door {
        map_id: 2652,
        entry: 440235,
        encounter: 2854,
    },
    Door {
        map_id: 2652,
        entry: 440239,
        encounter: 2854,
    },
    // NALORAKK_WATER_DOOR_OPENS_ON_HOARDMONGER_DONE_LOG_CORRELATED:
    // Retail WoWCombatLog-102026_223255.txt lines 65154, 65165, 65364
    // show encounter success and a player crossing the door plane 7.629s later.
    // The door state and exact trigger are not recorded.
    Door {
        map_id: 2825,
        entry: 531150,
        encounter: 3207,
    },
    // ACADEMY_BRAMBLE_BARRIER_UNLOCK_LOG_CORRELATED: WCL gWkPyzwJ2mbxRtdr fight 3
    // and tjnzpa3Axg6QLHfJ fight 1 place players past this door after all three
    // encounters. The three-boss gate is project policy; Retail's trigger is unknown.
    Door {
        map_id: 2526,
        entry: 382475,
        encounter: 2562,
    },
    Door {
        map_id: 2526,
        entry: 382475,
        encounter: 2563,
    },
    Door {
        map_id: 2526,
        entry: 382475,
        encounter: 2564,
    },
    // ULDAMAN_TALONDRAS_DOOR_PASSAGE_INFERRED: the TDB door blocks the only
    // nav route from Talondras to Emberon; its exact Retail trigger is unsourced.
    Door {
        map_id: 2451,
        entry: 376554,
        encounter: 2557,
    },
    // ULDAMAN_EMBERON_VAULT_DOOR_PASSAGE_INFERRED: the vault entrance
    // blocks the route after Talondras; its exact Retail trigger is unsourced.
    Door {
        map_id: 2451,
        entry: 376191,
        encounter: 2557,
    },
    // ULDAMAN_EMBERON_EXIT_DOOR_PASSAGE_INFERRED: the vault exit blocks the
    // route to Deios after Emberon; its exact Retail trigger is unsourced.
    Door {
        map_id: 2451,
        entry: 376640,
        encounter: 2558,
    },
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
    // BLAZIKON_DOOR_UNLOCK_LOG_CORRELATED: TC has no door data for 11DU_Foundry_Door01_Large.
    // WCL WBYDm1N6JTZzCFnd fight 3 pulls 5-8 (through Blazikon's kill) have no player west
    // of WoW x=371.5; pull 9 reaches x=224 beyond the door at x=336.8. Trigger unsourced.
    Door {
        map_id: 2651,
        entry: 430175,
        encounter: 2826,
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
