//! Read-only level inputs for temporary guardians and wild summons. Ordinary NPC spawning is unchanged.
use std::collections::HashMap;

use rusqlite::Connection;

use super::CreatureTemplate;

const CREATURE_TYPE_CRITTER: u8 = 8;

#[derive(Clone, Copy)]
pub(crate) struct GuardianLevelRange {
    pub content_tuning_id: u32,
    pub range: (u8, u8),
    pub delta: (i8, i8),
}

#[derive(Clone, Default)]
pub(crate) struct SummonLevels {
    ranges: HashMap<u32, (u8, u8)>,
    maps: HashMap<(u32, u32), u32>,
    areas: HashMap<u32, (u32, u32)>,
    deltas: HashMap<(u32, u32), (i8, i8)>,
    difficulty_fallbacks: HashMap<u32, u32>,
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
        let difficulty_fallbacks = conn
            .prepare("SELECT ID, FallbackDifficultyID FROM difficulty")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Self {
            ranges,
            maps,
            areas,
            deltas,
            difficulty_fallbacks,
        })
    }

    /// CreatureTemplate::GetDifficulty (TC a352b1fa Creature.cpp:252–262).
    pub(super) fn difficulty_chain(&self, difficulty: u32) -> impl Iterator<Item = u32> + '_ {
        std::iter::successors(Some(difficulty), |id| {
            self.difficulty_fallbacks.get(id).copied()
        })
    }

    pub(crate) fn guardian_range(
        &self,
        template: &CreatureTemplate,
        map: u32,
        difficulty: u32,
        area: Option<u32>,
    ) -> Option<GuardianLevelRange> {
        let tuning = self.content_tuning(template, map, difficulty, area)?;
        if tuning == 0 {
            // UF::UpdateFieldBase zero-initializes both scaling bounds; a missing
            // ContentTuning row never supplies the template's legacy min/max.
            return Some(GuardianLevelRange {
                content_tuning_id: 0,
                range: (0, 0),
                delta: (0, 0),
            });
        }
        Some(GuardianLevelRange {
            content_tuning_id: tuning,
            range: *self.ranges.get(&tuning)?,
            delta: self.delta(template.entry, difficulty)?,
        })
    }

    /// Creature::ApplyLevelScaling: template, then map, then actual area ancestry.
    /// Critters never acquire regional tuning. Missing terrain remains unknown.
    fn content_tuning(
        &self,
        template: &CreatureTemplate,
        map: u32,
        difficulty: u32,
        area: Option<u32>,
    ) -> Option<u32> {
        if let Some(tuning) = &template.tuning {
            return Some(tuning.content_tuning_id);
        }
        if template.creature_type == CREATURE_TYPE_CRITTER {
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

    /// Difficulty rows replace base rows, including their deltas when tuning is regional.
    fn delta(&self, entry: u32, difficulty: u32) -> Option<(i8, i8)> {
        self.difficulty_chain(difficulty)
            .find_map(|id| self.deltas.get(&(entry, id)))
            .or_else(|| self.deltas.get(&(entry, 0)))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardian51_source_projection_selects_template_map_parent_and_difficulty_delta() {
        let mut data = crate::world_data::load_world_data(&crate::test_data::world_db_path());
        let mut template = data.templates.remove(&1860).unwrap(); // Actual Voidwalker template.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE content_tuning_level (ID INTEGER, minlevel INTEGER, maxlevel INTEGER);
             INSERT INTO content_tuning_level VALUES (482,1,90),(73,1,30),(9001,10,60);
             CREATE TABLE difficulty (ID INTEGER, FallbackDifficultyID INTEGER);
             INSERT INTO difficulty VALUES (205,1),(1,0);
             CREATE TABLE map_difficulty (MapID INTEGER, DifficultyID INTEGER, ContentTuningID INTEGER);
             INSERT INTO map_difficulty VALUES (0,0,0),(0,1,9001);
             CREATE TABLE area_table (ID INTEGER, ParentAreaID INTEGER, ContentTuningID INTEGER);
             INSERT INTO area_table VALUES (9,12,0),(12,0,73),(100,0,0);
             CREATE TABLE content_creature_template (entry INTEGER, LevelScalingDeltaMin INTEGER, LevelScalingDeltaMax INTEGER);
             INSERT INTO content_creature_template VALUES (1860,2,2);
             CREATE TABLE content_creature_template_difficulty (entry INTEGER, DifficultyID INTEGER, LevelScalingDeltaMin INTEGER, LevelScalingDeltaMax INTEGER);
             INSERT INTO content_creature_template_difficulty VALUES (1860,1,-2,-2);",
        ).unwrap();
        let levels = SummonLevels::load(&conn).unwrap();
        let native = levels.guardian_range(&template, 0, 1, Some(9)).unwrap();
        assert_eq!(native.range, (1, 90), "template precedes regional tuning");
        assert_eq!(native.delta, (-2, -2), "difficulty replaces base delta");
        let follower = levels.guardian_range(&template, 0, 205, Some(9)).unwrap();
        assert_eq!(
            follower.delta,
            (-2, -2),
            "205 inherits Normal delta, not base"
        );
        template.tuning = None;
        let map = levels.guardian_range(&template, 0, 1, None).unwrap();
        assert_eq!(map.range, (10, 60), "map tuning does not need terrain");
        let parent = levels.guardian_range(&template, 0, 0, Some(9)).unwrap();
        assert_eq!(
            parent.range,
            (1, 30),
            "actual area inherits authored parent tuning"
        );
        assert_eq!(
            parent.delta,
            (2, 2),
            "regional tuning retains untuned template delta"
        );
        assert!(levels.guardian_range(&template, 0, 0, None).is_none());
        assert!(levels.guardian_range(&template, 0, 0, Some(999)).is_none());
        let untuned = levels.guardian_range(&template, 0, 0, Some(100)).unwrap();
        assert_eq!(
            untuned.range,
            (0, 0),
            "known ancestry without tuning preserves source zero bounds"
        );
        assert_eq!(untuned.delta, (0, 0));
        conn.execute("DELETE FROM content_creature_template", [])
            .unwrap();
        let missing_base_delta = SummonLevels::load(&conn).unwrap();
        assert!(
            missing_base_delta
                .guardian_range(&template, 0, 0, Some(9))
                .is_none(),
            "missing authored delta is not fabricated zero"
        );
        template.creature_type = CREATURE_TYPE_CRITTER;
        assert_eq!(
            levels
                .guardian_range(&template, 0, 1, Some(9))
                .unwrap()
                .range,
            (0, 0)
        );
    }
}

fn load_deltas(conn: &Connection) -> rusqlite::Result<HashMap<(u32, u32), (i8, i8)>> {
    let mut deltas: HashMap<_, _> = conn
        .prepare(
            "SELECT entry, 0, LevelScalingDeltaMin, LevelScalingDeltaMax
         FROM content_creature_template",
        )?
        .query_map([], |row| {
            Ok(((row.get(0)?, row.get(1)?), (row.get(2)?, row.get(3)?)))
        })?
        .collect::<rusqlite::Result<_>>()?;
    // Extend explicitly: difficulty 0 must replace base regardless of SQL row order.
    let difficulty_deltas = conn
        .prepare(
            "SELECT entry, DifficultyID, LevelScalingDeltaMin, LevelScalingDeltaMax
         FROM content_creature_template_difficulty",
        )?
        .query_map([], |row| {
            Ok(((row.get(0)?, row.get(1)?), (row.get(2)?, row.get(3)?)))
        })?
        .collect::<rusqlite::Result<HashMap<_, _>>>()?;
    deltas.extend(difficulty_deltas);
    Ok(deltas)
}
