//! Chest loot and goober quest credit (TC a352b1fa GameObject::Use).
use bevy::prelude::*;
use rusqlite::Row;
use shared::components::{Health, Position};
use shared::protocol::{GameObjectInfo, InteractionError};

use crate::maps::InMap;
use crate::persistence::quest_store::load_quest_log;
use crate::persistence::{ClientId, GameDatabase};
use crate::quest_data::QuestDb;

#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct QuestObject {
    quest: u32,
    loot: u32,
    cooldown: f64,
    available_at: f64,
}

impl QuestObject {
    pub(super) fn from_row(row: &Row) -> rusqlite::Result<Option<Self>> {
        let kind: u8 = row.get(6)?;
        if !matches!(kind, 3 | 10) {
            return Ok(None);
        }
        let respawn = row.get::<_, i32>(24)?;
        let consumable = kind == 3 || row.get::<_, u32>(21)? != 0;
        let cooldown = if consumable {
            // Negative spawn timers require explicit scripted respawn (TC GameObject::LoadFromDB).
            if respawn < 0 {
                f64::INFINITY
            } else {
                f64::from(respawn)
            }
        } else {
            (row.get::<_, f64>(20)? / 1000.0).max(row.get::<_, f64>(22)?)
        };
        Ok(Some(Self {
            quest: if kind == 10 {
                row.get::<_, i32>(19)? as u32
            } else {
                row.get::<_, i32>(25)? as u32
            },
            loot: if kind == 3 { row.get(19)? } else { 0 },
            cooldown,
            available_at: 0.0,
        }))
    }
}

/// Revalidate queued uses: map, life, model-box reach, active quest and lifecycle.
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
    if !bounds.reaches(
        super::wow_position(position),
        super::wow_position(player_position),
        super::GAME_OBJECT_INTERACT_DISTANCE,
    ) {
        return Err(InteractionError::TooFarAway);
    }
    let now = world.resource::<Time>().elapsed_secs_f64();
    if now < state.available_at {
        return Err(InteractionError::InvalidTarget);
    }
    if world
        .resource::<QuestDb>()
        .quests
        .contains_key(&state.quest)
    {
        let db = world.resource::<GameDatabase>();
        let quests = world.resource::<QuestDb>();
        let log = load_quest_log(&db.db, character);
        let inventory = crate::persistence::load_inventory(&db.db, character);
        let bag_count = |item| {
            inventory
                .items
                .iter()
                .filter(|slot| slot.item_id == item)
                .map(|slot| slot.stack_count)
                .sum()
        };
        if log.active(state.quest).is_none()
            || crate::quest_rules::is_complete_in_log(quests, &log, state.quest, bag_count)
        {
            return Err(InteractionError::InvalidTarget);
        }
    }
    if info.go_type == 3 {
        return Err(InteractionError::InvalidTarget);
    } else {
        crate::quest::game_object_credit(world, player, object, info.entry);
    }
    world
        .get_mut::<QuestObject>(object)
        .expect("validated quest object")
        .available_at = now + state.cooldown;
    Ok(())
}
