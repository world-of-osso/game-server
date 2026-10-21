//! Unit combat reach (TC a352b1fa `Unit::GetCombatReach`) from source model info.
use bevy::prelude::*;
use shared::components::{ModelDisplay, Player, Position, UnitScale};
use std::collections::HashMap;

#[cfg(test)]
#[path = "model_reach_tests.rs"]
mod tests;

/// `DEFAULT_PLAYER_COMBAT_REACH`, scaled by `Player::SetObjectScale` (Player.cpp:1577).
const PLAYER_COMBAT_REACH: f32 = 1.5;

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

/// A player's default reach, or a creature's model reach (`Creature::SetDisplayId`);
/// both times its object scale. `None` for a creature display without source model info.
pub(crate) fn combat_reach(world: &World, unit: Entity) -> Option<f32> {
    let scale = world.get::<UnitScale>(unit).map_or(1.0, |scale| scale.0);
    if world.get::<Player>(unit).is_some() {
        return Some(PLAYER_COMBAT_REACH * scale);
    }
    let display = world.get::<ModelDisplay>(unit)?.display_id;
    let reach = world.get_resource::<ModelReaches>()?.0.get(&display)?;
    Some(reach * scale)
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
