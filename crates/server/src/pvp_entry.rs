use bevy::prelude::*;
use shared::battleground::{BgTeam, template_by_id};

use super::{PlayerPvpState, PvpServerState};
use crate::maps::{InMap, InstanceManager, MapStore, TeleportOptions, WorldLocation};

// TDB1210 battleground_template1014, confirmed against local safe-location rows.
const WARSONG_GULCH: u32 = 1014;
const ALLIANCE_ENTRY: u32 = 7051;
const HORDE_ENTRY: u32 = 7050;

pub(super) fn enter_battleground(
    world: &mut World,
    state: &mut PvpServerState,
    player: Entity,
    character_id: u64,
    match_id: u64,
) -> Result<(), String> {
    let destination = entry_location(world, player)?;
    let sources = world
        .get_resource::<crate::game_objects::GameObjectSpawns>()
        .ok_or("gameobject catalog unavailable")?;
    let required = authored_object_entries(destination.map_id)?;
    for entry in required {
        if !sources
            .0
            .iter()
            .any(|spawn| spawn.map_id == destination.map_id && spawn.info.entry == entry)
        {
            return Err(format!(
                "authored battleground object {entry} is absent; rebuild Retail source content"
            ));
        }
    }
    let current = world
        .get::<InMap>(player)
        .ok_or("current map transfer is still pending")?;
    let origin = crate::maps::transfer::current_location(world, player, current.map_id);
    create_match_map(world, state, match_id)?;
    if !crate::maps::teleport_to(world, player, destination, TeleportOptions::default()) {
        return Err("battleground map entry was refused".into());
    }
    state
        .battleground_matches
        .get_mut(&match_id)
        .expect("validated invitation")
        .origins
        .insert(character_id, origin);
    Ok(())
}

fn entry_location(world: &World, player: Entity) -> Result<WorldLocation, String> {
    let invitation = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or("player has no battleground invitation")?;
    let starts = if invitation.battleground_id == WARSONG_GULCH {
        [ALLIANCE_ENTRY, HORDE_ENTRY]
    } else {
        shared::battleground::node_rules(invitation.map_id)
            .ok_or("this battleground has no implemented entry lifecycle")?
            .starts
    };
    let location_id = starts[match invitation.team {
        BgTeam::A => 0,
        BgTeam::B => 1,
    }];
    let store = world
        .get_resource::<MapStore>()
        .ok_or("map catalog is unavailable")?;
    let destination = store
        .safe_locs
        .get(&location_id)
        .copied()
        .ok_or_else(|| format!("battleground start location {location_id} is absent"))?;
    if destination.map_id != invitation.map_id {
        return Err("battleground start location disagrees with its map".into());
    }
    Ok(destination)
}

fn authored_object_entries(map_id: u32) -> Result<Vec<u32>, String> {
    if let Some(rules) = shared::battleground::node_rules(map_id) {
        return Ok(rules.nodes.iter().map(|node| node.entry).collect());
    }
    if map_id == shared::battleground::warsong_gulch().map_id {
        return Ok(vec![
            shared::protocol::WARSONG_ALLIANCE_FLAG_ENTRY,
            shared::protocol::WARSONG_HORDE_FLAG_ENTRY,
        ]);
    }
    Err("this battleground has no implemented objectives".into())
}

fn create_match_map(
    world: &mut World,
    state: &mut PvpServerState,
    match_id: u64,
) -> Result<InMap, String> {
    let invitation = state
        .battleground_matches
        .get(&match_id)
        .ok_or("battleground invitation expired")?;
    if let Some(map) = invitation.map {
        return Ok(map);
    }
    let template =
        template_by_id(invitation.instance.bg_id).ok_or("battleground template missing")?;
    let roster: Vec<_> = invitation
        .instance
        .team_a_players
        .iter()
        .chain(&invitation.instance.team_b_players)
        .copied()
        .collect();
    let capacity = u32::from(template.team_size) * 2;
    let now = crate::maps::difficulty::now(world);
    let map = world
        .resource_scope(|world, mut instances: Mut<InstanceManager>| {
            instances.create_battleground_map(
                world.resource::<MapStore>(),
                template.map_id,
                &roster,
                capacity,
                now,
            )
        })
        .map_err(|reason| format!("battleground map reservation refused: {reason:?}"))?;
    state
        .battleground_matches
        .get_mut(&match_id)
        .expect("validated match")
        .map = Some(map);
    crate::maps::transfer::spawn_map_content(world, map);
    Ok(map)
}
