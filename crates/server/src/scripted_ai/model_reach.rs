//! Native model geometry and the named follower player-race sizing policy.
use bevy::prelude::*;
use shared::components::{ModelDisplay, Player, Position, UnitScale};
use std::collections::{HashMap, HashSet};

// TC a352b1fa src/server/game/Entities/Object/ObjectDefines.h:39-40.
const DEFAULT_PLAYER_BOUNDING_RADIUS: f32 = 0.389;
const DEFAULT_PLAYER_COMBAT_REACH: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ModelGeometry {
    pub bounding_radius: f32,
    pub combat_reach: f32,
}

impl ModelGeometry {
    pub(crate) fn scaled(self, scale: f32) -> Self {
        Self {
            bounding_radius: self.bounding_radius * scale,
            combat_reach: self.combat_reach * scale,
        }
    }
}

pub(crate) const PLAYER_MODEL_GEOMETRY: ModelGeometry = ModelGeometry {
    bounding_radius: DEFAULT_PLAYER_BOUNDING_RADIUS,
    combat_reach: DEFAULT_PLAYER_COMBAT_REACH,
};

/// Actual creature_model_info rows plus native70448 player-race display identity.
#[derive(Resource)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct ModelReaches {
    pub source: HashMap<u32, ModelGeometry>,
    player_race_displays: HashSet<u32>,
}

impl ModelReaches {
    pub(crate) fn creature_geometry(&self, display: u32, follower: bool) -> Option<ModelGeometry> {
        if follower && self.player_race_displays.contains(&display) {
            Some(PLAYER_MODEL_GEOMETRY)
        } else {
            self.source.get(&display).copied()
        }
    }
}

pub(crate) fn load_model_reaches(conn: &rusqlite::Connection) -> rusqlite::Result<ModelReaches> {
    let mut models = conn.prepare(
        "SELECT DisplayID,BoundingRadius,CombatReach FROM encounter_creature_model_info",
    )?;
    let source = models
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                ModelGeometry {
                    bounding_radius: row.get(1)?,
                    combat_reach: row.get(2)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut displays =
        conn.prepare("SELECT ID FROM creature_display_info WHERE ExtendedDisplayInfoID != 0")?;
    let player_race_displays = displays
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(ModelReaches {
        source,
        player_race_displays,
    })
}

/// FOLLOWER_PLAYER_RACE_MODEL_REACH_TC_PLAYER_DEFAULTS: only actual followers
/// with native ExtendedDisplayInfoID!=0 use the player's source constants.
/// Ordinary NPCs and non-player-race followers still require exact model-info.
pub(crate) fn model_geometry(world: &World, unit: Entity) -> Option<ModelGeometry> {
    let scale = world.get::<UnitScale>(unit).map_or(1.0, |scale| scale.0);
    if world.get::<Player>(unit).is_some() {
        return Some(PLAYER_MODEL_GEOMETRY.scaled(scale));
    }
    let follower = world.get::<crate::followers::Follower>(unit).is_some();
    let display = world.get::<ModelDisplay>(unit)?.display_id;
    world
        .get_resource::<ModelReaches>()?
        .creature_geometry(display, follower)
        .map(|geometry| geometry.scaled(scale))
}

pub(crate) fn combat_reach(world: &World, unit: Entity) -> Option<f32> {
    model_geometry(world, unit).map(|geometry| geometry.combat_reach)
}

/// `WorldObject::GetDistance(obj)` (Object.cpp:430): exact3D minus both reaches.
pub(crate) fn distance(world: &World, from: Entity, to: Entity) -> Option<f32> {
    let a = world.get::<Position>(from)?;
    let b = world.get::<Position>(to)?;
    let exact = Vec3::new(a.x, a.y, a.z).distance(Vec3::new(b.x, b.y, b.z));
    let d = exact - combat_reach(world, from)? - combat_reach(world, to)?;
    Some(d.max(0.0))
}
