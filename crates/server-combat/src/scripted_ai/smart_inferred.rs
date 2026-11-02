//! WCL-sampled combat timers for creatures without authored scripts.
use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};

use super::{ACTION_CAST, EVENT_UPDATE_IC, SmartEvent};

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
        "SELECT timing_kind, delay_ms FROM content_inferred_creature_ability_sample WHERE map=?1 AND difficulty=?2 AND entry=?3 AND spell=?4 ORDER BY sample_id",
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
            return Err(rusqlite::Error::InvalidQuery);
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
            let kind: String = sample.get(0)?;
            let delay: u32 = sample.get(1)?;
            match kind.as_str() {
                "first_activity_proxy" => samples.first.push(delay),
                "successive_cast" if delay > 0 => samples.repeat.push(delay),
                _ => return Err(rusqlite::Error::InvalidQuery),
            }
        }
        if samples.first.is_empty() || samples.repeat.is_empty() {
            return Err(rusqlite::Error::InvalidQuery);
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
