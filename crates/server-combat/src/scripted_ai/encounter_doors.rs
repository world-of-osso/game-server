//! TC a352b1fa `InstanceScript::LoadDoorData`/`UpdateDoorState`: ported scripts'
//! `doorData[]`, keyed by `DungeonEncounter` ID because runtime boss ids are DB2 bits.
use std::collections::HashMap;

use bevy::prelude::*;
use shared::protocol::GameObjectInfo;

use super::instance_script::{EncounterState, instance_script};
use crate::game_objects::{
    GO_STATE_ACTIVE, GO_STATE_READY, GameObjectState, set_go_state, set_initial_go_state,
};
use crate::maps::InMap;

/// TC `EncounterDoorBehavior`; variants are added with the first door that uses them.
#[derive(Clone, Copy)]
enum Behavior {
    OpenWhenDone,
}

impl Behavior {
    fn opens(self, state: EncounterState) -> bool {
        match self {
            Self::OpenWhenDone => state == EncounterState::Done,
        }
    }
}

struct Door {
    map_id: u32,
    entry: u32,
    encounter_id: u32,
    behavior: Behavior,
}

/// Append-only door tables of ported instance scripts.
const DOORS: &[Door] = &[
    // instance_ruby_life_pools.cpp: GO_FIRE_WALL, DATA_KOKIA_BLAZEHOOF (DungeonEncounter 2606).
    Door {
        map_id: 2521,
        entry: 377194,
        encounter_id: 2606,
        behavior: Behavior::OpenWhenDone,
    },
];

/// `UpdateDoorState` for every door object of `map`'s copy: open only while every
/// encounter bound to its entry allows it.
pub(crate) fn update_doors(world: &mut World, map: InMap) {
    reconcile_doors(world, map, false);
}

pub(crate) fn initialize_doors(world: &mut World, map: InMap) {
    reconcile_doors(world, map, true);
}

fn reconcile_doors(world: &mut World, map: InMap, initial: bool) {
    if !DOORS.iter().any(|door| door.map_id == map.map_id) {
        return;
    }
    let Some(script) = instance_script(world, map) else {
        error!("Map {map:?} has door data but no instance script");
        return;
    };
    let states: HashMap<u32, EncounterState> = script
        .bosses
        .iter()
        .map(|boss| (boss.encounter_id, boss.state))
        .collect();
    let objects: Vec<(Entity, u32, Option<u8>)> = world
        .query::<(Entity, &InMap, &GameObjectInfo, Option<&GameObjectState>)>()
        .iter(world)
        .filter(|(_, copy, ..)| **copy == map)
        .map(|(object, _, info, state)| (object, info.entry, state.map(|state| state.0)))
        .collect();
    for (object, entry, current) in objects {
        let Some(open) = door_open(&states, map.map_id, entry) else {
            continue;
        };
        let state = if open {
            GO_STATE_ACTIVE
        } else {
            GO_STATE_READY
        };
        if current != Some(state) {
            if initial {
                set_initial_go_state(world, object, state);
            } else {
                set_go_state(world, object, state);
            }
        }
    }
}

/// `None` for objects that are not doors of this map.
fn door_open(states: &HashMap<u32, EncounterState>, map_id: u32, entry: u32) -> Option<bool> {
    let mut doors = DOORS
        .iter()
        .filter(|door| door.map_id == map_id && door.entry == entry)
        .peekable();
    doors.peek()?;
    Some(doors.all(|door| {
        let Some(&state) = states.get(&door.encounter_id) else {
            error!(
                "Door {} of map {map_id} names encounter {} absent from its instance script",
                door.entry, door.encounter_id
            );
            return false;
        };
        door.behavior.opens(state)
    }))
}
