//! Legacy spell IDs retained only for the physical movement fixtures.
use rusqlite::Connection;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Slot {
    Passive,
    Utility,
    Interrupt,
    Dispel,
    Control,
    Taunt,
    Heal,
    Hot,
    Defensive,
    Mitigation,
    Attack,
    Aoe,
    Buff,
}

#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub spec_id: u32,
    pub spell_id: u32,
    pub slot: Slot,
}

pub(super) fn load(db: &Connection) -> rusqlite::Result<Vec<Entry>> {
    let mut query = db.prepare("SELECT spec_id,spell_id,role_slot FROM follower_spec_kit ORDER BY priority,spell_id,min_level")?;
    query
        .query_map([], |row| {
            let slot: String = row.get(2)?;
            Ok(Entry {
                spec_id: row.get(0)?,
                spell_id: row.get(1)?,
                slot: serde_json::from_value(serde_json::Value::String(slot)).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
            })
        })?
        .collect()
}
