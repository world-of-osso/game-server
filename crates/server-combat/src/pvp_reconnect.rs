//! TC Battleground.cpp1120–1158: offline members retain their match for300s;
//! Player.cpp18343 restores the entry point when the saved battleground is gone.
//! CharacterData keeps the current position; this separate row keeps the captured
//! entry point and exact map copy without changing CharacterData's persisted ABI.
use bevy::prelude::*;
use redb::{ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use shared::battleground::{BgTeam, template_by_id};
use shared::protocol::BattlegroundSnapshot;

use super::{InvitedBattleground, PlayerPvpState, PvpServerState};
use crate::maps::{InMap, InstanceManager, TeleportOptions, WorldLocation};
use crate::persistence::{ClientId, GameDatabase};

pub(crate) const ENTRIES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_battleground_entries");
const MAX_OFFLINE_TIME_SECS: f64 = 300.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub match_id: u64,
    map_id: u32,
    instance_id: u32,
    origin_map_id: u32,
    origin_position: [f32; 3],
    origin_orientation: f32,
    original_faction: Option<u32>,
}

impl Entry {
    pub(super) fn new(
        match_id: u64,
        map: InMap,
        origin: WorldLocation,
        original_faction: Option<u32>,
    ) -> Self {
        Self {
            match_id,
            map_id: map.map_id,
            instance_id: map.instance_id,
            origin_map_id: origin.map_id,
            origin_position: origin.position.to_array(),
            origin_orientation: origin.orientation,
            original_faction,
        }
    }

    pub(crate) fn origin(&self) -> WorldLocation {
        WorldLocation {
            map_id: self.origin_map_id,
            position: Vec3::from_array(self.origin_position),
            orientation: self.origin_orientation,
        }
    }

    fn map(&self) -> InMap {
        InMap {
            map_id: self.map_id,
            instance_id: self.instance_id,
        }
    }
}

#[derive(Debug)]
pub(super) struct OfflinePlayer {
    match_id: u64,
    remove_at: f64,
}

pub(crate) fn load(db: &redb::Database, character_id: u64) -> Option<Entry> {
    let transaction = db.begin_read().expect("begin battleground entry read");
    let table = transaction
        .open_table(ENTRIES)
        .expect("open battleground entries");
    table
        .get(character_id)
        .expect("read battleground entry")
        .map(|row| serde_json::from_slice(row.value()).expect("decode battleground entry"))
}

pub(super) fn save(world: &World, character_id: u64, entry: &Entry) {
    let Some(database) = world.get_resource::<GameDatabase>() else {
        return;
    };
    let bytes = serde_json::to_vec(entry).expect("encode battleground entry");
    let transaction = database
        .db
        .begin_write()
        .expect("begin battleground entry write");
    transaction
        .open_table(ENTRIES)
        .expect("open battleground entries")
        .insert(character_id, bytes.as_slice())
        .expect("save battleground entry");
    transaction.commit().expect("commit battleground entry");
}

pub(crate) fn remove(db: &redb::Database, character_id: u64) {
    let transaction = db.begin_write().expect("begin battleground entry removal");
    transaction
        .open_table(ENTRIES)
        .expect("open battleground entries")
        .remove(character_id)
        .expect("remove battleground entry");
    transaction
        .commit()
        .expect("commit battleground entry removal");
}

pub(crate) fn matching_map(
    state: &PvpServerState,
    character_id: u64,
    entry: &Entry,
    now: f64,
) -> Option<InMap> {
    if state
        .offline_players
        .get(&character_id)
        .is_some_and(|offline| offline.remove_at <= now || offline.match_id != entry.match_id)
    {
        return None;
    }
    let matched = state.battleground_matches.get(&entry.match_id)?;
    let map = matched.map?;
    (map == entry.map()
        && matched.accepted.contains(&character_id)
        && team_of(matched, character_id).is_some())
    .then_some(map)
}

pub(crate) fn logged_out(world: &mut World, player: Entity) {
    let Some(snapshot) = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .filter(|snapshot| snapshot.accepted)
    else {
        return;
    };
    let match_id = snapshot.match_id;
    let character_id = world
        .get::<ClientId>(player)
        .expect("battleground character id")
        .0;
    let remove_at = world.resource::<Time>().elapsed_secs_f64() + MAX_OFFLINE_TIME_SECS;
    world
        .resource_mut::<PvpServerState>()
        .offline_players
        .entry(character_id)
        .or_insert(OfflinePlayer {
            match_id,
            remove_at,
        });
    // The shared objective update drops carried objects after the player disappears.
    // Arena's existing authoritative alive census ends on the last living logout,
    // without turning a living departure into a combat death.
}

