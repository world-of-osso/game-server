//! TC a352b1fa Arena.cpp:33-36,129-135; no fabricated match duration or awards.
use super::{InvitedBattleground, PvpServerState};
use crate::maps::InMap;
use crate::persistence::ClientId;
use bevy::prelude::*;
use shared::battleground::{BgOutcome, BgPhase, BgTeam};
use shared::components::{Health, Level, Player};
use shared::protocol::{ArenaUnitSnapshot, BattlegroundPlayerScoreSnapshot};
use std::collections::BTreeMap;

#[derive(Clone)]
struct Actor {
    map: InMap,
    unit: ArenaUnitSnapshot,
    level: u8,
}

pub(super) fn update_matches(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let mut query = world.query::<(Entity, &ClientId, &InMap, &Health, &Player, &Level)>();
    let actors: BTreeMap<_, _> = query
        .iter(world)
        .map(|(entity, id, map, health, player, level)| {
            (
                id.0,
                Actor {
                    map: *map,
                    level: level.0,
                    unit: ArenaUnitSnapshot {
                        character_id: id.0,
                        entity: entity.to_bits(),
                        name: player.name.clone(),
                        class: player.class,
                        team: BgTeam::A,
                        health: health.current.max(0.0).ceil() as u32,
                        max_health: health.max.max(0.0).ceil() as u32,
                    },
                },
            )
        })
        .collect();
    let changed = world.resource_scope(|_, mut state: Mut<PvpServerState>| {
        state
            .battleground_matches
            .iter_mut()
            .filter_map(|(id, matched)| update_match(matched, &actors, now).then_some(*id))
            .collect::<Vec<_>>()
    });
    for id in changed {
        super::pvp_objective::send_match_snapshot(world, id);
    }
}
fn update_match(
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Actor>,
    now: f64,
) -> bool {
    let Some(map) = matched.map else {
        return false;
    };
    let Some(arena) = matched.arena.as_mut() else {
        return false;
    };
    let previous = arena.snapshot.clone();
    let old_phase = matched.instance.phase;
    let teams = [
        (BgTeam::A, matched.instance.team_a_players.clone()),
        (BgTeam::B, matched.instance.team_b_players.clone()),
    ];
    let ready = teams.iter().all(|(_, ids)| {
        !ids.is_empty()
            && ids.iter().all(|id| {
                matched.accepted.contains(id)
                    && actors.get(id).is_some_and(|actor| actor.map == map)
            })
    });
    if matched.instance.phase == BgPhase::Waiting && ready {
        let start = *arena.preparation_started.get_or_insert(now);
        let remaining = (60.0 - (now - start)).ceil().max(0.0) as u32;
        arena.snapshot.preparation_remaining_secs = Some(remaining);
        if remaining == 0 {
            matched.instance.start();
            arena.started = Some(now);
        }
    }
    if matched.instance.phase == BgPhase::Ended {
        return false;
    }
    arena.snapshot.units = teams
        .into_iter()
        .flat_map(|(team, ids)| {
            ids.into_iter().filter_map(move |id| {
                actors.get(&id).map(|actor| {
                    let mut unit = actor.unit.clone();
                    unit.team = team;
                    if actor.map != map {
                        unit.health = 0;
                    }
                    unit
                })
            })
        })
        .collect();
    if matched.scoreboard.is_empty() && ready {
        matched.scoreboard = arena
            .snapshot
            .units
            .iter()
            .map(|unit| BattlegroundPlayerScoreSnapshot {
                character_id: unit.character_id,
                name: unit.name.clone(),
                class: unit.class,
                level: actors[&unit.character_id].level,
                team: unit.team,
                killing_blows: 0,
                deaths: 0,
                damage_done: 0,
                healing_done: 0,
                flag_captures: 0,
                flag_returns: 0,
                honorable_kills: None,
                honor_level: None,
            })
            .collect();
    }
    if matched.instance.phase == BgPhase::InProgress {
        for score in &mut matched.scoreboard {
            if !arena
                .snapshot
                .units
                .iter()
                .any(|unit| unit.character_id == score.character_id && unit.health > 0)
            {
                score.deaths = 1;
            }
        }
        // TC checks gold first. Simultaneous elimination therefore awards green,
        // not an invented draw policy.
        let alive = |team| {
            arena
                .snapshot
                .units
                .iter()
                .filter(|unit| unit.team == team && unit.health > 0)
                .count()
        };
        let outcome = if alive(BgTeam::A) == 0 && !matched.instance.team_b_players.is_empty() {
            Some(BgOutcome::Winner(BgTeam::B))
        } else if !matched.instance.team_a_players.is_empty() && alive(BgTeam::B) == 0 {
            Some(BgOutcome::Winner(BgTeam::A))
        } else {
            None
        };
        if let Some(outcome) = outcome {
            matched.instance.end(outcome);
        }
        arena.snapshot.shadow_sight_available =
            arena.started.is_some_and(|start| now - start >= 90.0);
    }
    previous != arena.snapshot || old_phase != matched.instance.phase
}
