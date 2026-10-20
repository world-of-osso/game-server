//! TDB `spell_target_position`: authored destinations, never inferred from spell IDs.
//! Target17 consumes OrderIndex0. NULL facing uses Retail SpellEffect.PositionFacing
//! (degrees above TAU), matching TrinityCore a352b1fa LoadSpellTargetPositions.
use crate::maps::WorldLocation;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct SpellTargetPositions(pub HashMap<(u32, u8), WorldLocation>);

/// Rows whose spell effect exists in DB2 or, once prepared, in `tdb_serverside_spell_effect`
/// (TC resolves both through SpellInfo).
pub(crate) fn load_positions(
    conn: &rusqlite::Connection,
) -> rusqlite::Result<SpellTargetPositions> {
    let serverside: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='tdb_serverside_spell_effect')",
        [],
        |row| row.get(0),
    )?;
    let (serverside_join, serverside_facing, serverside_exists) = if serverside {
        (
            "LEFT JOIN tdb_serverside_spell_effect s ON s.SpellID=p.ID
                 AND s.EffectIndex=p.EffectIndex AND s.DifficultyID=0",
            "s.EffectPosFacing",
            "s.SpellID",
        )
    } else {
        ("", "NULL", "NULL")
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT p.ID, p.EffectIndex, p.MapID, p.PositionX, p.PositionY, p.PositionZ,
                p.Orientation, COALESCE(e.EffectPos_facing, {serverside_facing})
         FROM tdb_spell_target_position p
         LEFT JOIN spell_effect e ON e.SpellID=p.ID AND e.EffectIndex=p.EffectIndex
                                 AND e.DifficultyID=0
         {serverside_join}
         WHERE p.OrderIndex=0 AND (e.SpellID IS NOT NULL OR {serverside_exists} IS NOT NULL)"
    ))?;
    let positions = stmt
        .query_map([], |row| {
            Ok((
                (row.get(0)?, row.get(1)?),
                WorldLocation {
                    map_id: row.get(2)?,
                    position: Vec3::new(row.get(3)?, row.get(4)?, row.get(5)?),
                    orientation: destination_orientation(row.get(6)?, row.get(7)?),
                },
            ))
        })?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    Ok(SpellTargetPositions(positions))
}

fn destination_orientation(authored: Option<f32>, effect_facing: f32) -> f32 {
    authored.unwrap_or_else(|| {
        if effect_facing > std::f32::consts::TAU {
            effect_facing.to_radians()
        } else {
            effect_facing
        }
    })
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
