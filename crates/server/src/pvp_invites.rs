use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{PvpChannel, PvpStateUpdate, RespondToBattlegroundInvite};

use super::pvp_support::build_pvp_snapshot;
use super::{PlayerPvpState, PvpServerState, respond_to_battleground_invitation};
use crate::auth::{Authenticated, InWorld};
use crate::maps::{InMap, InstanceManager};
use crate::networking::PlayerLink;

pub(super) fn process_battleground_invitation_responses(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &InWorld,
        &mut MessageReceiver<RespondToBattlegroundInvite>,
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
            apply_invitation_response(world, &mut state, player, character_id, request)
        });
        send_response(world, connection, player, result);
    }
}

fn apply_invitation_response(
    world: &mut World,
    state: &mut PvpServerState,
    player: Entity,
    character_id: u64,
    request: RespondToBattlegroundInvite,
) -> Result<String, String> {
    let invitation = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.clone())
        .ok_or("player has no battleground invitation")?;
    if invitation.match_id != request.match_id {
        return Err("battleground invitation is no longer available".into());
    }
    let matched = state
        .battleground_matches
        .get(&request.match_id)
        .ok_or("battleground invitation is no longer available")?;
    let map = matched.map;
    let maximum_level = matched.instance.template.bracket.max_level;
    // TC Handler359–365 / Battleground1528–1533: use the retained bracket, not BML max.
    let above_maximum = if request.accepted && invitation.arena.is_none() {
        let level = world
            .get::<shared::components::Level>(player)
            .ok_or("player level is unavailable")?;
        level.0 > maximum_level
    } else {
        false
    };
    // TC BattleGroundHandler.cpp347–358 turns a late BG Deserter accept into a decline.
    let deserter_error = if request.accepted && invitation.arena.is_none() {
        super::validate_no_deserter(world.get::<crate::auras::ActiveAuras>(player)).err()
    } else {
        None
    };
    let port_allowed = deserter_error.is_none() && !above_maximum;
    let accepted = request.accepted && port_allowed;
    respond_to_battleground_invitation(state, request.match_id, character_id, accepted)?;
    if accepted {
        return accept_and_teleport(
            world,
            state,
            player,
            character_id,
            request.match_id,
            &invitation.name,
        );
    }
    world
        .get_mut::<PlayerPvpState>(player)
        .expect("validated player")
        .battleground = None;
    if let Some(map) = map {
        world
            .resource_mut::<InstanceManager>()
            .unassign_battleground_player(map, character_id);
        if !state.battleground_matches.contains_key(&request.match_id) {
            close_match_map(world, map)?;
        }
    }
    if let Some(error) = deserter_error {
        return Err(error);
    }
    Ok(format!("declined {} invitation", invitation.name))
}

fn accept_and_teleport(
    world: &mut World,
    state: &mut PvpServerState,
    player: Entity,
    character_id: u64,
    match_id: u64,
    name: &str,
) -> Result<String, String> {
    if let Err(error) =
        super::pvp_entry::enter_battleground(world, state, player, character_id, match_id)
    {
        state
            .battleground_matches
            .get_mut(&match_id)
            .expect("validated invitation")
            .accepted
            .remove(&character_id);
        return Err(error);
    }
    world
        .get_mut::<PlayerPvpState>(player)
        .expect("validated player")
        .battleground
        .as_mut()
        .expect("validated invitation")
        .accepted = true;
    Ok(format!("{name} invitation accepted; map entry pending"))
}

pub(super) fn close_match_map(world: &mut World, map: InMap) -> Result<(), String> {
    if !world
        .resource_mut::<InstanceManager>()
        .close_battleground_map(map)
    {
        return Err("battleground map is still occupied or reserved".into());
    }
    crate::maps::unload_map_content(world, &[map]);
    Ok(())
}

pub(super) fn send_response(
    world: &mut World,
    connection: Entity,
    player: Entity,
    result: Result<String, String>,
) {
    let snapshot = world.get::<PlayerPvpState>(player).map(build_pvp_snapshot);
    let (message, error) = match result {
        Ok(message) => (Some(message), None),
        Err(error) => (None, Some(error)),
    };
    let Some(mut sender) = world.get_mut::<MessageSender<PvpStateUpdate>>(connection) else {
        error!("PvP response connection {connection} lacks its registered sender");
        return;
    };
    sender.send::<PvpChannel>(PvpStateUpdate {
        snapshot,
        message,
        error,
    });
}
