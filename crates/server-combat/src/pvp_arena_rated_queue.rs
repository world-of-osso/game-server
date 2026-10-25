//! Mainline full-party/leader/max-level checks; TC reference MMR queue window.
use super::{PlayerPvpQueue, PlayerPvpState, PvpServerState, arena, arena_rating};
use crate::auth::{Authenticated, InWorld};
use crate::group::GroupRegistry;
use crate::networking::PlayerLink;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::battleground::BgTeam;
use shared::components::Level;
use shared::protocol::{ArenaQueueMode, PvpBracketSnapshot, QueueForRatedPvp};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "pvp_personal_rating_tests.rs"]
mod achievement_tests;

pub(super) fn bracket(mode: ArenaQueueMode) -> PvpBracketSnapshot {
    match mode {
        ArenaQueueMode::Rated2v2 => PvpBracketSnapshot::Arena2v2,
        ArenaQueueMode::Rated3v3 => PvpBracketSnapshot::Arena3v3,
        _ => panic!("rated mode required"),
    }
}
fn mode(bracket: &PvpBracketSnapshot) -> Result<ArenaQueueMode, String> {
    match bracket {
        PvpBracketSnapshot::Arena2v2 => Ok(ArenaQueueMode::Rated2v2),
        PvpBracketSnapshot::Arena3v3 => Ok(ArenaQueueMode::Rated3v3),
        _ => Err("this rated mode lacks an authored match/round policy".into()),
    }
}
fn online(world: &mut World) -> BTreeMap<u64, (Entity, Entity)> {
    let mut query = world.query_filtered::<(Entity, &PlayerLink, &InWorld), With<Authenticated>>();
    query
        .iter(world)
        .map(|(connection, link, owner)| (owner.character_id, (connection, link.0)))
        .collect()
}
pub(super) fn receive_requests(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &InWorld,
        &mut MessageReceiver<QueueForRatedPvp>,
    ), With<Authenticated>>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, link, owner, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, owner.character_id, request.bracket))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, id, bracket) in requests {
        let result = queue_party(world, id, &bracket);
        match result {
            Ok((mode, mmr, members)) => {
                let online = online(world);
                for member in members {
                    let (connection, player) = online[&member];
                    super::pvp_invites::send_response(
                        world,
                        connection,
                        player,
                        Ok("queued for rated arena (TC reference)".into()),
                    );
                }
                form_matches(world, mode, Some(mmr));
            }
            Err(error) => super::pvp_invites::send_response(world, connection, player, Err(error)),
        }
    }
}
fn queue_party(
    world: &mut World,
    leader: u64,
    requested: &PvpBracketSnapshot,
) -> Result<(ArenaQueueMode, u32, Vec<u64>), String> {
    let mode = mode(requested)?;
    let groups = world
        .get_resource::<GroupRegistry>()
        .ok_or("group service unavailable")?;
    if groups.leader_of(leader) != Some(leader) {
        return Err("rated arena must be queued by the group leader".into());
    }
    let members = groups.group_members(leader);
    let size = usize::from(arena::team_size(mode).expect("rated size"));
    if members.len() != size {
        return Err(format!("rated arena requires exactly {size} group members"));
    }
    let maximum = world
        .resource::<arena::Catalog>()
        .0
        .iter()
        .map(|map| map.levels.max_level)
        .max()
        .ok_or("arena source catalog is empty")?;
    let online = online(world);
    let mut levels = Vec::new();
    let mut ratings = Vec::new();
    for member in &members {
        let (_, entity) = online
            .get(member)
            .ok_or("all rated group members must be online")?;
        let level = world
            .get::<Level>(*entity)
            .ok_or("member level unavailable")?
            .0;
        if level != maximum {
            return Err("rated arena requires a max-level group".into());
        }
        let player = world
            .get::<PlayerPvpState>(*entity)
            .ok_or("member PvP state unavailable")?;
        if player.queue.is_some() || player.battleground.is_some() {
            return Err("group member is already queued or invited".into());
        }
        // TC Group.cpp1250–1253 applies the LFG exclusion to rated arena rosters too.
        let using_lfg = world
            .get_resource::<crate::lfg::LfgServerState>()
            .is_some_and(|lfg| lfg.is_queued_or_checking_roles(*entity));
        if using_lfg {
            return Err("group member is using dungeon finder".into());
        }
        let progress = player
            .progress
            .brackets
            .iter()
            .find(|entry| entry.bracket == *requested)
            .ok_or("rated bracket state missing")?;
        ratings.push(
            progress
                .matchmaker_rating
                .unwrap_or(arena_rating::INITIAL_MMR),
        );
        levels.push(level);
    }
    let mmr = (ratings.iter().map(|rating| u64::from(*rating)).sum::<u64>() / members.len() as u64)
        as u32;
    let queued_at = world.resource::<Time>().elapsed_secs_f64();
    {
        let mut state = world.resource_mut::<PvpServerState>();
        if state
            .rated_team_queues
            .values()
            .flatten()
            .any(|team| team.members.iter().any(|member| members.contains(member)))
        {
            return Err("group member already has a rated queue".into());
        }
        if state.battleground_matches.values().any(|matched| {
            matched
                .arena
                .as_ref()
                .and_then(|arena| arena.rated.as_ref())
                .is_some_and(|rated| {
                    !rated.applied
                        && rated
                            .players
                            .iter()
                            .any(|player| members.contains(&player.id))
                })
        }) {
            return Err("a previous rated result is still pending".into());
        }
        state
            .rated_team_queues
            .entry(mode)
            .or_default()
            .push(arena_rating::QueuedTeam {
                leader,
                members: members.clone(),
                levels,
                mmr,
                queued_at,
            });
    }
    for member in &members {
        let (_, entity) = online[member];
        world
            .get_mut::<PlayerPvpState>(entity)
            .expect("validated member")
            .queue = Some(PlayerPvpQueue::RatedBracket {
            bracket: requested.clone(),
        });
    }
    Ok((mode, mmr, members))
}
pub(super) fn match_queued_teams(world: &mut World) {
    reconcile_queues(world);
    for mode in [ArenaQueueMode::Rated2v2, ArenaQueueMode::Rated3v3] {
        form_matches(world, mode, None);
    }
}
fn reconcile_queues(world: &mut World) {
    let online = online(world);
    let queued = world.resource::<PvpServerState>().rated_team_queues.clone();
    let valid: BTreeSet<_> = queued
        .values()
        .flatten()
        .filter(|team| {
            let Some(groups) = world.get_resource::<GroupRegistry>() else {
                return false;
            };
            let members = groups.group_members(team.leader);
            groups.leader_of(team.leader) == Some(team.leader)
                && members.len() == team.members.len()
                && team
                    .members
                    .iter()
                    .all(|id| members.contains(id) && online.contains_key(id))
        })
        .map(|team| team.leader)
        .collect();
    let remaining = {
        let mut state = world.resource_mut::<PvpServerState>();
        for queue in state.rated_team_queues.values_mut() {
            queue.retain(|team| valid.contains(&team.leader));
        }
        state
            .rated_team_queues
            .values()
            .flatten()
            .flat_map(|team| team.members.iter().copied())
            .collect::<BTreeSet<_>>()
    };
    for (id, (connection, entity)) in online {
        let clear = world.get::<PlayerPvpState>(entity).is_some_and(|player| {
            matches!(player.queue, Some(PlayerPvpQueue::RatedBracket { .. }))
                && !remaining.contains(&id)
        });
        if clear {
            world
                .get_mut::<PlayerPvpState>(entity)
                .expect("PvP member")
                .queue = None;
            super::pvp_invites::send_response(
                world,
                connection,
                entity,
                Ok("rated group queue ended".into()),
            );
        }
    }
}
fn form_matches(world: &mut World, mode: ArenaQueueMode, seed: Option<u32>) {
    loop {
        let queued = world
            .resource::<PvpServerState>()
            .rated_team_queues
            .get(&mode)
            .cloned()
            .unwrap_or_default();
        let now = world.resource::<Time>().elapsed_secs_f64();
        let Some((left, right)) = arena_rating::find_pair(&queued, now, seed) else {
            return;
        };
        let (a, b) = (&queued[left], &queued[right]);
        let players = read_ratings(world, mode, a, b);
        let levels: Vec<_> = a.levels.iter().chain(&b.levels).copied().collect();
        let id = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
            arena::create_match(
                &mut state,
                world.resource::<arena::Catalog>(),
                mode,
                [a.members.clone(), b.members.clone()],
                &levels,
            )
        });
        let Some(id) = id else {
            return;
        };
        let mut state = world.resource_mut::<PvpServerState>();
        let matched = state
            .battleground_matches
            .get_mut(&id)
            .expect("created rated match");
        matched.arena.as_mut().expect("arena").rated = Some(arena_rating::RatedMatch {
            players,
            team_mmr: [a.mmr, b.mmr],
            applied: false,
        });
        let queue = state
            .rated_team_queues
            .get_mut(&mode)
            .expect("rated mode queue");
        queue.remove(right);
        queue.remove(left);
        drop(state);
        arena::publish_invitation(world, id);
    }
}
fn read_ratings(
    world: &mut World,
    mode: ArenaQueueMode,
    a: &arena_rating::QueuedTeam,
    b: &arena_rating::QueuedTeam,
) -> Vec<arena_rating::RatedPlayer> {
    let online = online(world);
    let requested = bracket(mode);
    [(BgTeam::A, &a.members), (BgTeam::B, &b.members)]
        .into_iter()
        .flat_map(|(team, members)| {
            members
                .iter()
                .map(|id| {
                    let (_, entity) = online[id];
                    let player = world.get::<PlayerPvpState>(entity).expect("queued member");
                    let entry = player
                        .progress
                        .brackets
                        .iter()
                        .find(|entry| entry.bracket == requested)
                        .expect("rated bracket");
                    let (rating, mmr) = match (entry.rating, entry.matchmaker_rating) {
                        (Some(rating), Some(mmr)) => (rating, mmr),
                        (_, None) => (
                            arena_rating::INITIAL_PERSONAL_RATING,
                            arena_rating::INITIAL_MMR,
                        ),
                        (None, Some(_)) => panic!("sourced MMR without personal rating"),
                    };
                    arena_rating::RatedPlayer {
                        id: *id,
                        rating,
                        mmr,
                        team,
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
