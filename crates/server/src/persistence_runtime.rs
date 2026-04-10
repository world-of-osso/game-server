use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::components::{Health, Player, Position};

use super::{CharacterData, GameDatabase, load_character, save_character};

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
);
type DirtyPlayerFilter = (
    Or<(Changed<Position>, Changed<Health>, Changed<Player>)>,
    With<Player>,
    With<ClientId>,
);

#[derive(SystemParam)]
pub(super) struct DirtyPlayersQuery<'w, 's> {
    query: Query<'w, 's, DirtyPlayerData, DirtyPlayerFilter>,
}

/// Periodically save all player data to the database.
pub(super) fn save_dirty_characters(
    db: Option<Res<GameDatabase>>,
    players: DirtyPlayersQuery,
    mut tick_count: Local<u64>,
) {
    *tick_count += 1;
    if !(*tick_count).is_multiple_of(SAVE_INTERVAL_TICKS) {
        return;
    }
    let Some(db) = db else { return };

    let mut saved_count = 0;
    for (position, health, player, client_id) in players.query.iter() {
        let mut data = load_character(&db.db, client_id.0)
            .unwrap_or_else(|| CharacterData::from_components(player, position, health));
        data.sync_live_components(player, position, health);
        save_character(&db.db, client_id.0, &data);
        saved_count += 1;
    }
    if saved_count == 0 {
        return;
    }
    info!("Periodic save: {} characters written", saved_count);
}

pub(super) fn flush_all_characters_on_exit(
    db: Option<Res<GameDatabase>>,
    players: Query<(&Position, &Health, &Player, &ClientId)>,
    mut exit_events: MessageReader<AppExit>,
) {
    if exit_events.read().next().is_none() {
        return;
    }

    let Some(db) = db else { return };

    let mut saved_count = 0usize;
    for (position, health, player, client_id) in players.iter() {
        let mut data = load_character(&db.db, client_id.0)
            .unwrap_or_else(|| CharacterData::from_components(player, position, health));
        data.sync_live_components(player, position, health);
        save_character(&db.db, client_id.0, &data);
        saved_count += 1;
    }
    if saved_count > 0 {
        info!("Shutdown save: {} characters written", saved_count);
    }
}
