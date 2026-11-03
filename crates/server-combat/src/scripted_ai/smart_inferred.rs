//! WCL-sampled combat timers for creatures without authored scripts.
use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};

use super::{ACTION_CAST, EVENT_JUST_SUMMONED, EVENT_UPDATE_IC, SmartEvent, TARGET_SELF};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimingSamples {
    pub first: Vec<u32>,
    pub repeat: Vec<u32>,
}

/// Load explicitly inferred timings. An absent optional table means there is no
/// inferred dataset; it does not change the authored SmartAI path.
pub(crate) fn load(
    conn: &Connection,
) -> rusqlite::Result<HashMap<(u32, u32, u32), Vec<SmartEvent>>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='content_inferred_creature_ability')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        bevy::log::info!("No inferred creature ability dataset in world.db");
        return Ok(HashMap::new());
    }
    let mut query = conn.prepare(
        "SELECT map, difficulty, entry, spell, target_type FROM content_inferred_creature_ability ORDER BY map, difficulty, entry, spell",
    )?;
    let mut sample_query = conn.prepare(
        "SELECT sample_id, timing_kind, delay_ms FROM content_inferred_creature_ability_sample WHERE map=?1 AND difficulty=?2 AND entry=?3 AND spell=?4 ORDER BY sample_id",
    )?;
    let mut result: HashMap<(u32, u32, u32), Vec<SmartEvent>> = HashMap::new();
    let mut rows = query.query([])?;
    while let Some(row) = rows.next()? {
        let map: u32 = row.get(0)?;
        let difficulty: u32 = row.get(1)?;
        let entry: u32 = row.get(2)?;
        let spell: u32 = row.get(3)?;
        let target_type: u32 = row.get(4)?;
        if !matches!(target_type, 1 | 2 | 5 | 6) {
            return Err(invalid_data(format!(
                "content_inferred_creature_ability map={map} difficulty={difficulty} entry={entry} spell={spell}: unsupported target_type={target_type}"
            )));
        }
        let template: Option<(String, String)> = conn
            .query_row(
                "SELECT AIName, ScriptName FROM content_creature_template WHERE entry=?1",
                [entry],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if template
            .as_ref()
            .is_none_or(|(ai, script)| !ai.is_empty() || !script.is_empty())
        {
            continue;
        }
        let mut samples = TimingSamples {
            first: Vec::new(),
            repeat: Vec::new(),
        };
        let mut sample_rows = sample_query.query([map, difficulty, entry, spell])?;
        while let Some(sample) = sample_rows.next()? {
            let sample_id: i64 = sample.get(0)?;
            let kind: String = sample.get(1)?;
            let raw_delay: i64 = sample.get(2)?;
            let delay = u32::try_from(raw_delay).ok();
            match (kind.as_str(), delay) {
                ("first_activity_proxy", Some(delay)) => samples.first.push(delay),
                ("successive_cast", Some(delay)) if delay > 0 => samples.repeat.push(delay),
                _ => {
                    return Err(invalid_data(format!(
                        "content_inferred_creature_ability_sample map={map} difficulty={difficulty} entry={entry} spell={spell} sample_id={sample_id}: invalid timing_kind={kind:?} delay_ms={raw_delay}; first delay must be nonnegative and successive delay must be positive"
                    )));
                }
            }
        }
        if samples.first.is_empty() || samples.repeat.is_empty() {
            return Err(invalid_data(format!(
                "content_inferred_creature_ability_sample map={map} difficulty={difficulty} entry={entry} spell={spell}: requires first_activity_proxy and successive_cast samples"
            )));
        }
        result
            .entry((map, difficulty, entry))
            .or_default()
            .push(SmartEvent {
                id: spell,
                link: 0,
                event_type: EVENT_UPDATE_IC,
                phase_mask: 0,
                chance: 100,
                flags: 0,
                event_params: [0; 5],
                action_type: ACTION_CAST,
                action_params: [spell, 0, 0, 0, 0, 0, 0],
                target_type,
                target_params: [0; 4],
                target_position: [0.0; 4],
                inferred_timing: Some(samples),
            });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inferred_loader_reports_the_invalid_sample_and_reason() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE content_creature_template(entry INTEGER, AIName TEXT, ScriptName TEXT);
            INSERT INTO content_creature_template VALUES(241813,'','');
            CREATE TABLE content_inferred_creature_ability(map INTEGER, difficulty INTEGER, entry INTEGER, spell INTEGER, target_type INTEGER);
            INSERT INTO content_inferred_creature_ability VALUES(2825,8,241813,1297701,5);
            CREATE TABLE content_inferred_creature_ability_sample(map INTEGER,difficulty INTEGER,entry INTEGER,spell INTEGER,sample_id INTEGER,timing_kind TEXT,delay_ms INTEGER);
            INSERT INTO content_inferred_creature_ability_sample VALUES(2825,8,241813,1297701,42,'successive_cast',0);").unwrap();
        let error = load(&db).unwrap_err().to_string();
        for expected in [
            "content_inferred_creature_ability_sample",
            "map=2825",
            "difficulty=8",
            "entry=241813",
            "spell=1297701",
            "sample_id=42",
            "successive_cast",
            "delay_ms=0",
            "must be positive",
        ] {
            assert!(error.contains(expected), "missing {expected} in {error}");
        }
    }

    #[test]
    fn inferred_loader_loads_the_canonical_world_database() {
        let scripts = super::super::load_smart_scripts(&crate::test_data::world_db_path())
            .expect("canonical world.db inferred ability rows must load");
        assert!(scripts.4.contains_key(&(2825, 8, 241813)));
    }
}

fn invalid_data(message: String) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_CHECK),
        Some(message),
    )
}

/// TC SmartAI::IsSummonedBy dispatches SMART_EVENT_JUST_SUMMONED. These
/// WCL-observed self casts occur once on that hook, without a combat timer.
pub(crate) fn load_summon_events(
    conn: &Connection,
) -> rusqlite::Result<HashMap<(u32, u32, u32), Vec<SmartEvent>>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='content_inferred_summon_spell')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(HashMap::new());
    }
    let mut statement = conn.prepare(
        "SELECT s.map,s.difficulty,s.entry,s.spell FROM content_inferred_summon_spell s JOIN content_creature_template t ON t.entry=s.entry WHERE t.AIName='' AND t.ScriptName='' ORDER BY s.map,s.difficulty,s.entry,s.spell",
    )?;
    let mut rows = statement.query([])?;
    let mut result: HashMap<(u32, u32, u32), Vec<SmartEvent>> = HashMap::new();
    while let Some(row) = rows.next()? {
        let key = (row.get(0)?, row.get(1)?, row.get(2)?);
        let spell: u32 = row.get(3)?;
        result.entry(key).or_default().push(SmartEvent {
            id: spell,
            link: 0,
            event_type: EVENT_JUST_SUMMONED,
            phase_mask: 0,
            chance: 100,
            flags: 0,
            event_params: [0; 5],
            action_type: ACTION_CAST,
            action_params: [spell, 0, 0, 0, 0, 0, 0],
            target_type: TARGET_SELF,
            target_params: [0; 4],
            target_position: [0.0; 4],
            inferred_timing: None,
        });
    }
    Ok(result)
}