fn load_login_entry(world: &World, player: Entity, character_id: u64) -> Option<Entry> {
    let battleground = world
        .get::<InMap>(player)
        .and_then(|map| world.resource::<crate::maps::MapStore>().map(map.map_id))
        .is_some_and(|map| {
            matches!(
                map.instance_type,
                crate::maps::store::InstanceType::Arena
                    | crate::maps::store::InstanceType::Battleground
            )
        });
    if !battleground {
        return None;
    }
    world
        .get_resource::<GameDatabase>()
        .and_then(|db| load(&db.db, character_id))
}

pub(crate) fn logged_in(world: &mut World, player: Entity, character_id: u64) {
    let Some(entry) = load_login_entry(world, player, character_id) else {
        return;
    };
    let now = world.resource::<Time>().elapsed_secs_f64();
    let map = matching_map(
        world.resource::<PvpServerState>(),
        character_id,
        &entry,
        now,
    );
    if map.is_none() {
        return_to_saved_origin(world, player, character_id, &entry);
        return;
    }
    assert_eq!(
        world.get::<InMap>(player).copied(),
        map,
        "login must restore the reserved battleground copy"
    );
    restore_member(world, player, character_id, &entry);
}

fn restore_member(world: &mut World, player: Entity, character_id: u64, entry: &Entry) {
    let snapshot = build_match_snapshot(
        &world.resource::<PvpServerState>().battleground_matches[&entry.match_id],
        entry.match_id,
        character_id,
        true,
    );
    world
        .resource_mut::<PvpServerState>()
        .offline_players
        .remove(&character_id);
    world
        .get_mut::<PlayerPvpState>(player)
        .expect("login PvP state")
        .battleground = Some(snapshot);
    super::pvp_entry::restore_team_reaction(world, player);
    world
        .get_mut::<PlayerPvpState>(player)
        .expect("login PvP state")
        .battleground_original_faction = entry.original_faction;
    super::pvp_objective::send_match_snapshot(world, entry.match_id);
}

fn return_to_saved_origin(world: &mut World, player: Entity, character_id: u64, entry: &Entry) {
    if !crate::maps::teleport_to(world, player, entry.origin(), TeleportOptions::default()) {
        error!("Character {character_id} cannot return to its saved battleground entry point");
        return;
    }
    if let Some(database) = world.get_resource::<GameDatabase>() {
        remove(&database.db, character_id);
    }
}

fn team_of(matched: &InvitedBattleground, character_id: u64) -> Option<BgTeam> {
    if matched.instance.team_a_players.contains(&character_id) {
        Some(BgTeam::A)
    } else if matched.instance.team_b_players.contains(&character_id) {
        Some(BgTeam::B)
    } else {
        None
    }
}

fn describe_match(matched: &InvitedBattleground) -> (u32, String, u32) {
    match &matched.arena {
        Some(arena) => (
            arena.source.id,
            arena.source.name.clone(),
            arena.source.map_id,
        ),
        None => {
            let template =
                template_by_id(matched.instance.bg_id).expect("supported battleground template");
            (template.id, template.name.clone(), template.map_id)
        }
    }
}

pub(super) fn build_match_snapshot(
    matched: &InvitedBattleground,
    match_id: u64,
    character_id: u64,
    accepted: bool,
) -> BattlegroundSnapshot {
    let (id, name, map_id) = describe_match(matched);
    let (score_a, score_b) = matched.displayed_scores();
    BattlegroundSnapshot {
        match_id,
        battleground_id: id,
        name,
        map_id,
        team: team_of(matched, character_id).expect("retained battleground member"),
        phase: matched.instance.phase,
        accepted,
        invitation_time_left_secs: None,
        score_a,
        score_b,
        flags: matched.flag_objective(),
        outcome: matched.instance.outcome,
        players: matched.scoreboard.rows.clone(),
        honor_reward: None,
        arena: matched.arena.as_ref().map(|arena| arena.snapshot.clone()),
        world_states: matched.world_states(),
    }
}

pub(super) fn expire_offline_players(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let expired: Vec<_> = world
        .resource::<PvpServerState>()
        .offline_players
        .iter()
        .filter(|(_, offline)| offline.remove_at <= now)
        .map(|(id, offline)| (*id, offline.match_id))
        .collect();
    for (character_id, match_id) in expired {
        expire_member(world, character_id, match_id);
    }
}

fn expire_member(world: &mut World, character_id: u64, match_id: u64) {
    let result = world.resource_scope(|_, mut state: Mut<PvpServerState>| {
        state.offline_players.remove(&character_id);
        let map = state.battleground_matches.get(&match_id)?.map?;
        super::pvp_leave::remove_participant(&mut state, match_id, character_id);
        Some((map, !state.battleground_matches.contains_key(&match_id)))
    });
    let Some((map, empty)) = result else { return };
    world
        .resource_mut::<InstanceManager>()
        .unassign_battleground_player(map, character_id);
    if empty {
        if let Err(message) = super::pvp_invites::close_match_map(world, map) {
            error!("Close offline battleground {match_id}: {message}");
        }
    }
    // Keep the durable entry point until the offline character next logs in.
}
