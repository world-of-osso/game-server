//! Quest runtime: quest givers, the quest log and objective progress (Retail
//! `QuestHandler` flow, AzerothCore rules in `quest_rules`).
//!
//! Requests (`quest_requests`) load the character's `QuestLogData` and
//! `InventoryData` from redb, apply, save, and mark the character in
//! `QuestDirty`. Kill credit comes from NPC deaths (`Added<DeathState>`) for the
//! creature's rewarded players (`creature_tap`: its tappers and their groups within
//! reward distance). Item objectives read bag counts, so any inventory change
//! recorded in `GoldUpdateQueue` marks the owner dirty; level changes do too.
//! In `PostUpdate` each dirty character's log is diffed against the last view
//! sent (`QuestLogUpdate`) and the markers of every NPC its connection queried
//! are re-sent (`QuestGiverStatusMultiple`).
use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use redb::Database;
use shared::components::{Level, MovementControl, Npc, Player, Position};
use shared::death::DeathState as PlayerLifeState;
use shared::protocol::{
    QuestChannel, QuestGiverStatusEntry, QuestGiverStatusMultiple, QuestLogSnapshot, QuestLogUpdate,
};

use crate::auth::{Authenticated, InWorld};
use crate::combat::DeathState;
use crate::creature_tap::{CreatureTap, RewardPlayers, distance, rewarded_characters};
use crate::death::PlayerDeathState;
use crate::gold::GoldUpdateQueue;
use crate::group::GroupRegistry;
use crate::networking::PlayerLink;
use crate::persistence::quest_store::{QuestLogData, load_quest_log, save_quest_log};
use crate::persistence::{self, ClientId, GameDatabase, InventoryData};
use crate::quest_data::QuestDb;
use crate::quest_rules::{
    QuestPlayer, credit_area_trigger, credit_kill, credit_killed_creature, giver_status,
};
use crate::quest_view::{QuestLogView, bag_count, diff_log, log_entries, log_snapshot, view_of};

#[path = "quest_requests.rs"]
mod quest_requests;

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
        app.init_resource::<crate::maps::difficulty::ServerClock>();
        app.init_resource::<quest_requests::PendingShares>();
        app.init_resource::<crate::experience::XpAwards>();
        app.add_systems(
            Update,
            (
                crate::quest_time::refresh,
                send_initial_quest_log,
                quest_requests::process_quest_requests,
            )
                .chain(),
        );
        app.add_systems(
            PostUpdate,
            (
                credit_quest_kills,
                credit_quest_area_triggers,
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
    db: &redb::Database,
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
                bag_count(db, inventory, item_id)
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
        let snapshot = log_snapshot(db, quests, &log, &inventory);
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

/// `KillRewarder::_RewardKillCredit` (KillRewarder.cpp:177-188) for every rewarded
/// player of a dead creature (`creature_tap`): a grouped player needs to be alive or
/// without a corpse.
fn credit_quest_kills(
    killed: Query<(&Npc, &CreatureTap, &Position, &crate::maps::InMap), Added<DeathState>>,
    players: RewardPlayers,
    lives: Query<(&ClientId, Option<&PlayerDeathState>), With<Player>>,
    groups: Option<Res<GroupRegistry>>,
    store: QuestStore,
    mut dirty: ResMut<QuestDirty>,
) {
    let Some((db, quests)) = store.get() else {
        return;
    };
    let gets_credit = |character: u64| {
        let grouped = groups
            .as_deref()
            .is_some_and(|groups| !groups.group_members(character).is_empty());
        !grouped
            || lives.iter().any(|(id, death)| {
                id.0 == character && death.is_none_or(|death| death.state == PlayerLifeState::Alive)
            })
    };
    for (npc, tap, position, map) in &killed {
        for character_id in
            rewarded_characters(&tap.tappers, (map, position), &players, groups.as_deref())
                .into_iter()
                .filter(|&character| gets_credit(character))
        {
            let mut log = load_quest_log(db, character_id);
            if credit_killed_creature(quests, &mut log, npc.template_id) {
                save_quest_log(db, character_id, &log);
                dirty.0.insert(character_id);
            }
        }
    }
}

/// `Player::KilledMonsterCredit(entry)` outside a kill (scripts): one kill of `entry` for
/// `player`'s quests that count it.
pub(crate) fn killed_monster_credit(world: &mut World, player: Entity, entry: u32) {
    let Some(character_id) = world.get::<ClientId>(player).map(|id| id.0) else {
        return;
    };
    let (Some(db), Some(quests)) = (
        world.get_resource::<GameDatabase>(),
        world.get_resource::<QuestDb>(),
    ) else {
        return;
    };
    let mut log = load_quest_log(&db.db, character_id);
    if !credit_kill(quests, &mut log, entry) {
        return;
    }
    save_quest_log(&db.db, character_id, &log);
    world.resource_mut::<QuestDirty>().0.insert(character_id);
}

/// The quest area triggers the player stands in on its current map.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuestTriggerPresence {
    map: crate::maps::InMap,
    inside: Vec<u32>,
}

type TriggerPlayers<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ClientId,
        &'static Position,
        &'static crate::maps::InMap,
        Option<&'static MovementControl>,
        Option<&'static PlayerDeathState>,
        Option<&'static QuestTriggerPresence>,
    ),
    (With<Player>, Changed<Position>),
>;

/// Quest triggers containing a WoW-space point on `map_id`.
fn quest_triggers_at(quests: &QuestDb, map_id: u32, point: Vec3) -> Vec<u32> {
    quests
        .area_triggers
        .get(&map_id)
        .into_iter()
        .flatten()
        .filter(|trigger| trigger.shape.contains(trigger.center, point))
        .map(|trigger| trigger.id)
        .collect()
}

/// `WorldSession::HandleAreaTriggerOpcode` (MiscHandler.cpp:479-564) for the quest
/// triggers: the Retail client reports entering a trigger, here the server sees the
/// position cross into one (as `maps::area_triggers` does for teleports). A first
/// position on a map only records the triggers it stands in. A player in flight
/// triggers nothing; only a living one explores (`IsAlive() && packet.Entered`).
fn credit_quest_area_triggers(
    mut commands: Commands,
    players: TriggerPlayers,
    store: QuestStore,
    mut dirty: ResMut<QuestDirty>,
) {
    let Some((db, quests)) = store.get() else {
        return;
    };
    for (player, character, position, &map, control, death, presence) in &players {
        let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
        let inside = quest_triggers_at(quests, map.map_id, Vec3::new(x, y, z));
        let entered: Vec<u32> = match presence {
            Some(presence) if presence.map == map => inside
                .iter()
                .copied()
                .filter(|trigger| !presence.inside.contains(trigger))
                .collect(),
            _ => Vec::new(),
        };
        if presence.is_none_or(|presence| presence.map != map || presence.inside != inside) {
            commands
                .entity(player)
                .insert(QuestTriggerPresence { map, inside });
        }
        let in_flight = control.is_some_and(|control| control.controlled);
        let alive = death.is_none_or(|death| death.state == PlayerLifeState::Alive);
        if entered.is_empty() || in_flight || !alive {
            continue;
        }
        let mut log = load_quest_log(db, character.0);
        let mut changed = false;
        for trigger in quests.area_triggers[&map.map_id]
            .iter()
            .filter(|trigger| entered.contains(&trigger.id))
        {
            changed |= credit_area_trigger(quests, &mut log, &trigger.quests);
        }
        if changed {
            save_quest_log(db, character.0, &log);
            dirty.0.insert(character.0);
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
        let view = view_of(&log_entries(db, quests, &log, &inventory));
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
            db,
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
