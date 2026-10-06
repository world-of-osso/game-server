use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::{Level, Npc};
use shared::protocol::{
    AchievementChannel, AchievementSnapshot, AchievementStateUpdate, AchievementToastSnapshot,
};

use crate::auth::{Authenticated, InWorld};
use crate::maps::InMap;
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

#[path = "achievement_catalog.rs"]
mod catalog;
pub(crate) use catalog::{AchievementCatalog, CriteriaEvent, load_achievement_catalog};

#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlayerAchievementState {
    pub earned_ids: BTreeSet<u32>,
    #[serde(default)]
    pub criteria_progress: BTreeMap<u32, u64>,
}

/// One-time migration of the six former level placeholders. Counter-bearing rows
/// already use Retail IDs and are never remapped.
pub(crate) fn decode_saved_achievement_state(bytes: &[u8]) -> (PlayerAchievementState, bool) {
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("decode achievement JSON");
    let legacy = value.get("criteria_progress").is_none();
    let mut state: PlayerAchievementState =
        serde_json::from_value(value).expect("decode achievements");
    if legacy {
        const RETAIL_IDS: [u32; 6] = [6, 7, 9, 14783, 15805, 19459];
        state.earned_ids = state
            .earned_ids
            .into_iter()
            .map(|id| {
                if (1..=6).contains(&id) {
                    RETAIL_IDS[(id - 1) as usize]
                } else {
                    id
                }
            })
            .collect();
    }
    (state, legacy)
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct AchievementTracker {
    last_processed_level: u8,
}

pub struct AchievementPlugin;

impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                send_initial_achievement_snapshot,
                process_level_achievement_updates,
            )
                .chain(),
        );
    }
}

fn send_initial_achievement_snapshot(world: &mut World) {
    let connections: Vec<_> = world
        .query_filtered::<(Entity, &PlayerLink), (With<Authenticated>, Added<InWorld>)>()
        .iter(world)
        .map(|(connection, link)| (connection, link.0))
        .collect();
    for (connection, player) in connections {
        let Some(level) = world.get::<Level>(player).map(|level| level.0) else {
            continue;
        };
        let Some((snapshot, _)) =
            update_player_events(world, player, &[CriteriaEvent::ReachLevel(level)], None)
        else {
            continue;
        };
        send_updates(world, connection, snapshot, Vec::new());
        world.entity_mut(player).insert(AchievementTracker {
            last_processed_level: level,
        });
    }
}

