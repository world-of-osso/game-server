//! TC a352b1fa AreaTriggerDataStore.cpp398–506: composite source keys and static mode.
use std::collections::HashMap;

use bevy::math::Vec3;
use rusqlite::{Connection, Row};

use super::WorldAreaTrigger;
use crate::maps::WorldLocation;

#[derive(Debug, Clone)]
pub(super) struct Spawn {
    map_id: u32,
    difficulties: Vec<u32>,
    pub location: WorldLocation,
    pub area: WorldAreaTrigger,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Catalog(HashMap<u32, Vec<Spawn>>);

impl Catalog {
    pub(super) fn spawns(&self, map_id: u32, difficulty: u32) -> Vec<Spawn> {
        self.0
            .get(&map_id)
            .into_iter()
            .flatten()
            .filter(|spawn| spawn.difficulties.contains(&difficulty))
            .cloned()
            .collect()
    }
}

pub(crate) fn load_catalog(connection: &Connection) -> rusqlite::Result<Catalog> {
    let mut statement = connection.prepare(
        "SELECT s.SpawnId,s.AreaTriggerCreatePropertiesId,s.IsCustom,s.MapId,s.SpawnDifficulties,
         s.PosX,s.PosY,s.PosZ,s.Orientation,p.AreaTriggerId,p.IsAreatriggerCustom,
         p.SpellForVisuals,p.Shape,p.ShapeData0,p.ShapeData1,p.ShapeData2,p.ShapeData3,
         p.ShapeData4,p.ShapeData5,p.ShapeData6,p.ShapeData7,s.ScriptName,p.ScriptName,
         s.PhaseUseFlags,s.PhaseId,s.PhaseGroup,
         p.Flags|p.MoveCurveId|p.ScaleCurveId|p.MorphCurveId|p.FacingCurveId,p.TimeToTargetScale,t.Id,
         EXISTS(SELECT 1 FROM tdb_areatrigger_create_properties_orbit o
             WHERE o.AreaTriggerCreatePropertiesId=p.Id AND o.IsCustom=p.IsCustom)
         FROM tdb_areatrigger s LEFT JOIN tdb_areatrigger_create_properties p
             ON p.Id=s.AreaTriggerCreatePropertiesId AND p.IsCustom=s.IsCustom
         LEFT JOIN tdb_areatrigger_template t ON t.Id=p.AreaTriggerId AND t.IsCustom=p.IsAreatriggerCustom
         ORDER BY s.SpawnId",
    )?;
    let mut catalog = HashMap::<u32, Vec<Spawn>>::new();
    for row in statement.query_map([], decode_spawn)? {
        let spawn = row?;
        catalog.entry(spawn.map_id).or_default().push(spawn);
    }
    Ok(Catalog(catalog))
}

fn decode_spawn(row: &Row) -> rusqlite::Result<Spawn> {
    validate_static_mode(row)?;
    let location = WorldLocation {
        map_id: row.get(3)?,
        position: Vec3::new(row.get(5)?, row.get(6)?, row.get(7)?),
        orientation: row.get(8)?,
    };
    if !location.position.is_finite() || !location.orientation.is_finite() {
        return Err(invalid_spawn(row, "non-finite source pose"));
    }
    let spawn_script: String = row.get(21)?;
    // AreaTrigger.cpp1073–1089: the spawn script has priority over properties.
    let script_name = if spawn_script.is_empty() {
        row.get(22)?
    } else {
        spawn_script
    };
    Ok(Spawn {
        map_id: location.map_id,
        difficulties: crate::world_data::parse_spawn_difficulties(&row.get::<_, String>(4)?),
        location,
        area: WorldAreaTrigger {
            spawn_id: row.get(0)?,
            create_properties_id: row.get(1)?,
            create_properties_custom: row.get(2)?,
            template_id: row.get(9)?,
            template_custom: row.get(10)?,
            visual_spell: row.get(11)?,
            shape: row.get(12)?,
            shape_data: [
                row.get(13)?,
                row.get(14)?,
                row.get(15)?,
                row.get(16)?,
                row.get(17)?,
                row.get(18)?,
                row.get(19)?,
                row.get(20)?,
            ],
            script_name,
            phase_use_flags: row.get(23)?,
            phase_id: row.get(24)?,
            phase_group: row.get(25)?,
        },
    })
}

fn validate_static_mode(row: &Row) -> rusqlite::Result<()> {
    let entry: u32 = row.get(9)?;
    let template: Option<u32> = row.get(28)?;
    if entry != 0 && template.is_none() {
        return Err(invalid_spawn(row, "missing source template/custom key"));
    }
    let flags_or_curves: u32 = row.get(26)?;
    let scale_time: u32 = row.get(27)?;
    let orbit: bool = row.get(29)?;
    if flags_or_curves != 0 || scale_time != 0 || orbit {
        return Err(invalid_spawn(
            row,
            "static spawn has forbidden flags, curves, scale time or movement",
        ));
    }
    Ok(())
}

fn invalid_spawn(row: &Row, reason: &str) -> rusqlite::Error {
    let spawn_id = row.get::<_, u64>(0);
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Integer,
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("world-static AreaTrigger {spawn_id:?}: {reason}"),
        )
        .into(),
    )
}
