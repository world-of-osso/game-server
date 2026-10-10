//! TrinityCore `spell_target_position` (SpellID, EffectIndex, MapID, PositionX/Y/Z,
//! Orientation). Destination rows are authored data, never inferred from spell IDs.
use crate::maps::WorldLocation;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub(crate) struct SpellTargetPositions(pub HashMap<(u32, u8), WorldLocation>);

pub(crate) fn load_positions(
    conn: &rusqlite::Connection,
) -> rusqlite::Result<SpellTargetPositions> {
    let mut stmt = conn.prepare("SELECT ID, EffectIndex, MapID, PositionX, PositionY, PositionZ, Orientation FROM spell_target_position")?;
    let positions = stmt
        .query_map([], |row| {
            Ok((
                (row.get(0)?, row.get(1)?),
                WorldLocation {
                    map_id: row.get(2)?,
                    position: Vec3::new(row.get(3)?, row.get(4)?, row.get(5)?),
                    orientation: row.get(6)?,
                },
            ))
        })?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    Ok(SpellTargetPositions(positions))
}

pub(crate) fn load_positions_startup(mut commands: Commands) {
    let result = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .and_then(|conn| load_positions(&conn));
    match result {
        Ok(positions) => commands.insert_resource(positions),
        Err(err) => {
            error!("SpellTargetPosition unavailable: {err}; fixed-destination spells cannot cast")
        }
    }
}
