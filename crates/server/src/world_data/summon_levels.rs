//! Read-only source level inputs for temporary guardians. Ordinary NPC spawning is unchanged.
use std::collections::HashMap;

use rusqlite::Connection;

use super::CreatureTemplate;

#[derive(Clone, Default)]
pub(crate) struct SummonLevels {
    ranges: HashMap<u32, (u8, u8)>,
    maps: HashMap<(u32, u32), u32>,
    areas: HashMap<u32, (u32, u32)>,
    deltas: HashMap<(u32, u32), (i8, i8)>,
}

impl SummonLevels {
    pub(crate) fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let ranges = conn
            .prepare("SELECT ID, minlevel, maxlevel FROM content_tuning_level")?
            .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?
            .collect::<rusqlite::Result<_>>()?;
        let maps = conn
            .prepare("SELECT MapID, DifficultyID, ContentTuningID FROM map_difficulty")?
            .query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let areas = conn
            .prepare("SELECT ID, ParentAreaID, ContentTuningID FROM area_table")?
            .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?
            .collect::<rusqlite::Result<_>>()?;
        let deltas = load_deltas(conn)?;
        Ok(Self {
            ranges,
            maps,
            areas,
            deltas,
        })
    }

    /// Creature::ApplyLevelScaling: template, then map, then actual area ancestry.
    /// Critters never acquire regional tuning. Missing terrain remains unknown.
    pub(crate) fn content_tuning(
        &self,
        template: &CreatureTemplate,
        map: u32,
        difficulty: u32,
        area: Option<u32>,
    ) -> Option<u32> {
        if let Some(tuning) = &template.tuning {
            return Some(tuning.content_tuning_id);
        }
        if template.creature_type == 8 {
            return Some(0);
        }
        let map_tuning = self.maps.get(&(map, difficulty)).copied().unwrap_or(0);
        if map_tuning != 0 {
            return Some(map_tuning);
        }
        let mut area = area?;
        for _ in 0..=self.areas.len() {
            if area == 0 {
                return Some(0);
            }
            let &(parent, tuning) = self.areas.get(&area)?;
            if tuning != 0 {
                return Some(tuning);
            }
            area = parent;
        }
        None
    }

    pub(crate) fn range(&self, tuning: u32) -> Option<(u8, u8)> {
        self.ranges.get(&tuning).copied()
    }

    /// Difficulty rows replace base rows, including their deltas when tuning is regional.
    pub(crate) fn delta(&self, entry: u32, difficulty: u32) -> (i8, i8) {
        self.deltas
            .get(&(entry, difficulty))
            .or_else(|| self.deltas.get(&(entry, 0)))
            .copied()
            .unwrap_or((0, 0))
    }
}

fn load_deltas(conn: &Connection) -> rusqlite::Result<HashMap<(u32, u32), (i8, i8)>> {
    // UNION ALL preserves the existing template_for precedence: difficulty 0 replaces base.
    conn.prepare(
        "SELECT entry, 0, LevelScalingDeltaMin, LevelScalingDeltaMax
         FROM content_creature_template
         UNION ALL
         SELECT entry, DifficultyID, LevelScalingDeltaMin, LevelScalingDeltaMax
         FROM content_creature_template_difficulty",
    )?
    .query_map([], |row| {
        Ok(((row.get(0)?, row.get(1)?), (row.get(2)?, row.get(3)?)))
    })?
    .collect()
}
