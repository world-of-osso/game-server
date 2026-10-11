//! TDB1210 `playercreateinfo`: normal starts keyed by race AND class.
//! NPE/Exile's Reach columns are imported but intro selection is not implemented.

use std::collections::HashMap;

use rusqlite::Connection;

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
    let mut starts = HashMap::new();
    for (race, class) in crate::race_class::playable_pairs() {
        let location = if let Some(approved) = approved_start(race) {
            Some(approved)
        } else {
            let source = authored.get(&(race, class)).copied();
            // WORKAROUND: Mardum 1481 is not implemented. Keep the DH at the
            // race's authentic Warrior start; retire when Mardum is implemented.
            match source {
                Some(start) if class == 12 && start.map_id == 1481 => {
                    authored.get(&(race, 1)).copied()
                }
                other => other,
            }
        };
        if let Some(location) = location {
            starts.insert((race, class), location);
        }
    }
    Ok(starts)
}
