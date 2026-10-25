//! TDB1210 `playercreateinfo`: normal starts keyed by race AND class.
//! NPE/Exile's Reach columns are imported but intro selection is not implemented.

use std::collections::HashMap;

use rusqlite::Connection;

const CLASS_DEMON_HUNTER: u8 = 12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StartLocation {
    pub map_id: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

const fn start(map_id: u32, x: f32, y: f32, z: f32, orientation: f32) -> StartLocation {
    StartLocation {
        map_id,
        x,
        y,
        z,
        orientation,
    }
}

/// WORKAROUND: Kul Tiran, Dracthyr, Earthen and Haranir intros are not implemented;
/// retain the documented capital arrivals (TaxiNodes 2/23). Retire these overrides
/// when their actual starting zones and intro flows are implemented.
/// Skyborne 95/96 are the user-approved reconstruction, absent from TDB1210.
pub(crate) fn approved_start(race: u8) -> Option<StartLocation> {
    match race {
        52 | 32 | 85 | 86 => Some(start(0, -8841.06, 489.656, 109.607, 0.0)),
        70 | 84 | 91 => Some(start(1, 1798.27, -4363.27, 102.848, 0.0)),
        95 => Some(start(
            2991,
            4088.501,
            1847.5,
            976.169,
            std::f32::consts::FRAC_PI_2,
        )),
        96 => Some(start(
            2991,
            4051.417,
            1858.625,
            977.620,
            std::f32::consts::FRAC_PI_2,
        )),
        _ => None,
    }
}

pub(crate) fn load_starts(conn: &Connection) -> rusqlite::Result<HashMap<(u8, u8), StartLocation>> {
    let mut statement = conn.prepare(
        "SELECT race, class, map, position_x, position_y, position_z, orientation \
         FROM tdb_playercreateinfo ORDER BY race, class",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            (row.get(0)?, row.get(1)?),
            StartLocation {
                map_id: row.get(2)?,
                x: row.get(3)?,
                y: row.get(4)?,
                z: row.get(5)?,
                orientation: row.get(6)?,
            },
        ))
    })?;
    let authored = rows.collect::<rusqlite::Result<HashMap<_, _>>>()?;
    Ok(crate::race_class::playable_pairs()
        .filter_map(|pair| resolve_start(&authored, pair).map(|location| (pair, location)))
        .collect())
}

fn resolve_start(
    authored: &HashMap<(u8, u8), StartLocation>,
    (race, class): (u8, u8),
) -> Option<StartLocation> {
    if let Some(approved) = approved_start(race) {
        return Some(approved);
    }
    // WORKAROUND: Mardum 1481 is not implemented. Every offered DH pair,
    // including pairs absent from TDB's DH rows, uses its race's authentic
    // lowest-ID non-DH class start. Retire when Mardum is implemented.
    if class == CLASS_DEMON_HUNTER {
        return authored
            .iter()
            .filter(|((source_race, source_class), _)| {
                *source_race == race && *source_class != CLASS_DEMON_HUNTER
            })
            .min_by_key(|((_, source_class), _)| *source_class)
            .map(|(_, location)| *location);
    }
    authored.get(&(race, class)).copied()
}
