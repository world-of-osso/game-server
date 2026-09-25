//! Quest runtime: quest givers, the quest log and objective progress (Retail
//! `QuestHandler` flow, AzerothCore rules in `quest_rules`).
//!
//! Requests (`quest_requests`) load the character's `QuestLogData` and
//! `InventoryData` from redb, apply, save, and mark the character in
//! `QuestDirty`. Kill credit comes from NPC deaths (`Added<DeathState>` +
//! `LastDamager`, as reputation does) for the killer and group members within
//! reward distance. Item objectives read bag counts, so any inventory change
//! recorded in `GoldUpdateQueue` marks the owner dirty; level changes do too.
//! In `PostUpdate` each dirty character's log is diffed against the last view
//! sent (`QuestLogUpdate`) and the markers of every NPC its connection queried
//! are re-sent (`QuestGiverStatusMultiple`).
use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{Level, Npc, Player, Position};
use shared::protocol::{
    QuestChannel, QuestGiverStatusEntry, QuestGiverStatusMultiple, QuestLogSnapshot, QuestLogUpdate,
};

use crate::auth::{Authenticated, InWorld};
use crate::combat::{DeathState, LastDamager};
use crate::gold::GoldUpdateQueue;
use crate::group::GroupRegistry;
use crate::networking::PlayerLink;
use crate::persistence::quest_store::{QuestLogData, load_quest_log, save_quest_log};
use crate::persistence::{self, ClientId, GameDatabase, InventoryData};
use crate::quest_data::QuestDb;
use crate::quest_rules::{QuestPlayer, credit_kill, giver_status};
use crate::quest_view::{QuestLogView, bag_count, diff_log, log_entries, log_snapshot, view_of};

#[path = "quest_requests.rs"]
mod quest_requests;

/// Group members this close to the killed creature share kill credit
/// (TrinityCore `CONFIG_GROUP_XP_DISTANCE` default).
const GROUP_REWARD_DISTANCE: f32 = 74.0;

/// Characters whose quest log or quest markers must be re-sent.
#[derive(Resource, Default)]
pub(crate) struct QuestDirty(pub HashSet<u64>);

/// Quest log last sent to this connection; updates diff against it.
#[derive(Component, Debug, Default)]
pub(crate) struct SentQuestLog {
    entries: QuestLogView,
    watched: Vec<u32>,
}

/// NPCs this connection asked markers for; re-sent on every quest change.
#[derive(Component, Debug, Default)]
pub(crate) struct QueriedQuestGivers(pub HashSet<Entity>);

/// Persistence plus quest content: everything quest systems read.
#[derive(SystemParam)]
pub(crate) struct QuestStore<'w> {
    db: Option<Res<'w, GameDatabase>>,
    quests: Option<Res<'w, QuestDb>>,
}

impl QuestStore<'_> {
    pub(crate) fn get(&self) -> Option<(&Database, &QuestDb)> {
        Some((&self.db.as_ref()?.db, self.quests.as_deref()?))
    }
}

pub struct QuestPlugin;

impl Plugin for QuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestDirty>();
        app.init_resource::<crate::experience::XpAwards>();
        app.add_systems(
            Update,
            (
                send_initial_quest_log,
                quest_requests::process_quest_requests,
            )
                .chain(),
        );
        app.add_systems(
            PostUpdate,
            (
                credit_quest_kills,
                mark_quest_inputs_changed,
                send_quest_updates,
            )
                .chain()
                .before(crate::inventory::InventoryDeltas)
                .before(ReplicationSystems::Send),
        );
    }
}

pub(crate) fn quest_player(player: &Player, level: &Level) -> QuestPlayer {
    QuestPlayer {
        level: level.0,
        race: player.race,
        class: player.class,
    }
}

/// Marker of each NPC for this character.
pub(crate) fn giver_statuses(
    quests: &QuestDb,
    log: &QuestLogData,
    inventory: &InventoryData,
    player: QuestPlayer,
    npcs: impl IntoIterator<Item = (Entity, u32)>,
) -> Vec<QuestGiverStatusEntry> {
    npcs.into_iter()
        .map(|(npc, template_id)| QuestGiverStatusEntry {
            npc: npc.to_bits(),
            status: giver_status(quests, log, player, template_id, |item_id| {
                bag_count(inventory, item_id)
            }),
        })
        .collect()
}

