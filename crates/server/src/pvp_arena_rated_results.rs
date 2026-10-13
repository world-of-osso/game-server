//! Apply the reference rating only to an authoritative completed arena, once.
use super::{PlayerPvpState, PvpServerState, arena_rated_queue, arena_rating};
use crate::persistence::{self, ClientId, GameDatabase};
use bevy::prelude::*;
use shared::battleground::{BgOutcome, BgTeam};
use shared::protocol::ArenaRatingChange;
use std::collections::BTreeMap;

pub(super) fn apply_result(world: &mut World, id: u64) {
    let (mode, winner, rated, map) = {
        let matched = &world.resource::<PvpServerState>().battleground_matches[&id];
        let Some(arena) = matched.arena.as_ref() else {
            return;
        };
        let Some(rated) = arena.rated.as_ref().filter(|rated| !rated.applied) else {
            return;
        };
        let Some(BgOutcome::Winner(winner)) = matched.instance.outcome else {
            return;
        };
        (arena.mode, winner, rated.clone(), arena.source.map_id)
    };
    let mut query = world.query::<(Entity, &ClientId, &PlayerPvpState)>();
    let online: BTreeMap<_, _> = query
        .iter(world)
        .map(|(entity, id, _)| (id.0, entity))
        .collect();
    let mut changes = Vec::new();
    let mut result_credits = Vec::new();
    let team_size = super::arena::team_size(mode).expect("validated rated mode");
    for before in &rated.players {
        let mut progress = match online.get(&before.id) {
            Some(entity) => world
                .get::<PlayerPvpState>(*entity)
                .expect("arena player")
                .progress
                .clone(),
            None => persistence::load_pvp(&world.resource::<GameDatabase>().db, before.id),
        };
        let bracket = arena_rated_queue::bracket(mode);
        let entry = progress
            .brackets
            .iter_mut()
            .find(|entry| entry.bracket == bracket)
            .expect("arena bracket");
        let won = before.team == winner;
        let own = if before.team == BgTeam::A { 0 } else { 1 };
        let opponent_mmr = rated.team_mmr[1 - own];
        let rating_after = before
            .rating
            .saturating_add_signed(arena_rating::rating_delta(before.rating, opponent_mmr, won));
        let mmr_delta = arena_rating::matchmaker_delta(rated.team_mmr[own], opponent_mmr, won);
        let mmr_after = before.mmr.saturating_add_signed(mmr_delta);
        entry.rating = Some(rating_after);
        entry.matchmaker_rating = Some(mmr_after);
        if won {
            entry.season_wins += 1;
            entry.weekly_wins += 1;
        } else {
            entry.season_losses += 1;
            entry.weekly_losses += 1;
        }
        persistence::save_pvp(&world.resource::<GameDatabase>().db, before.id, &progress);
        if let Some(entity) = online.get(&before.id) {
            world
                .get_mut::<PlayerPvpState>(*entity)
                .expect("arena player")
                .progress = progress;
            result_credits.push((*entity, before.rating, won));
        }
        changes.push(ArenaRatingChange {
            character_id: before.id,
            rating_before: before.rating,
            rating_after,
            mmr_before: before.mmr,
            mmr_after,
        });
    }
    {
        let mut state = world.resource_mut::<PvpServerState>();
        let arena = state
            .battleground_matches
            .get_mut(&id)
            .expect("completed match")
            .arena
            .as_mut()
            .expect("arena");
        arena.snapshot.rating_changes = changes;
        arena.rated.as_mut().expect("rated match").applied = true;
    }
    for (player, before_rating, won) in result_credits {
        crate::achievement::record_ranked_arena_result(
            world,
            player,
            before_rating,
            map,
            team_size,
            won,
        );
    }
}
