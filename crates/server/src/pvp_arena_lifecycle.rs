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
    super::arena_shuffle::reset_pending_rounds(world);
    super::arena_preparation::sync_preparation(world);
    super::arena_objects::update_gates(world, now);
    super::arena_sight::update_sight(world, now);
    super::arena_dalaran::update(world, now);
    super::arena_cage::update(world, now);
    for id in changed {
        super::arena_rated_results::apply_result(world, id);
        finish_match(world, id);
        super::pvp_objective::send_match_snapshot(world, id);
    }
}
fn finish_match(world: &mut World, id: u64) {
    let matched = &world.resource::<PvpServerState>().battleground_matches[&id];
    if matched.instance.phase != BgPhase::Ended {
        return;
    }
    let players: Vec<_> = matched
        .arena
        .as_ref()
        .expect("arena match")
        .snapshot
        .units
        .iter()
        .map(|unit| Entity::try_from_bits(unit.entity).expect("server arena entity"))
        .collect();
    for &player in &players {
        crate::combat::combat_stop(world, player);
        crate::spell_cast::cancel_cast(world, player);
        if world
            .get::<Health>(player)
            .is_some_and(|health| health.current <= 0.0)
        {
            crate::death::resurrect_player_fully(world, player);
        }
    }
    let health: BTreeMap<_, _> = players
        .iter()
        .filter_map(|player| {
            world.get::<Health>(*player).map(|health| {
                (
                    player.to_bits(),
                    (
                        health.current.max(0.0).ceil() as u32,
                        health.max.max(0.0).ceil() as u32,
                    ),
                )
            })
        })
        .collect();
    let mut state = world.resource_mut::<PvpServerState>();
    for unit in &mut state
        .battleground_matches
        .get_mut(&id)
        .expect("finished arena")
        .arena
        .as_mut()
        .expect("arena")
        .snapshot
        .units
    {
        if let Some((current, max)) = health.get(&unit.entity) {
            unit.health = *current;
            unit.max_health = *max;
        }
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
    let scores_changed = std::mem::take(&mut arena.score_dirty);
    let previous = arena.snapshot.clone();
    let old_phase = matched.instance.phase;
    let teams = [
        (BgTeam::A, matched.instance.team_a_players.clone()),
        (BgTeam::B, matched.instance.team_b_players.clone()),
    ];
    let ready = teams.iter().all(|(_, ids)| {
        ids.len() == usize::from(super::arena::team_size(arena.mode).expect("arena team size"))
            && ids.iter().all(|id| {
                matched.accepted.contains(id)
                    && actors.get(id).is_some_and(|actor| actor.map == map)
            })
    });
    if matched.instance.phase == BgPhase::Waiting && (ready || arena.preparation_started.is_some())
    {
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
                objective_scores: Vec::new(),
                honorable_kills: None,
                honor_level: None,
            })
            .collect();
    }
    if matched.instance.phase == BgPhase::InProgress {
        let shuffle = arena.shuffle.is_some();
        for score in &mut matched.scoreboard {
            // TC Battleground.cpp:1359 counts combat deaths, not living departures.
            if actors
                .get(&score.character_id)
                .is_some_and(|actor| actor.map == map && actor.unit.health == 0)
            {
                if shuffle {
                    score.deaths += 1;
                } else {
                    score.deaths = 1;
                }
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
        let defeated = |team| {
            if shuffle {
                arena
                    .snapshot
                    .units
                    .iter()
                    .filter(|unit| unit.team == team)
                    .any(|unit| {
                        actors
                            .get(&unit.character_id)
                            .is_some_and(|actor| actor.map == map && actor.unit.health == 0)
                    })
            } else {
                alive(team) == 0
            }
        };
        let outcome = if defeated(BgTeam::A) && !matched.instance.team_b_players.is_empty() {
            Some(BgOutcome::Winner(BgTeam::B))
        } else if !matched.instance.team_a_players.is_empty() && defeated(BgTeam::B) {
            Some(BgOutcome::Winner(BgTeam::A))
        } else {
            None
        };
        arena.snapshot.shadow_sight_available =
            arena.started.is_some_and(|start| now - start >= 90.0);
        if let Some(outcome) = outcome {
            if shuffle {
                if let BgOutcome::Winner(winner) = outcome {
                    super::arena_shuffle::finish_round(matched, winner);
                }
            } else {
                matched.instance.end(outcome);
            }
        }
    }
    scores_changed
        || previous != matched.arena.as_ref().expect("arena").snapshot
        || old_phase != matched.instance.phase
}
