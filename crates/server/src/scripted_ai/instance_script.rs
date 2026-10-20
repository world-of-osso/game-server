//! TrinityCore `InstanceScript` (game/Instances/InstanceScript.cpp): the boss
//! encounter states of one dungeon copy, named by `instance_template.script`.
//!
//! A copy's script is created the first time one of its bosses needs it, with the
//! bosses its instance lock saved `DONE` and every other boss `NOT_STARTED`
//! (`InstanceMap::CreateInstanceData` → `InstanceScript::Load`), and lives until the
//! copy unloads (`unload_instance_scripts`). A boss becoming `DONE` updates the copy's
//! instance lock (`InstanceMap::UpdateInstanceLock`): the players inside are saved,
//! and the newly saved ones told `InstanceSaveCreated`.
//!
//! `SetBossState` (InstanceScript.cpp:374-478) sends `EncounterStart` on
//! `IN_PROGRESS` and `EncounterEnd` on `DONE` (success) to the copy's players, as
//! TrinityCore sends SMSG_INSTANCE_ENCOUNTER_START/END (no Stockade script sets
//! `FAIL`, which only `BossAI::_DespawnAtEvade` does). A reset from
//! `IN_PROGRESS` to `NOT_STARTED` (a wipe; TrinityCore sends nothing) also sends
//! `EncounterEnd` without success, as Retail's SMSG_ENCOUNTER_END does.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Player;
use shared::protocol::{
    EncounterChannel, EncounterDisengageUnit, EncounterEnd, EncounterEngageUnit, EncounterStart,
};

use crate::maps::InMap;
use crate::networking::PlayerLink;

/// The priority TrinityCore scripts engage their boss frame with.
const BOSS_FRAME_PRIORITY: u8 = 1;

/// `EncounterState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterState {
    NotStarted,
    InProgress,
    /// TC BossAI::_DespawnAtEvade; not a completed or saved encounter.
    Fail,
    Done,
}

/// One boss of an instance script (`BossInfo`).
#[derive(Debug, Clone, PartialEq)]
pub struct BossInfo {
    pub state: EncounterState,
    /// `DungeonEncounter.db2` ID at Normal difficulty (`LoadDungeonEncounterData`).
    pub encounter_id: u32,
}

/// An `InstanceScript`: its `instance_template.script` name, its bosses by boss id
/// (`SetBossNumber`) and the boss creatures its `LoadObjectData(creatureData)` names
/// (entry, boss id).
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceScript {
    pub name: &'static str,
    pub bosses: Vec<BossInfo>,
    pub boss_creatures: Vec<(u32, u32)>,
    /// `OnlyOnceAreaTriggerScript` success state; discarded when this copy unloads.
    pub(crate) handled_area_triggers: HashSet<u32>,
    /// Instance script GetData/SetData state; neither boss saves nor actor resets own it.
    pub(crate) data: HashMap<u32, u32>,
    /// Ramparts OnUnitDeath state, discarded with this copy; not a saved boss bit.
    pub(crate) ramparts_sentry_deaths: u8,
    /// Remaining ms of scheduled Ramparts reset events; copy-owned, not actor-owned.
    pub(crate) ramparts_reset_delays: Vec<u32>,
}

impl InstanceScript {
    /// `GetSaveData` boss states: the `DONE` bosses as a boss id mask.
    pub fn done_bosses(&self) -> u32 {
        self.bosses
            .iter()
            .enumerate()
            .filter(|(_, boss)| boss.state == EncounterState::Done)
            .fold(0, |mask, (boss_id, _)| mask | 1 << boss_id)
    }

    /// `InstanceScript::Load`: bosses saved `DONE` start `DONE`.
    fn load(mut self, done_bosses: u32) -> Self {
        for (boss_id, boss) in self.bosses.iter_mut().enumerate() {
            if done_bosses & (1 << boss_id) != 0 {
                boss.state = EncounterState::Done;
            }
        }
        self
    }
}

/// The boss creature entries of `map_id`'s instance script whose boss id is in
/// `done_bosses`: they do not spawn in a copy created with them `DONE` (TrinityCore
/// keeps them dead through their saved respawn time).
pub fn done_boss_entries(store: &crate::maps::MapStore, map_id: u32, done_bosses: u32) -> Vec<u32> {
    create_map_script(store, map_id, 0)
        .map(|script| {
            script
                .boss_creatures
                .iter()
                .filter(|&&(_, boss_id)| done_bosses & (1 << boss_id) != 0)
                .map(|&(entry, _)| entry)
                .collect()
        })
        .unwrap_or_default()
}

