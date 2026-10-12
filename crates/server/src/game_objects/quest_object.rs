//! Chest loot and goober quest credit (TC a352b1fa GameObject::Use).
use bevy::prelude::*;
use rusqlite::Row;
use shared::components::{Health, Position};
use shared::protocol::{GameObjectInfo, InteractionError};

use super::locks::{LockDb, LockRequirement};
use super::{GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_GOOBER};
use crate::maps::InMap;
use crate::persistence::quest_store::load_quest_log;
use crate::persistence::{ClientId, GameDatabase};
use crate::quest_data::QuestDb;

#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct QuestObject {
    quest: u32,
    loot: u32,
    lock: Vec<LockRequirement>,
    cooldown: f64,
    available_at: f64,
}

fn object_cooldown(row: &Row, kind: u8) -> rusqlite::Result<f64> {
    let consumable = kind == GAMEOBJECT_TYPE_CHEST || row.get::<_, u32>(21)? != 0;
    if !consumable {
        return Ok((row.get::<_, f64>(20)? / 1000.0).max(row.get::<_, f64>(22)?));
    }
    let respawn = row.get::<_, i32>(23)?;
    // Negative spawn timers require explicit scripted respawn (TC GameObject::LoadFromDB).
    Ok(if respawn < 0 {
        f64::INFINITY
    } else {
        f64::from(respawn)
    })
}

impl QuestObject {
    pub(super) fn from_row(row: &Row, locks: &LockDb) -> rusqlite::Result<Option<Self>> {
        let kind: u8 = row.get(6)?;
        if !matches!(kind, GAMEOBJECT_TYPE_CHEST | GAMEOBJECT_TYPE_GOOBER) {
            return Ok(None);
        }
        Ok(Some(Self {
            quest: if kind == GAMEOBJECT_TYPE_GOOBER {
                row.get::<_, i32>(19)? as u32
            } else {
                // Chest questID is highlighting/tracking metadata, not a use gate.
                0
            },
            loot: if kind == GAMEOBJECT_TYPE_CHEST {
                row.get(19)?
            } else {
                0
            },
            lock: locks.requirements(row.get(18)?)?,
            cooldown: object_cooldown(row, kind)?,
            available_at: 0.0,
        }))
    }
}

fn validate_reach(world: &World, player: Entity, object: Entity) -> Result<(), InteractionError> {
    let map = world
        .get::<InMap>(object)
        .ok_or(InteractionError::InvalidTarget)?;
    if world.get::<InMap>(player) != Some(map) {
        return Err(InteractionError::InvalidTarget);
    }
    if world
        .get::<Health>(player)
        .is_none_or(|health| health.current <= 0.0)
    {
        return Err(InteractionError::PlayerDead);
    }
    let position = world
        .get::<Position>(object)
        .ok_or(InteractionError::InvalidTarget)?;
    let player_position = world
        .get::<Position>(player)
        .ok_or(InteractionError::InvalidTarget)?;
    let bounds = world
        .get::<super::GameObjectBounds>(object)
        .ok_or(InteractionError::InvalidTarget)?;
    let reaches = bounds.reaches(
        super::wow_position(position),
        super::wow_position(player_position),
        super::GAME_OBJECT_INTERACT_DISTANCE,
    );
    if reaches {
        Ok(())
    } else {
        Err(InteractionError::TooFarAway)
    }
}

fn validate_quest_and_lock(
    world: &World,
    character: u64,
    state: &QuestObject,
) -> Result<(), InteractionError> {
    let db = world.resource::<GameDatabase>();
    let quests = world.resource::<QuestDb>();
    let inventory = crate::persistence::load_inventory(&db.db, character);
    let bag_count = |item| crate::quest_view::bag_count(&db.db, &inventory, item);
    if !super::locks::can_open(&state.lock, bag_count) {
        return Err(InteractionError::InvalidTarget);
    }
    // TC checks the goober quest only when it has a known quest template.
    if quests.quests.contains_key(&state.quest) {
        let log = load_quest_log(&db.db, character);
        let complete = crate::quest_rules::is_complete_in_log(quests, &log, state.quest, bag_count);
        if log.active(state.quest).is_none() || complete {
            return Err(InteractionError::InvalidTarget);
        }
    }
    Ok(())
}

/// Revalidate queued uses: map, life, model-box reach, authored key, quest and reuse.
pub(crate) fn use_object(
    world: &mut World,
    player: Entity,
    object: Entity,
) -> Result<(), InteractionError> {
    let info = world
        .get::<GameObjectInfo>(object)
        .ok_or(InteractionError::InvalidTarget)?
        .clone();
    let state = world
        .get::<QuestObject>(object)
        .ok_or(InteractionError::InvalidTarget)?
        .clone();
    let character = world
        .get::<ClientId>(player)
        .ok_or(InteractionError::InvalidTarget)?
        .0;
    validate_reach(world, player, object)?;
    validate_quest_and_lock(world, character, &state)?;
    let now = world.resource::<Time>().elapsed_secs_f64();
    if now < state.available_at {
        return Err(InteractionError::InvalidTarget);
    }
    if info.go_type == GAMEOBJECT_TYPE_CHEST {
        crate::loot::open_game_object_loot(world, player, object, character, state.loot)?;
    } else {
        crate::quest::game_object_credit(world, player, object, info.entry);
    }
    world
        .get_mut::<QuestObject>(object)
        .expect("validated quest object")
        .available_at = now + state.cooldown;
    crate::achievement::record_game_object_use(world, player, info.entry);
    Ok(())
}
