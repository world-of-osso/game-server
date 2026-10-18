use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::LeaveBattleground;

use super::{PlayerPvpState, PvpServerState};
use crate::auth::{Authenticated, InWorld};
use crate::maps::{InMap, InstanceManager, TeleportOptions, WorldLocation};
use crate::networking::PlayerLink;

pub(super) fn process_battleground_leave_requests(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &InWorld,
        &mut MessageReceiver<LeaveBattleground>,
    ), With<Authenticated>>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(connection, link, in_world, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, in_world.character_id, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, character_id, request) in requests {
        let result = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
            return_to_origin(world, &mut state, player, character_id, request.match_id)
        });
        super::pvp_invites::send_response(world, connection, player, result);
    }
}

fn return_to_origin(
    world: &mut World,
    state: &mut PvpServerState,
    player: Entity,
    character_id: u64,
    match_id: u64,
) -> Result<String, String> {
    let (map, origin) = return_destination(world, state, player, character_id, match_id)?;
    let arena = state.battleground_matches[&match_id].arena.is_some();
    let race = world
        .get::<shared::components::Player>(player)
        .ok_or("battleground player race missing")?
        .race;
    if !crate::maps::teleport_to(world, player, origin, TeleportOptions::default()) {
        return Err("return to battleground origin was refused".into());
    }
    world
        .entity_mut(player)
        .insert(crate::faction::current_pvp_faction_for_race(race));
    if arena {
        super::arena_preparation::clear_preparation(world, player);
    }
    let original_faction = {
        let mut pvp = world
            .get_mut::<PlayerPvpState>(player)
            .expect("validated player");
        pvp.battleground = None;
        pvp.battleground_original_faction.take()
    };
    if let Some(original) = original_faction {
        world
            .get_mut::<shared::components::UnitFactionTemplate>(player)
            .expect("validated player faction")
            .0 = original;
    }
    world
        .resource_mut::<InstanceManager>()
        .unassign_battleground_player(map, character_id);
    remove_participant(state, match_id, character_id);
    state.offline_players.remove(&character_id);
    if let Some(database) = world.get_resource::<crate::persistence::GameDatabase>() {
        super::reconnect::remove(&database.db, character_id);
    }
    if !state.battleground_matches.contains_key(&match_id) {
        super::pvp_invites::close_match_map(world, map)?;
    }
    Ok("returned to battleground origin".into())
}

fn return_destination(
    world: &World,
    state: &PvpServerState,
    player: Entity,
    character_id: u64,
    match_id: u64,
) -> Result<(InMap, WorldLocation), String> {
    let snapshot = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or("player is not in a battleground")?;
    if snapshot.match_id != match_id {
        return Err("battleground scoreboard is no longer current".into());
    }
    let matched = state
        .battleground_matches
        .get(&match_id)
        .ok_or("battleground match missing")?;
    if !matched.accepted.contains(&character_id) {
        return Err("player has not entered this battleground".into());
    }
    let map = matched.map.ok_or("battleground map missing")?;
    if world.get::<InMap>(player) != Some(&map) {
        return Err("battleground map transfer is still pending".into());
    }
    let origin = matched
        .origins
        .get(&character_id)
        .copied()
        .ok_or("battleground origin missing")?;
    Ok((map, origin))
}

pub(super) fn remove_participant(state: &mut PvpServerState, match_id: u64, character_id: u64) {
    let matched = state
        .battleground_matches
        .get_mut(&match_id)
        .expect("validated match");
    matched
        .instance
        .team_a_players
        .retain(|id| *id != character_id);
    matched
        .instance
        .team_b_players
        .retain(|id| *id != character_id);
    matched.accepted.remove(&character_id);
    matched.origins.remove(&character_id);
    matched.original_queues.remove(&character_id);
    let refill =
        matched.arena.is_none() && matched.instance.phase != shared::battleground::BgPhase::Ended;
    state.battleground_refill_pending |= refill;
    if matched.instance.team_a_players.is_empty() && matched.instance.team_b_players.is_empty() {
        state.battleground_matches.remove(&match_id);
    }
}
