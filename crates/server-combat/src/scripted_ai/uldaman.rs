//! ULDAMAN_PREHISTORIA_DOOR_LITTLEWIGS_TIMER: LittleWigs UldamanLegacyOfTyr/
//! Trash.lua:176–185 observes the dwarves opening the door after Bromach.
use bevy::prelude::*;

use super::instance_script::{self, EncounterState};
use crate::game_objects::{GO_STATE_ACTIVE, set_go_state};
use crate::maps::InMap;
use shared::protocol::GameObjectInfo;

const ULDAMAN: u32 = 2451;
const BROMACH_ENCOUNTER: u32 = 2556;
const LOST_DWARVES_BOSS: u32 = 0;
const PREHISTORIA_DOOR: u32 = 376557;
const OPEN_DELAY_SECS: f64 = 18.3;

#[derive(Component)]
struct DoorOpenAt(f64);

pub(super) fn on_boss_state_changed(
    world: &mut World,
    map: InMap,
    encounter: u32,
    state: EncounterState,
) {
    if map.map_id != ULDAMAN
        || encounter != BROMACH_ENCOUNTER
        || state != EncounterState::Done
        || instance_script::get_boss_state(world, map, LOST_DWARVES_BOSS)
            != Some(EncounterState::Done)
    {
        return;
    }
    let door = world
        .query::<(Entity, &InMap, &GameObjectInfo)>()
        .iter(world)
        .find(|(_, door_map, info)| **door_map == map && info.entry == PREHISTORIA_DOOR)
        .map(|(entity, _, _)| entity);
    let Some(door) = door else {
        error!(?map, "Uldaman Prehistoria door 376557 is missing");
        return;
    };
    let open_at = world.resource::<Time>().elapsed_secs_f64() + OPEN_DELAY_SECS;
    world.entity_mut(door).insert(DoorOpenAt(open_at));
}

pub(super) fn update_door(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let ready: Vec<Entity> = world
        .query::<(Entity, &DoorOpenAt)>()
        .iter(world)
        .filter_map(|(entity, deadline)| (now >= deadline.0).then_some(entity))
        .collect();
    for door in ready {
        set_go_state(world, door, GO_STATE_ACTIVE);
        world.entity_mut(door).remove::<DoorOpenAt>();
    }
}