/// Authored compatibility data preserves the existing saved boss ordinals, including Azure.
/// It is part of the registry, not a retry/fallback when imported bindings are missing.
fn compatibility_catalog(map_id: u32) -> InstanceScript {
    let mut script = InstanceScript {
        name: "data_encounters",
        bosses: Vec::new(),
        boss_creatures: Vec::new(),
        handled_area_triggers: HashSet::new(),
        data: HashMap::new(),
        ramparts_sentry_deaths: 0,
        ramparts_reset_delays: Vec::new(),
    };
    for row in include_str!("encounter_compat.csv")
        .lines()
        .filter(|row| !row.starts_with('#'))
    {
        let values: Vec<u32> = row
            .split(',')
            .map(|v| v.parse().expect("invalid encounter compatibility catalog"))
            .collect();
        let [map, entry, encounter_id, boss_id] = values.as_slice() else {
            panic!("invalid encounter compatibility row");
        };
        if *map != map_id {
            continue;
        }
        assert_eq!(
            *boss_id as usize,
            script.bosses.len(),
            "compatibility boss ordinals must be contiguous"
        );
        script.bosses.push(BossInfo {
            state: EncounterState::NotStarted,
            encounter_id: *encounter_id,
        });
        script.boss_creatures.push((*entry, *boss_id));
    }
    script
}

/// Existing script ordinals remain stable; otherwise DB2 encounter bits are boss ids.
/// Sparse bits keep the same saved-mask identity used by the raid-entry catalog.
fn create_map_script(
    store: &crate::maps::MapStore,
    map_id: u32,
    difficulty: u32,
) -> Option<InstanceScript> {
    let mut script = compatibility_catalog(map_id);
    let has_legacy = !script.boss_creatures.is_empty();
    let mut bindings: Vec<_> = store
        .raid_creature_encounters
        .iter()
        .filter(|((map, _), _)| *map == map_id)
        .map(|((_, entry), bit)| (*entry, *bit))
        .collect();
    bindings.sort_unstable_by_key(|(entry, bit)| (*bit, *entry));
    for (entry, bit) in bindings {
        if script
            .boss_creatures
            .iter()
            .any(|(known, _)| *known == entry)
        {
            continue;
        }
        let Some(encounter) = encounter_for_bit(store, map_id, difficulty, bit) else {
            continue;
        };
        let boss_id = if has_legacy {
            script
                .bosses
                .iter()
                .position(|boss| boss.encounter_id == encounter.id)
                .unwrap_or(script.bosses.len()) as u32
        } else {
            bit
        };
        append_bound_boss(&mut script, entry, boss_id, encounter.id);
    }
    (!script.boss_creatures.is_empty()).then_some(script)
}

fn encounter_for_bit(
    store: &crate::maps::MapStore,
    map_id: u32,
    difficulty: u32,
    bit: u32,
) -> Option<&crate::maps::store::DungeonEncounter> {
    store
        .dungeon_encounters
        .values()
        .filter(|encounter| {
            let matches_difficulty = difficulty == 0
                || encounter.difficulty_id <= 0
                || encounter.difficulty_id as u32 == difficulty;
            encounter.map_id == map_id && encounter.bit == bit && matches_difficulty
        })
        .min_by_key(|encounter| encounter.id)
}

fn append_bound_boss(script: &mut InstanceScript, entry: u32, boss_id: u32, encounter_id: u32) {
    assert!(
        boss_id < 32,
        "Encounter boss id exceeds saved mask: {boss_id}"
    );
    while script.bosses.len() <= boss_id as usize {
        script.bosses.push(BossInfo {
            state: EncounterState::NotStarted,
            encounter_id: 0,
        });
    }
    script.bosses[boss_id as usize].encounter_id = encounter_id;
    script.boss_creatures.push((entry, boss_id));
}

/// A bound creature gets lifecycle even with no SmartAI rows or a C++ ScriptName.
pub(super) fn bound_boss(world: &World, creature: Entity) -> Option<super::boss_ai::BossAi> {
    let map = *world.get::<InMap>(creature)?;
    if map.instance_id == 0 {
        return None;
    }
    let entry = world.get::<shared::components::Npc>(creature)?.template_id;
    let difficulty = world
        .get_resource::<crate::maps::InstanceManager>()
        .and_then(|instances| instances.copy_difficulty(map))
        .map_or(0, |d| d.difficulty_id);
    let script = create_map_script(
        world.get_resource::<crate::maps::MapStore>()?,
        map.map_id,
        difficulty,
    )?;
    script
        .boss_creatures
        .iter()
        .find_map(|(boss, id)| (*boss == entry).then(|| super::boss_ai::BossAi::new(*id)))
}

