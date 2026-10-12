//! Meeting stones (docs/specs/meeting-stones.md): a party member targets another
//! member and uses the stone; a second member completes the summoning ritual; the
//! target gets a summon offer and accepts to be moved to the summoner.
//!
//! Using a meeting stone is TrinityCore `GameObject::Use` for
//! `GAMEOBJECT_TYPE_MEETINGSTONE` (GameObject.cpp:3188-3217): the user's target
//! must be another player in its party or raid, and both must be at least the
//! stone's ContentTuning max level. The user then casts Summoning Stone Effect
//! (59782; 61994 Ritual of Summoning for the Summoning Portal 194097) through
//! the spell pipeline, without a spellbook check. TrinityCore returns silently
//! when a check fails; here the user gets the matching cast error, and a target
//! with a summon already pending is refused up front with the check of the
//! summon spell (Spell.cpp:6551-6552). The ritual portal the cast opens is
//! `ritual`; the offer and its answer are `summon`.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, Row};
use shared::components::{Level, Player};
use shared::protocol::{
    CastFailed, GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_RITUAL, GameObjectInfo,
};
use shared::spell_data::CastFailReason;

use crate::game_objects::GameObjectBounds;
use crate::group::GroupRegistry;
use crate::networking::CombatTarget;
use crate::persistence::ClientId;

pub(crate) mod ritual;
pub(crate) mod summon;

/// Summoning Stone Effect: 5 s cast, then a 2 min channel holding the portal.
pub(crate) const SUMMONING_STONE_EFFECT: u32 = 59782;
/// Ritual of Summoning, cast at the Summoning Portal meeting stone instead.
pub(crate) const SUMMONING_PORTAL_SPELL: u32 = 61994;
/// Summoning Portal (a `GAMEOBJECT_TYPE_MEETINGSTONE`).
const SUMMONING_PORTAL: u32 = 194_097;

/// A `GAMEOBJECT_TYPE_RITUAL` template (GameObjectData.h:376-390).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RitualTemplate {
    pub info: GameObjectInfo,
    /// Model box, WoW axes, unscaled.
    pub min: Vec3,
    pub max: Vec3,
    /// Participants including the summoner (`casters`, Data0).
    pub casters: u32,
    /// Cast by the summoner once all participants clicked (`spell`, Data1).
    pub spell: u32,
    /// Participants must share the summoner's party or raid (`castersGrouped`, Data6).
    pub grouped: bool,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct MeetingStoneData {
    /// Meeting stone entry → ContentTuning max level; absent for stones without
    /// ContentTuning (`GetContentTuningData` finds no row).
    max_levels: HashMap<u32, u8>,
    rituals: HashMap<u32, RitualTemplate>,
}

impl MeetingStoneData {
    pub(crate) fn ritual(&self, entry: u32) -> Option<&RitualTemplate> {
        self.rituals.get(&entry)
    }
}

/// Stone levels from `tdb_gameobject_template.ContentTuningId` (the column the
/// merged `content_gameobject_template` lacks) through `content_tuning_level`.
const STONE_LEVEL_QUERY: &str = "SELECT t.entry, l.maxlevel \
     FROM content_gameobject_template t \
     JOIN tdb_gameobject_template tt ON tt.entry = t.entry \
     JOIN content_tuning_level l ON l.ID = tt.ContentTuningId \
     WHERE t.type = ?1";

const RITUAL_QUERY: &str = "SELECT t.entry, t.displayId, t.name, t.size, t.Data0, t.Data1, \
     t.Data6, d.GeoBox_0, d.GeoBox_1, d.GeoBox_2, d.GeoBox_3, d.GeoBox_4, d.GeoBox_5 \
     FROM tdb_gameobject_template t \
     JOIN game_object_display_info d ON d.ID = t.displayId \
     WHERE t.type = ?1";

