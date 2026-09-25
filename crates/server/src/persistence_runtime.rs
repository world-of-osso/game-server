use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::components::{Health, Player, Position};

use super::{CharacterData, GameDatabase, load_character, save_character};
use crate::death::PlayerDeathState;

/// Marker component linking a player entity to its character_id for periodic saves.
#[derive(Component, Debug)]
pub struct ClientId(pub u64);

/// Save interval in ticks (300 ticks at 20Hz = 15 seconds).
const SAVE_INTERVAL_TICKS: u64 = 300;

type DirtyPlayerData = (
    &'static Position,
    &'static Health,
    &'static Player,
    &'static ClientId,
    Option<&'static PlayerDeathState>,
);
type DirtyPlayerFilter = (
    Or<(
        Changed<Position>,
        Changed<Health>,
        Changed<Player>,
        Changed<PlayerDeathState>,
    )>,
    With<Player>,
    With<ClientId>,
);

#[derive(SystemParam)]
pub(super) struct DirtyPlayersQuery<'w, 's> {
    changed: Query<'w, 's, Entity, DirtyPlayerFilter>,
    players: Query<'w, 's, DirtyPlayerData, (With<Player>, With<ClientId>)>,
}

/// Periodically save the players changed since the last save. The system runs every
/// fixed tick and `Changed` only covers the previous run, so changed players are
/// collected every tick and kept until the save tick writes them.
pub(super) fn save_dirty_characters(
    db: Option<Res<GameDatabase>>,
    players: DirtyPlayersQuery,
    mut tick_count: Local<u64>,
    mut unsaved: Local<HashSet<Entity>>,
) {
    unsaved.extend(players.changed.iter());
    *tick_count += 1;
    if !(*tick_count).is_multiple_of(SAVE_INTERVAL_TICKS) {
        return;
    }
    let Some(db) = db else { return };

    let mut saved_count = 0;
    for entity in unsaved.drain() {
        let Ok((position, health, player, client_id, death)) = players.players.get(entity) else {
            continue;
        };
        save_live_character(&db.db, client_id.0, player, position, health, death);
        saved_count += 1;
    }
    if saved_count == 0 {
        return;
    }
    info!("Periodic save: {} characters written", saved_count);
}

pub(super) fn flush_all_characters_on_exit(
    db: Option<Res<GameDatabase>>,
    players: Query<(
        &Position,
        &Health,
        &Player,
        &ClientId,
        Option<&PlayerDeathState>,
    )>,
    mut exit_events: MessageReader<AppExit>,
) {
    if exit_events.read().next().is_none() {
        return;
    }

    let Some(db) = db else { return };

    let mut saved_count = 0usize;
    for (position, health, player, client_id, death) in players.iter() {
        save_live_character(&db.db, client_id.0, player, position, health, death);
        saved_count += 1;
    }
    if saved_count > 0 {
        info!("Shutdown save: {} characters written", saved_count);
    }
}

/// Save a player's live state (periodic, logout and shutdown saves).
pub fn save_live_character(
    db: &redb::Database,
    character_id: u64,
    player: &Player,
    position: &Position,
    health: &Health,
    death: Option<&PlayerDeathState>,
) {
    let mut data = load_character(db, character_id)
        .unwrap_or_else(|| CharacterData::from_components(player, position, health));
    data.sync_live_components(player, position, health);
    save_character(db, character_id, &data);
    super::save_corpse(db, character_id, ghost_corpse(death).as_ref());
}

/// Only a released ghost keeps its corpse across logins (AzerothCore saves
/// `PLAYER_FLAGS_GHOST` and the `corpse` row).
fn ghost_corpse(death: Option<&PlayerDeathState>) -> Option<super::CorpseData> {
    let death = death.filter(|death| death.state == shared::death::DeathState::Ghost)?;
    let corpse = death.corpse.as_ref()?;
    Some(super::CorpseData {
        map_id: corpse.map_id,
        x: corpse.x,
        y: corpse.y,
        z: corpse.z,
        created_at: corpse.created_at,
    })
}
