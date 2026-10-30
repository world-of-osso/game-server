//! ULDAMAN_PREHISTORIA_DOOR_LITTLEWIGS_TIMER: LittleWigs UldamanLegacyOfTyr/
//! Trash.lua:176–185 observes the dwarves opening the door after Bromach.
use bevy::prelude::*;

use super::instance_script::{self, EncounterState};
use crate::game_objects::{GO_STATE_ACTIVE, set_go_state};
use crate::maps::InMap;
use shared::components::{Health, Npc, Player, Position, UnitFlags};
use shared::protocol::GameObjectInfo;

const ULDAMAN: u32 = 2451;
const BROMACH_ENCOUNTER: u32 = 2556;
const LOST_DWARVES_BOSS: u32 = 0;
const PREHISTORIA_DOOR: u32 = 376557;
const OPEN_DELAY_SECS: f64 = 18.3;
const EMBERON_ENCOUNTER: u32 = 2558;
const DEIOS: u32 = 184125;
const DEIOS_RP: u32 = 191220;
const TEMPORAL_THEFT: u32 = 382264;
const WARMUP_SECS: f64 = 16.9;
const WARMUP_STATE: u32 = 2451001;
const CAST_REQUESTED: u32 = 1;
const ACTIVATED: u32 = 2;
const IMMUNE_TO_PC: u32 = 0x100;
// DEIOS_WARMUP_PROXIMITY_INFERRED: use the boss template's 20-yard detection
// range. No source identifies the Normal/Follower RP trigger.
const WARMUP_RADIUS: f32 = 20.0;

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

/// DEIOS_WARMUP_LITTLEWIGS_TIMER: Temporal Theft succeeds before the 16.9s bar.
pub(super) fn update_deios_warmup(world: &mut World) {
    let bosses: Vec<_> = world
        .query::<(Entity, &Npc, &InMap, &Position)>()
        .iter(world)
        .filter(|(_, npc, map, _)| npc.template_id == DEIOS && map.map_id == ULDAMAN)
        .map(|(entity, _, map, position)| (entity, *map, *position))
        .collect();
    for (boss, map, position) in bosses {
        update_deios_copy(world, boss, map, position);
    }
}

fn update_deios_copy(world: &mut World, boss: Entity, map: InMap, position: Position) {
    let Some(emberon) = instance_script::boss_id_for_encounter(world, map, EMBERON_ENCOUNTER)
    else {
        return;
    };
    if instance_script::get_boss_state(world, map, emberon) != Some(EncounterState::Done) {
        return;
    }
    let state = instance_script::get_data(world, map, WARMUP_STATE);
    if state == ACTIVATED {
        return;
    }
    let rp = world
        .query::<(Entity, &Npc, &InMap)>()
        .iter(world)
        .find(|(_, npc, copy)| npc.template_id == DEIOS_RP && **copy == map)
        .map(|(entity, _, _)| entity);
    let Some(rp) = rp else {
        return;
    };
    if state == 0 {
        let nearby = world
            .query::<(&Player, &InMap, &Position, &Health)>()
            .iter(world)
            .any(|(_, copy, player, health)| {
                *copy == map
                    && health.current > 0.0
                    && (player.x - position.x).powi(2)
                        + (player.y - position.y).powi(2)
                        + (player.z - position.z).powi(2)
                        <= WARMUP_RADIUS.powi(2)
            });
        if nearby {
            match crate::spell_cast::creature_cast(world, rp, TEMPORAL_THEFT, None, false) {
                Ok(()) => instance_script::set_data(world, map, WARMUP_STATE, CAST_REQUESTED),
                Err(error) => error!(?map, ?error, "Deios RP Temporal Theft cast rejected"),
            }
        }
        return;
    }
    let completed = world
        .get::<crate::spell_cast::SpellHistory>(rp)
        .and_then(|history| history.last_successful_cast_at(TEMPORAL_THEFT));
    if completed.is_some_and(|at| world.resource::<Time>().elapsed_secs_f64() >= at + WARMUP_SECS) {
        let mut actor = world.entity_mut(boss);
        if let Some(mut flags) = actor.get_mut::<UnitFlags>() {
            flags.0 &= !IMMUNE_TO_PC;
        }
        instance_script::set_data(world, map, WARMUP_STATE, ACTIVATED);
    }
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
