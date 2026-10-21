//! Authored unit reach and the approved `MISSING_MODEL_INFO_TC_PLAYER_DEFAULTS_SCALED`
//! project policy. TC a352b1fa supplies the default dimensions, not missing-row admission.
use bevy::prelude::*;
use shared::components::{ModelDisplay, Player, Position, UnitScale};
use std::collections::HashMap;

/// `DEFAULT_PLAYER_COMBAT_REACH`, scaled by `Player::SetObjectScale` (Player.cpp:1577).
pub(crate) const PLAYER_COMBAT_REACH: f32 = 1.5;
/// TC a352b1fa ObjectDefines.h:39, rounded to its f32 representation.
pub(crate) const PLAYER_BOUNDING_RADIUS: f32 = 0.389;

/// `creature_model_info.CombatReach` by display, from the prepared
/// `encounter_creature_model_info` rows (scripts/encounter_model_geometry.py).
#[derive(Resource)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct ModelReaches(pub(crate) HashMap<u32, f32>);

pub(crate) fn load_model_reaches(conn: &rusqlite::Connection) -> rusqlite::Result<ModelReaches> {
    let mut statement =
        conn.prepare("SELECT DisplayID,CombatReach FROM encounter_creature_model_info")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(ModelReaches(rows.collect::<rusqlite::Result<_>>()?))
}

/// Pure `MISSING_MODEL_INFO_TC_PLAYER_DEFAULTS_SCALED` dimension decision.
/// Callers must establish actual display identity and load their source catalog first.
pub(crate) fn scaled_model_dimension(authored: Option<f32>, default: f32, scale: f32) -> f32 {
    authored.unwrap_or(default) * scale
}

/// Authored model reach wins; an actual display missing model-info uses the approved
/// TC player default. `UnitScale` already includes creature and display scale.
pub(crate) fn creature_combat_reach(models: &ModelReaches, display: u32, scale: f32) -> f32 {
    scaled_model_dimension(models.0.get(&display).copied(), PLAYER_COMBAT_REACH, scale)
}

/// A player's default reach, or a creature's authored/project-default reach.
/// Missing display identity or an uninitialized source catalog remains unavailable.
pub(crate) fn combat_reach(world: &World, unit: Entity) -> Option<f32> {
    let scale = world.get::<UnitScale>(unit).map_or(1.0, |scale| scale.0);
    if world.get::<Player>(unit).is_some() {
        return Some(PLAYER_COMBAT_REACH * scale);
    }
    let display = world.get::<ModelDisplay>(unit)?.display_id;
    let models = world.get_resource::<ModelReaches>()?;
    Some(creature_combat_reach(models, display, scale))
}

/// `WorldObject::GetDistance(obj)` (Object.cpp:430): exact 3D distance minus both
/// combat reaches, floored at zero.
pub(crate) fn distance(world: &World, from: Entity, to: Entity) -> Option<f32> {
    let a = world.get::<Position>(from)?;
    let b = world.get::<Position>(to)?;
    let exact = Vec3::new(a.x, a.y, a.z).distance(Vec3::new(b.x, b.y, b.z));
    let d = exact - combat_reach(world, from)? - combat_reach(world, to)?;
    Some(d.max(0.0))
}