fn send_initial_quest_log(
    mut commands: Commands,
    connections: Query<(Entity, &InWorld), (With<Authenticated>, Added<InWorld>)>,
    store: QuestStore,
    mut senders: Query<&mut MessageSender<QuestLogSnapshot>>,
) {
    let Some((db, quests)) = store.get() else {
        return;
    };
    for (connection, in_world) in &connections {
        let log = load_quest_log(db, in_world.character_id);
        let inventory = persistence::load_inventory(db, in_world.character_id);
        let snapshot = log_snapshot(quests, &log, &inventory);
        commands.entity(connection).insert((
            SentQuestLog {
                entries: view_of(&snapshot.entries),
                watched: snapshot.watched_quest_ids.clone(),
            },
            QueriedQuestGivers::default(),
        ));
        if let Ok(mut sender) = senders.get_mut(connection) {
            sender.send::<QuestChannel>(snapshot);
        }
    }
}

pub(crate) type PlayerPositions<'w, 's> =
    Query<'w, 's, (&'static ClientId, &'static Position), With<Player>>;

fn distance(a: &Position, b: &Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// The killer plus its group members within reward distance of the creature.
pub(crate) fn credited_characters(
    killer: u64,
    creature: &Position,
    players: &PlayerPositions,
    groups: Option<&GroupRegistry>,
) -> Vec<u64> {
    let members = groups.map_or_else(Vec::new, |groups| groups.group_members(killer));
    if members.is_empty() {
        return vec![killer];
    }
    members
        .into_iter()
        .filter(|&member| {
            member == killer
                || players.iter().any(|(id, position)| {
                    id.0 == member && distance(position, creature) <= GROUP_REWARD_DISTANCE
                })
        })
        .collect()
}

fn credit_quest_kills(
    killed: Query<(&Npc, &LastDamager, &Position), Added<DeathState>>,
    players: PlayerPositions,
    groups: Option<Res<GroupRegistry>>,
    store: QuestStore,
    mut dirty: ResMut<QuestDirty>,
) {
    let Some((db, quests)) = store.get() else {
        return;
    };
    for (npc, killer, position) in &killed {
        let Ok((killer_id, _)) = players.get(killer.0) else {
            continue;
        };
        for character_id in credited_characters(killer_id.0, position, &players, groups.as_deref())
        {
            let mut log = load_quest_log(db, character_id);
            if credit_kill(quests, &mut log, npc.template_id) {
                save_quest_log(db, character_id, &log);
                dirty.0.insert(character_id);
            }
        }
    }
}

/// Bag changes move item objectives; level changes move quest markers.
fn mark_quest_inputs_changed(
    inventories: Res<GoldUpdateQueue>,
    levels: Query<&ClientId, Changed<Level>>,
    mut dirty: ResMut<QuestDirty>,
) {
    dirty
        .0
        .extend(inventories.changed_inventories().iter().copied());
    dirty.0.extend(levels.iter().map(|id| id.0));
}

type UpdateConnections<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static InWorld,
        &'static PlayerLink,
        &'static mut SentQuestLog,
        &'static mut QueriedQuestGivers,
    ),
>;

fn send_quest_updates(
    mut dirty: ResMut<QuestDirty>,
    store: QuestStore,
    mut connections: UpdateConnections,
    players: Query<(&Player, &Level)>,
    npcs: Query<&Npc>,
    mut log_senders: Query<&mut MessageSender<QuestLogUpdate>>,
    mut status_senders: Query<&mut MessageSender<QuestGiverStatusMultiple>>,
) {
    let Some((db, quests)) = store.get() else {
        return;
    };
    if dirty.0.is_empty() {
        return;
    }
    let changed = std::mem::take(&mut dirty.0);
    for (connection, in_world, link, mut sent, mut queried) in &mut connections {
        if !changed.contains(&in_world.character_id) {
            continue;
        }
        let log = load_quest_log(db, in_world.character_id);
        let inventory = persistence::load_inventory(db, in_world.character_id);
        let view = view_of(&log_entries(quests, &log, &inventory));
        if let Some(update) = diff_log(&sent.entries, &sent.watched, &view, &log.watched)
            && let Ok(mut sender) = log_senders.get_mut(connection)
        {
            sender.send::<QuestChannel>(update);
            sent.entries = view;
            sent.watched = log.watched.clone();
        }
        queried.0.retain(|&npc| npcs.contains(npc));
        let Ok((player, level)) = players.get(link.0) else {
            continue;
        };
        if queried.0.is_empty() {
            continue;
        }
        let givers = queried
            .0
            .iter()
            .filter_map(|&npc| Some((npc, npcs.get(npc).ok()?.template_id)));
        let statuses = giver_statuses(
            quests,
            &log,
            &inventory,
            quest_player(player, level),
            givers,
        );
        if let Ok(mut sender) = status_senders.get_mut(connection) {
            sender.send::<QuestChannel>(QuestGiverStatusMultiple { statuses });
        }
    }
}

#[cfg(test)]
#[path = "quest_tests.rs"]
mod tests;
