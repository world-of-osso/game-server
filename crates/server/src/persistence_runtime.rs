use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use shared::components::{Health, Player, Position};

use redb::ReadableTable;

use super::{CHARACTER_CORPSES, CHARACTERS, CharacterData, GameDatabase};
use crate::death::PlayerDeathState;
use crate::maps::persistence::CHARACTER_MAPS;
use crate::maps::{InMap, MapTransfer};

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
    Option<&'static InMap>,
    Option<&'static MapTransfer>,
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

    let characters: Vec<_> = unsaved
        .drain()
        .filter_map(|entity| players.players.get(entity).ok())
        .map(live_character)
        .collect();
    if characters.is_empty() {
        return;
    }
    save_live_characters(&db.db, &characters);
    info!("Periodic save: {} characters written", characters.len());
}

pub(super) fn flush_all_characters_on_exit(
    db: Option<Res<GameDatabase>>,
    players: Query<DirtyPlayerData>,
    mut exit_events: MessageReader<AppExit>,
) {
    if exit_events.read().next().is_none() {
        return;
    }

    let Some(db) = db else { return };

    let characters: Vec<_> = players.iter().map(live_character).collect();
    if characters.is_empty() {
        return;
    }
    save_live_characters(&db.db, &characters);
    info!("Shutdown save: {} characters written", characters.len());
}

/// What `save_live_character` writes for one player.
struct LiveCharacter<'a> {
    character_id: u64,
    player: &'a Player,
    position: &'a Position,
    health: &'a Health,
    death: Option<&'a PlayerDeathState>,
    map_id: u32,
}

fn live_character<'a>(
    (position, health, player, client_id, death, map, transfer): <DirtyPlayerData as bevy::ecs::query::QueryData>::Item<'a, '_>,
) -> LiveCharacter<'a> {
    LiveCharacter {
        character_id: client_id.0,
        player,
        position,
        health,
        death,
        map_id: crate::maps::saved_map_id(map, transfer),
    }
}

/// Save a player's live state (logout and single-character saves).
pub fn save_live_character(
    db: &redb::Database,
    character_id: u64,
    player: &Player,
    position: &Position,
    health: &Health,
    death: Option<&PlayerDeathState>,
    map_id: u32,
) {
    save_live_characters(
        db,
        &[LiveCharacter {
            character_id,
            player,
            position,
            health,
            death,
            map_id,
        }],
    );
}

/// All characters in one write transaction: one commit (and disk sync) per save,
/// not three per character.
fn save_live_characters(db: &redb::Database, characters: &[LiveCharacter]) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut maps = txn
            .open_table(CHARACTER_MAPS)
            .expect("Failed to open character maps table");
        let mut rows = txn
            .open_table(CHARACTERS)
            .expect("Failed to open characters table");
        let mut corpses = txn
            .open_table(CHARACTER_CORPSES)
            .expect("Failed to open corpse table");
        for character in characters {
            let id = character.character_id;
            maps.insert(id, character.map_id)
                .expect("Failed to save character map");
            let mut data = rows
                .get(id)
                .expect("Failed to read character")
                .map(|guard| {
                    CharacterData::decode_compatible(guard.value())
                        .expect("Failed to decode character")
                })
                .unwrap_or_else(|| {
                    CharacterData::from_components(
                        character.player,
                        character.position,
                        character.health,
                    )
                });
            data.sync_live_components(character.player, character.position, character.health);
            rows.insert(id, data.encode_stored().as_slice())
                .expect("Failed to insert character");
            match ghost_corpse(character.death) {
                Some(corpse) => corpses
                    .insert(id, bitcode::encode(&corpse).as_slice())
                    .map(drop)
                    .expect("Failed to insert corpse"),
                None => corpses
                    .remove(id)
                    .map(drop)
                    .expect("Failed to remove corpse"),
            }
        }
    }
    txn.commit().expect("Failed to commit live character save");
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