fn ritual_from_row(row: &Row) -> rusqlite::Result<(u32, RitualTemplate)> {
    let entry: u32 = row.get(0)?;
    let geo = |index: usize| -> rusqlite::Result<f32> { Ok(row.get::<_, f64>(index)? as f32) };
    let template = RitualTemplate {
        info: GameObjectInfo {
            entry,
            go_type: GAMEOBJECT_TYPE_RITUAL,
            display_id: row.get(1)?,
            name: row.get(2)?,
            scale: row.get::<_, f64>(3)? as f32,
        },
        min: Vec3::new(geo(7)?, geo(8)?, geo(9)?),
        max: Vec3::new(geo(10)?, geo(11)?, geo(12)?),
        casters: row.get(4)?,
        spell: row.get(5)?,
        grouped: row.get::<_, i64>(6)? != 0,
    };
    Ok((entry, template))
}

pub fn load_meeting_stone_data(conn: &Connection) -> rusqlite::Result<MeetingStoneData> {
    let mut levels = conn.prepare(STONE_LEVEL_QUERY)?;
    let max_levels = levels
        .query_map([GAMEOBJECT_TYPE_MEETINGSTONE], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut rituals = conn.prepare(RITUAL_QUERY)?;
    let rituals = rituals
        .query_map([GAMEOBJECT_TYPE_RITUAL], ritual_from_row)?
        .collect::<rusqlite::Result<_>>()?;
    Ok(MeetingStoneData {
        max_levels,
        rituals,
    })
}

impl RitualTemplate {
    pub(crate) fn bounds(&self, orientation: f32) -> GameObjectBounds {
        GameObjectBounds {
            min: self.min,
            max: self.max,
            scale: self.info.scale,
            orientation,
        }
    }
}

pub struct MeetingStonePlugin;

impl Plugin for MeetingStonePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MeetingStoneData>().add_systems(
            Update,
            (summon::receive_summon_responses, ritual::expire_rituals),
        );
    }
}

/// A living player in reach used a spell-casting game object (`npc_interaction`).
pub(crate) fn use_object(world: &mut World, user: Entity, object: Entity) {
    let Some(info) = world.get::<GameObjectInfo>(object).cloned() else {
        return;
    };
    match info.go_type {
        GAMEOBJECT_TYPE_MEETINGSTONE => use_meeting_stone(world, user, info.entry),
        GAMEOBJECT_TYPE_RITUAL => ritual::use_ritual(world, user, object),
        _ => {}
    }
}

fn use_meeting_stone(world: &mut World, user: Entity, entry: u32) {
    let spell_id = if entry == SUMMONING_PORTAL {
        SUMMONING_PORTAL_SPELL
    } else {
        SUMMONING_STONE_EFFECT
    };
    if let Err(reason) = check_meeting_stone(world, user, entry) {
        crate::spell_cast::send_to_owner(
            world,
            user,
            CastFailed {
                spell_id,
                reason,
                detail: None,
            },
        );
        return;
    }
    // Refusals from the cast itself are answered by the pipeline.
    let _ = crate::spell_cast::cast_granted(world, user, spell_id, None);
}

fn check_meeting_stone(world: &World, user: Entity, entry: u32) -> Result<(), CastFailReason> {
    let target = world
        .get::<CombatTarget>(user)
        .and_then(|target| target.0)
        .filter(|&target| {
            target != user
                && world.get::<Player>(target).is_some()
                && same_group(world, user, target)
        })
        .ok_or(CastFailReason::InvalidTarget)?;
    if let Some(&max_level) = world.resource::<MeetingStoneData>().max_levels.get(&entry) {
        if level(world, user) < max_level {
            return Err(CastFailReason::LevelRequirement);
        }
        if level(world, target) < max_level {
            return Err(CastFailReason::TargetTooLowLevel);
        }
    }
    if summon::has_pending(world, target) {
        return Err(CastFailReason::SummonPending);
    }
    Ok(())
}

fn level(world: &World, player: Entity) -> u8 {
    world.get::<Level>(player).map_or(0, |level| level.0)
}

/// `Player::IsInSameRaidWith`: both players are in one party or raid.
pub(crate) fn same_group(world: &World, a: Entity, b: Entity) -> bool {
    let (Some(a), Some(b)) = (world.get::<ClientId>(a), world.get::<ClientId>(b)) else {
        return false;
    };
    world
        .get_resource::<GroupRegistry>()
        .is_some_and(|groups| groups.is_in_same_group(a.0, b.0))
}

#[cfg(test)]
mod tests;