#[derive(Resource, Debug, Default)]
pub struct InstanceScripts(pub HashMap<InMap, InstanceScript>);

/// The copy's instance script, created from `instance_template.script` when missing.
pub(super) fn instance_script(world: &mut World, map: InMap) -> Option<&mut InstanceScript> {
    if !world
        .get_resource_or_init::<InstanceScripts>()
        .0
        .contains_key(&map)
    {
        let instances = world.get_resource::<crate::maps::InstanceManager>();
        let done_bosses = instances.map_or(0, |instances| instances.initial_done_bosses(map));
        let difficulty = instances
            .and_then(|instances| instances.copy_difficulty(map))
            .map_or(0, |difficulty| difficulty.difficulty_id);
        let store = world.get_resource::<crate::maps::MapStore>()?;
        let script = create_map_script(store, map.map_id, difficulty)?.load(done_bosses);
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

/// `InstanceScript::GetData`: unassigned source keys read zero.
pub(super) fn get_data(world: &mut World, map: InMap, key: u32) -> u32 {
    instance_script(world, map)
        .and_then(|script| script.data.get(&key).copied())
        .unwrap_or(0)
}

/// `InstanceScript::SetData`: copy-lifetime custom state, deliberately not saved to DB.
pub(super) fn set_data(world: &mut World, map: InMap, key: u32, value: u32) {
    if let Some(script) = instance_script(world, map) {
        script.data.insert(key, value);
    }
}

/// `InstanceScript::OnCreatureGroupDepleted`.
pub(crate) fn creature_group_depleted(
    world: &mut World,
    map: InMap,
    leader_ids: &crate::zones::CreatureStringIds,
) {
    super::azure_vault::finish_leymor_intro(world, map, leader_ids);
}

/// Runtime slot of an encounter identity, independent of source ordinals/DB2 bits.
pub(super) fn boss_id_for_encounter(world: &mut World, map: InMap, encounter: u32) -> Option<u32> {
    instance_script(world, map)?
        .bosses
        .iter()
        .position(|boss| boss.encounter_id == encounter)
        .map(|slot| slot as u32)
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
    let difficulty_id = world
        .get_resource::<crate::maps::InstanceManager>()
        .and_then(|instances| instances.copy_difficulty(map))
        .map_or(0, |difficulty| difficulty.difficulty_id);
    match state {
        EncounterState::InProgress => send_to_players(
            world,
            map,
            EncounterStart {
                encounter_id,
                difficulty_id,
                group_size,
            },
        ),
        EncounterState::Done => {
            send_to_players(
                world,
                map,
                EncounterEnd {
                    encounter_id,
                    difficulty_id,
                    group_size,
                    success: true,
                },
            );
            // TC InstanceScript435–441 reads loot eligibility before saving the new lock.
            crate::achievement::record_encounter_loot_defeat(
                world,
                map,
                encounter_id,
                difficulty_id,
            );
            update_instance_lock(world, map, encounter_id, difficulty_id);
            crate::achievement::record_encounter_defeat(world, map, encounter_id, difficulty_id);
            crate::mythic_plus::record_boss(world, map, encounter_id);
            crate::raid_history::record_first_kill(world, map, encounter_id, difficulty_id);
        }
        EncounterState::NotStarted | EncounterState::Fail
            if previous == EncounterState::InProgress =>
        {
            send_to_players(
                world,
                map,
                EncounterEnd {
                    encounter_id,
                    difficulty_id,
                    group_size,
                    success: false,
                },
            )
        }
        EncounterState::NotStarted | EncounterState::Fail => {}
    }
    super::wailing_caverns::on_boss_state_changed(world, map, encounter_id, state);
    super::hellfire_ramparts::on_boss_state_changed(world, map, encounter_id, state);
    true
}

/// `InstanceScript::SetBossState(DONE)` → `InstanceMap::UpdateInstanceLock` with the
/// killed boss's `DungeonEncounter.Bit`.
fn update_instance_lock(world: &mut World, map: InMap, encounter_id: u32, difficulty_id: u32) {
    let Some(done_bosses) = world
        .get_resource::<InstanceScripts>()
        .and_then(|scripts| scripts.0.get(&map))
        .map(InstanceScript::done_bosses)
    else {
        return;
    };
    let bit = world
        .get_resource::<crate::maps::MapStore>()
        .and_then(|store| store.dungeon_encounter(encounter_id, map.map_id, difficulty_id))
        .map(|encounter| encounter.bit);
    let players: Vec<(Entity, u64)> = players_in_map(world, map)
        .into_iter()
        .filter_map(|player| {
            let character = world.get::<crate::persistence::ClientId>(player)?.0;
            Some((player, character))
        })
        .collect();
    let unix = crate::maps::difficulty::now(world).unix;
    let Some(mut instances) = world.get_resource_mut::<crate::maps::InstanceManager>() else {
        return;
    };
    let saved = instances.update_instance_lock(map, &players, done_bosses, bit, unix);
    crate::maps::difficulty::send_instance_saved(world, &saved);
}

/// `SendEncounterUnit(ENCOUNTER_FRAME_ENGAGE, unit, priority)`.
pub fn send_engage_unit(world: &mut World, map: InMap, unit: Entity) {
    send_engage_unit_with_priority(world, map, unit, BOSS_FRAME_PRIORITY);
}

pub fn send_engage_unit_with_priority(world: &mut World, map: InMap, unit: Entity, priority: u8) {
    send_to_players(
        world,
        map,
        EncounterEngageUnit {
            unit: unit.to_bits(),
            target_frame_priority: priority,
        },
    );
    super::bossmod::engage(world, unit);
}

/// `SendEncounterUnit(ENCOUNTER_FRAME_DISENGAGE, unit)`.
pub fn send_disengage_unit(world: &mut World, map: InMap, unit: Entity) {
    super::bossmod::disengage(world, unit);
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
pub(super) fn send_to_players<M: lightyear::prelude::Message + Clone>(
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

/// A live instance boss, not an ordinary creature with a similar spell.
pub(super) fn active_encounter_id(world: &World, map: InMap, unit: Entity) -> Option<u32> {
    let npc = world.get::<shared::components::Npc>(unit)?;
    let script = world.get_resource::<InstanceScripts>()?.0.get(&map)?;
    let (_, boss_id) = script
        .boss_creatures
        .iter()
        .find(|(entry, _)| *entry == npc.template_id)?;
    let boss = script.bosses.get(*boss_id as usize)?;
    (boss.state == EncounterState::InProgress).then_some(boss.encounter_id)
}

#[cfg(test)]
mod mythic_tests {
    use super::*;

    #[test]
    fn mythic_azure_encounter_lifecycle_preserves_saved_ordinals_and_copy_isolation() {
        let mut world = World::new();
        world.insert_resource(crate::maps::MapStore::default());
        world.insert_resource(Time::<()>::default());
        world.insert_resource(crate::maps::ServerClock {
            offset_secs: 0,
            fixed_unix: Some(1_791_705_600),
        });
        let map = InMap {
            map_id: 2515,
            instance_id: 1,
        };
        let other = InMap {
            instance_id: 2,
            ..map
        };
        let encounters = [
            (186644, 2582),
            (186739, 2585),
            (186737, 2583),
            (186738, 2584),
        ];
        for (boss, (entry, encounter)) in encounters.into_iter().enumerate() {
            let boss = boss as u32;
            assert_eq!(
                get_boss_state(&mut world, map, boss),
                Some(EncounterState::NotStarted)
            );
            assert!(set_boss_state(
                &mut world,
                map,
                boss,
                EncounterState::InProgress
            ));
            assert!(set_boss_state(
                &mut world,
                map,
                boss,
                EncounterState::NotStarted
            ));
            assert!(set_boss_state(&mut world, map, boss, EncounterState::Done));
            assert!(!set_boss_state(
                &mut world,
                map,
                boss,
                EncounterState::InProgress
            ));
            assert_eq!(
                get_boss_state(&mut world, other, boss),
                Some(EncounterState::NotStarted)
            );
            let scripts = world.resource::<InstanceScripts>();
            assert_eq!(
                scripts.0[&map].bosses[boss as usize].encounter_id,
                encounter
            );
            assert_eq!(
                done_boss_entries(world.resource::<crate::maps::MapStore>(), 2515, 1 << boss),
                vec![entry]
            );
        }
        assert_eq!(
            world.resource::<InstanceScripts>().0[&map].done_bosses(),
            15
        );
    }
}

/// Unloaded copies take their instance scripts with them.
pub(crate) fn unload_instance_scripts(world: &mut World, maps: &[InMap]) {
    if let Some(mut scripts) = world.get_resource_mut::<InstanceScripts>() {
        scripts.0.retain(|map, _| !maps.contains(map));
    }
}
