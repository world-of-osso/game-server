//! TC a352b1fa Battleground.cpp1354–1383: kill rewards are distinct from world honor.
use super::PvpServerState;
use crate::auth::{Authenticated, InWorld};
use crate::maps::InMap;
use crate::networking::PlayerLink;
use crate::persistence::ClientId;
use bevy::prelude::*;
use shared::battleground::BgPhase;
use shared::components::{Player, Position, UnitScale};
use std::collections::BTreeSet;

// TC Player.cpp1577 Player::SetObjectScale and SharedDefines DEFAULT_PLAYER_COMBAT_REACH.
const PLAYER_COMBAT_REACH: f32 = 1.5;

pub(crate) fn record_battleground_player_kill(world: &mut World, killer: Entity, victim: Entity) {
    if killer == victim
        || world.get::<Player>(killer).is_none()
        || world.get::<Player>(victim).is_none()
    {
        return;
    }
    let Some((map, team)) = kill_reward_roster(world, killer, victim) else {
        return;
    };
    let recipients = read_kill_reward_recipients(world, killer, victim, map, &team);
    for player in recipients {
        crate::achievement::record_battleground_kill(world, player, map.map_id);
    }
}

fn read_kill_reward_recipients(
    world: &mut World,
    killer: Entity,
    victim: Entity,
    map: InMap,
    team: &BTreeSet<u64>,
) -> BTreeSet<Entity> {
    let victim_position = *world
        .get::<Position>(victim)
        .expect("accepted victim has world position");
    let victim_reach = player_combat_reach(world, victim);
    let recipients: BTreeSet<_> = world
        .query::<(&Authenticated, &PlayerLink, &InWorld)>()
        .iter(world)
        .filter(|(_, link, owner)| {
            team.contains(&owner.character_id) && world.get::<InMap>(link.0) == Some(&map)
        })
        .filter(|(_, link, _)| {
            link.0 == killer || at_reward_distance(world, link.0, victim_position, victim_reach)
        })
        .map(|(_, link, _)| link.0)
        .collect();
    recipients
}

fn kill_reward_roster(
    world: &World,
    killer: Entity,
    victim: Entity,
) -> Option<(InMap, BTreeSet<u64>)> {
    let state = world.get_resource::<PvpServerState>()?;
    let map = *world.get::<InMap>(killer)?;
    if world.get::<InMap>(victim) != Some(&map) {
        return None;
    }
    let killer = world.get::<ClientId>(killer)?.0;
    let victim = world.get::<ClientId>(victim)?.0;
    let matched = state.battleground_matches.values().find(|matched| {
        let active_copy = matched.map == Some(map) && matched.instance.phase == BgPhase::InProgress;
        let accepted = matched.accepted.contains(&killer) && matched.accepted.contains(&victim);
        active_copy && accepted
    })?;
    Some((map, kill_reward_team(matched, killer, victim)?))
}

fn kill_reward_team(
    matched: &super::InvitedBattleground,
    killer: u64,
    victim: u64,
) -> Option<BTreeSet<u64>> {
    let instance = &matched.instance;
    let victim_in_roster =
        instance.team_a_players.contains(&victim) || instance.team_b_players.contains(&victim);
    if !victim_in_roster {
        return None;
    }
    let team = if instance.team_a_players.contains(&killer) {
        &instance.team_a_players
    } else if instance.team_b_players.contains(&killer) {
        &instance.team_b_players
    } else {
        return None;
    };
    Some(
        team.iter()
            .filter(|id| matched.accepted.contains(id))
            .copied()
            .collect(),
    )
}

fn player_combat_reach(world: &World, player: Entity) -> f32 {
    // Native UnitScale is stored only when modified; absence means unmodified scale1.
    let scale = world.get::<UnitScale>(player).map_or(1.0, |scale| scale.0);
    PLAYER_COMBAT_REACH * scale
}

fn reward_position(world: &World, player: Entity) -> (Position, f32) {
    if let Some(life) = world.get::<crate::death::PlayerDeathState>(player)
        && life.state != shared::death::DeathState::Alive
        && let Some(corpse) = &life.corpse
    {
        return (
            Position {
                x: corpse.x,
                y: corpse.y,
                z: corpse.z,
            },
            0.0,
        );
    }
    (
        *world
            .get::<Position>(player)
            .expect("accepted teammate has world position"),
        player_combat_reach(world, player),
    )
}

fn at_reward_distance(world: &World, player: Entity, victim: Position, victim_reach: f32) -> bool {
    // TC Player.cpp26397–26409 uses the corpse if dead; Object.cpp430–433 subtracts reaches.
    let (position, reach) = reward_position(world, player);
    let distance =
        (crate::creature_tap::distance(&position, &victim) - reach - victim_reach).max(0.0);
    distance <= crate::creature_tap::GROUP_REWARD_DISTANCE
}
