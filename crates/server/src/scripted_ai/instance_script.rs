//! TrinityCore `InstanceScript` (game/Instances/InstanceScript.cpp): the boss
//! encounter states of one dungeon copy, named by `instance_template.script`.
//!
//! A copy's script is created with every boss `NOT_STARTED` the first time one of
//! its bosses needs it and lives until the copy unloads (`unload_instance_scripts`);
//! a recreated copy starts fresh (instance locks are phase 4).
//!
//! `SetBossState` (InstanceScript.cpp:374-478) sends `EncounterStart` on
//! `IN_PROGRESS` and `EncounterEnd` on `DONE` (success) to the copy's players, as
//! TrinityCore sends SMSG_INSTANCE_ENCOUNTER_START/END (no Stockade script sets
//! `FAIL`, which only `BossAI::_DespawnAtEvade` does). A reset from
//! `IN_PROGRESS` to `NOT_STARTED` (a wipe; TrinityCore sends nothing) also sends
//! `EncounterEnd` without success, as Retail's SMSG_ENCOUNTER_END does.

use std::collections::HashMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Player;
use shared::protocol::{
    EncounterChannel, EncounterDisengageUnit, EncounterEnd, EncounterEngageUnit, EncounterStart,
};

use crate::maps::InMap;
use crate::networking::PlayerLink;

/// `DIFFICULTY_NORMAL`: every hosted dungeon copy (docs/specs/instances.md).
const DIFFICULTY_NORMAL: u32 = 1;
/// The priority TrinityCore scripts engage their boss frame with.
const BOSS_FRAME_PRIORITY: u8 = 1;

/// `EncounterState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterState {
    NotStarted,
    InProgress,
    Done,
}

/// One boss of an instance script (`BossInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct BossInfo {
    pub state: EncounterState,
    /// `DungeonEncounter.db2` ID at Normal difficulty (`LoadDungeonEncounterData`).
    pub encounter_id: u32,
}

/// An `InstanceScript`: its `instance_template.script` name and its bosses by
/// boss id (`SetBossNumber`).
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceScript {
    pub name: &'static str,
    pub bosses: Vec<BossInfo>,
}

#[derive(Resource, Debug, Default)]
pub struct InstanceScripts(pub HashMap<InMap, InstanceScript>);

/// The copy's instance script, created from `instance_template.script` when missing.
fn instance_script(world: &mut World, map: InMap) -> Option<&mut InstanceScript> {
    if !world
        .get_resource_or_init::<InstanceScripts>()
        .0
        .contains_key(&map)
    {
        let name = world
            .get_resource::<crate::maps::MapStore>()
            .and_then(|store| store.instance_scripts.get(&map.map_id))?;
        let script = super::the_stockade::instance_script(name)?;
        world
            .resource_mut::<InstanceScripts>()
            .0
            .insert(map, script);
    }
    world
        .get_resource_mut::<InstanceScripts>()
        .map(Mut::into_inner)
        .and_then(|scripts| scripts.0.get_mut(&map))
}

/// `InstanceScript::GetBossState`; `None` outside an instance script.
pub fn get_boss_state(world: &mut World, map: InMap, boss_id: u32) -> Option<EncounterState> {
    instance_script(world, map)?
        .bosses
        .get(boss_id as usize)
        .map(|boss| boss.state)
}

/// `InstanceScript::SetBossState`: false when unchanged or blocked (a `DONE` boss
/// stays done).
pub fn set_boss_state(world: &mut World, map: InMap, boss_id: u32, state: EncounterState) -> bool {
    let Some(boss) =
        instance_script(world, map).and_then(|script| script.bosses.get_mut(boss_id as usize))
    else {
        return false;
    };
    let previous = boss.state;
    if previous == state {
        return false;
    }
    if previous == EncounterState::Done {
        error!(
            "InstanceScript: Tried to set instance boss {boss_id} state from DONE back to {state:?} for map {}, instance id {}. Blocked!",
            map.map_id, map.instance_id
        );
        return false;
    }
    boss.state = state;
    let encounter_id = boss.encounter_id;
    info!("Instance {map:?}: boss {boss_id} (encounter {encounter_id}) {previous:?} -> {state:?}");
    let group_size = players_in_map(world, map).len() as u32;
    match state {
        EncounterState::InProgress => send_to_players(
            world,
            map,
            EncounterStart {
                encounter_id,
                difficulty_id: DIFFICULTY_NORMAL,
                group_size,
            },
        ),
        EncounterState::Done => send_to_players(
            world,
            map,
            EncounterEnd {
                encounter_id,
                difficulty_id: DIFFICULTY_NORMAL,
                group_size,
                success: true,
            },
        ),
        EncounterState::NotStarted if previous == EncounterState::InProgress => send_to_players(
            world,
            map,
            EncounterEnd {
                encounter_id,
                difficulty_id: DIFFICULTY_NORMAL,
                group_size,
                success: false,
            },
        ),
        EncounterState::NotStarted => {}
    }
    true
}

/// `SendEncounterUnit(ENCOUNTER_FRAME_ENGAGE, unit, priority)`.
pub fn send_engage_unit(world: &mut World, map: InMap, unit: Entity) {
    send_to_players(
        world,
        map,
        EncounterEngageUnit {
            unit: unit.to_bits(),
            target_frame_priority: BOSS_FRAME_PRIORITY,
        },
    );
}

/// `SendEncounterUnit(ENCOUNTER_FRAME_DISENGAGE, unit)`.
pub fn send_disengage_unit(world: &mut World, map: InMap, unit: Entity) {
    send_to_players(
        world,
        map,
        EncounterDisengageUnit {
            unit: unit.to_bits(),
        },
    );
}

/// The players in map copy `map` (`Map::DoOnPlayers`).
pub fn players_in_map(world: &mut World, map: InMap) -> Vec<Entity> {
    world
        .query_filtered::<(Entity, &InMap), With<Player>>()
        .iter(world)
        .filter(|(_, player_map)| **player_map == map)
        .map(|(player, _)| player)
        .collect()
}

/// `Map::SendToPlayers` on the encounter channel.
fn send_to_players<M: lightyear::prelude::Message + Clone>(
    world: &mut World,
    map: InMap,
    message: M,
) {
    let players = players_in_map(world, map);
    let mut connections = world.query::<(&PlayerLink, &mut MessageSender<M>)>();
    for (link, mut sender) in connections.iter_mut(world) {
        if players.contains(&link.0) {
            sender.send::<EncounterChannel>(message.clone());
        }
    }
}

/// Unloaded copies take their instance scripts with them.
pub(crate) fn unload_instance_scripts(world: &mut World, maps: &[InMap]) {
    if let Some(mut scripts) = world.get_resource_mut::<InstanceScripts>() {
        scripts.0.retain(|map, _| !maps.contains(map));
    }
}
