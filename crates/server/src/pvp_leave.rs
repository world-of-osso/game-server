use bevy::prelude::*;
use lightyear::prelude::*;
use shared::battleground::BgPhase;
use shared::components::CombatStatus;
use shared::protocol::{ArenaQueueMode, LeaveBattleground, RequeueSkirmish};

use super::{PlayerPvpState, PvpServerState};
use crate::auth::{Authenticated, InWorld};
use crate::maps::{InMap, InstanceManager, TeleportOptions, WorldLocation};
use crate::networking::PlayerLink;

const DESERTER_SPELL_ID: u32 = 26013;

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

/// Project policy SKIRMISH_REQUEUE_LEAVE_THEN_QUEUE_SAME_MODE: Retail PVPMatchResults.lua
/// proves only the skirmish-only button; native ordering stays
/// SKIRMISH_REQUEUE_TRANSITION_UNQUALIFIED. The played mode is queued at origin ACK.
#[derive(Component, Clone, Copy)]
pub(crate) struct PendingSkirmishRequeue {
    mode: ArenaQueueMode,
    connection: Entity,
}

pub(super) fn process_skirmish_requeue_requests(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &InWorld,
        &mut MessageReceiver<RequeueSkirmish>,
    ), With<Authenticated>>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(connection, link, in_world, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, in_world.character_id, request.match_id))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, character_id, match_id) in requests {
        let result = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
            let mode = finished_skirmish_mode(world, &state, player, match_id)?;
            let returned = return_to_origin(world, &mut state, player, character_id, match_id)?;
            Ok((mode, returned))
        });
        let result = result.map(|(mode, returned)| {
            world
                .entity_mut(player)
                .insert(PendingSkirmishRequeue { mode, connection });
            returned
        });
        super::pvp_invites::send_response(world, connection, player, result);
    }
}

/// Successful origin ACK: ordinary skirmish admission; a refusal leaves the player there.
pub(crate) fn queue_skirmish_again(
    world: &mut World,
    player: Entity,
    pending: PendingSkirmishRequeue,
) {
    let Some(character_id) = world
        .get::<InWorld>(pending.connection)
        .map(|owner| owner.character_id)
    else {
        warn!("{player} left before its skirmish requeue could be queued");
        return;
    };
    super::arena::queue_and_form(
        world,
        pending.connection,
        player,
        character_id,
        pending.mode,
    );
}

fn finished_skirmish_mode(
    world: &World,
    state: &PvpServerState,
    player: Entity,
    match_id: u64,
) -> Result<ArenaQueueMode, String> {
    let current = world
        .get::<PlayerPvpState>(player)
        .and_then(|pvp| pvp.battleground.as_ref())
        .is_some_and(|snapshot| snapshot.match_id == match_id);
    let matched = state
        .battleground_matches
        .get(&match_id)
        .filter(|_| current)
        .ok_or("battleground scoreboard is no longer current")?;
    let skirmish = matched
        .arena
        .as_ref()
        .map(|arena| arena.mode)
        .filter(|mode| {
            matches!(
                mode,
                ArenaQueueMode::Skirmish2v2 | ArenaQueueMode::Skirmish3v3
            )
        });
    match skirmish {
        Some(mode) if matched.instance.phase == BgPhase::Ended => Ok(mode),
        _ => Err("Queue Again requires a finished arena skirmish".into()),
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
    let matched = &state.battleground_matches[&match_id];
    let arena = matched.arena.is_some();
    let early_battleground_exit = !arena
        && matches!(
            matched.instance.phase,
            BgPhase::Waiting | BgPhase::InProgress
        );
    let race = world
        .get::<shared::components::Player>(player)
        .ok_or("battleground player race missing")?
        .race;
    // TC Battleground.cpp846–849: resurrect dead participants fully before exit.
    let dead_participant = world
        .get::<shared::components::Health>(player)
        .is_some_and(|health| health.current <= 0.0);
    if dead_participant {
        crate::death::resurrect_player_fully(world, player);
    }
    if !crate::maps::teleport_to(world, player, origin, TeleportOptions::default()) {
        return Err("return to battleground origin was refused".into());
    }
    if !arena {
        super::remove_battleground_mount_auras(world, player);
        super::clear_battleground_preparation(world, player);
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
    // TC Player.cpp24310–24326: cast after removal/resurrection, never on arena/end exit.
    if early_battleground_exit {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: player,
                target: player,
                spell_id: DESERTER_SPELL_ID,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
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
    // TC BattleGroundHandler.cpp431–439: combat blocks leave except STATUS_WAIT_LEAVE.
    let in_combat = world
        .get::<CombatStatus>(player)
        .is_some_and(|status| status.0);
    if in_combat && matched.instance.phase != BgPhase::Ended {
        return Err("cannot leave a battlefield while in combat".into());
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