fn process_level_achievement_updates(world: &mut World) {
    let players: Vec<_> = world
        .query_filtered::<(Entity, &Level, &AchievementTracker), Changed<Level>>()
        .iter(world)
        .filter(|(_, level, tracker)| level.0 != tracker.last_processed_level)
        .map(|(player, level, _)| (player, level.0))
        .collect();
    for (player, level) in players {
        let Some((snapshot, toasts)) =
            update_player_events(world, player, &[CriteriaEvent::ReachLevel(level)], None)
        else {
            continue;
        };
        world.entity_mut(player).insert(AchievementTracker {
            last_processed_level: level,
        });
        if let Some(connection) = connection_for_player(world, player) {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

fn connection_for_player(world: &mut World, player: Entity) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &PlayerLink), With<Authenticated>>()
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

fn update_player_events(
    world: &mut World,
    player: Entity,
    events: &[CriteriaEvent],
    completed_map: Option<u32>,
) -> Option<(AchievementSnapshot, Vec<AchievementToastSnapshot>)> {
    if !world.contains_resource::<AchievementCatalog>() {
        return None;
    }
    let result = world.resource_scope(|world, catalog: Mut<AchievementCatalog>| {
        let mut state = world.get_mut::<PlayerAchievementState>(player)?;
        let toasts = events
            .iter()
            .flat_map(|event| catalog.update(&mut state, *event, completed_map))
            .collect();
        Some((catalog.snapshot(&state), toasts))
    });
    if result.is_some() {
        persist_player_achievements(world, player);
    }
    result
}

fn persist_player_achievements(world: &mut World, player: Entity) {
    let character = world
        .get::<persistence::ClientId>(player)
        .map(|id| id.0)
        .or_else(|| {
            world
                .query::<(&PlayerLink, &InWorld)>()
                .iter(world)
                .find_map(|(link, in_world)| (link.0 == player).then_some(in_world.character_id))
        });
    if let Some(character) = character
        && let Some(db) = world.get_resource::<GameDatabase>()
        && let Some(state) = world.get::<PlayerAchievementState>(player)
    {
        persistence::save_achievements(&db.db, character, state);
    }
}

fn send_updates(
    world: &mut World,
    connection: Entity,
    snapshot: AchievementSnapshot,
    toasts: Vec<AchievementToastSnapshot>,
) {
    let Some(mut sender) = world.get_mut::<MessageSender<AchievementStateUpdate>>(connection)
    else {
        return;
    };
    if toasts.is_empty() {
        sender.send::<AchievementChannel>(AchievementStateUpdate {
            snapshot: Some(snapshot),
            completed: None,
            message: Some("achievement progress updated".into()),
            error: None,
        });
        return;
    }
    for toast in toasts {
        sender.send::<AchievementChannel>(AchievementStateUpdate {
            snapshot: Some(snapshot.clone()),
            completed: Some(toast),
            message: Some("achievement completed".into()),
            error: None,
        });
    }
}

/// Successful scripted encounter boundary. A scripted defeat (Hogger's surrender
/// included) credits its creature criteria even when no corpse is produced.
pub(crate) fn record_encounter_defeat(
    world: &mut World,
    map: InMap,
    encounter_id: u32,
    difficulty: u32,
) {
    let Some(store) = world.get_resource::<crate::maps::MapStore>() else {
        return;
    };
    if store
        .dungeon_encounter(encounter_id, map.map_id, difficulty)
        .is_none()
    {
        return;
    }
    let encounters = store.encounters_for_map(map.map_id, difficulty);
    let mask = world
        .get_resource::<crate::maps::InstanceManager>()
        .map_or(0, |instances| instances.completed_encounters(map));
    let complete = !encounters.is_empty()
        && encounters
            .iter()
            .all(|encounter| mask & (1 << encounter.bit) != 0);
    let creature = world
        .get_resource::<crate::scripted_ai::instance_script::InstanceScripts>()
        .and_then(|scripts| scripts.0.get(&map))
        .and_then(|script| {
            script.boss_creatures.iter().find_map(|&(entry, boss_id)| {
                (script.bosses[boss_id as usize].encounter_id == encounter_id).then_some(entry)
            })
        });
    let mut events = vec![CriteriaEvent::DefeatEncounter(encounter_id)];
    if let Some(creature) = creature {
        events.push(CriteriaEvent::KillCreature(creature));
    }
    for player in crate::scripted_ai::instance_script::players_in_map(world, map) {
        let Some((snapshot, toasts)) =
            update_player_events(world, player, &events, complete.then_some(map.map_id))
        else {
            continue;
        };
        if let Some(connection) = connection_for_player(world, player) {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

/// Ordinary NPC kills use the actual player killer. Scripted boss deaths are
/// already credited by SetBossState(DONE), so are not counted twice.
pub(crate) fn record_creature_death(world: &mut World, creature: Entity, killer: Option<Entity>) {
    let Some(killer) = killer else { return };
    let Some(npc) = world.get::<Npc>(creature) else {
        return;
    };
    let entry = npc.template_id;
    let map = world.get::<InMap>(creature).copied();
    let scripted_boss = map
        .and_then(|map| {
            world
                .get_resource::<crate::scripted_ai::instance_script::InstanceScripts>()?
                .0
                .get(&map)
        })
        .is_some_and(|script| script.boss_creatures.iter().any(|&(boss, _)| boss == entry));
    if scripted_boss {
        return;
    }
    let Some((snapshot, toasts)) =
        update_player_events(world, killer, &[CriteriaEvent::KillCreature(entry)], None)
    else {
        return;
    };
    if let Some(connection) = connection_for_player(world, killer) {
        send_updates(world, connection, snapshot, toasts);
    }
}

#[cfg(test)]
#[path = "achievement_tests.rs"]
mod tests;
