//! Additive, level-gated project policy over native player starter spellbooks.
use bevy::prelude::*;
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

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Target {
    #[default]
    Enemy,
    #[serde(rename = "self")]
    SelfUnit,
    Party,
    Tank,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Conditions {
    pub target: Target,
    pub health_below: Option<f32>,
    pub missing_aura: Option<u32>,
    pub required_aura: Option<u32>,
    pub no_repeat_group: Vec<u32>,
    pub min_enemies: usize,
    pub min_injured: usize,
    pub attacked: bool,
    pub lost_threat: bool,
    pub combat: bool,
    pub ground: bool,
    pub source_center: bool,
    pub exclusive_aura: bool,
    pub dispel_types: Vec<u32>,
    pub requires_cast: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub spec_id: u32,
    pub spell_id: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub slot: Slot,
    pub conditions: Conditions,
}

impl Entry {
    pub fn available(&self, spec: u32, level: u8) -> bool {
        self.spec_id == spec && self.min_level <= level && level <= self.max_level
    }
}

pub(super) fn load(db: &Connection) -> rusqlite::Result<Vec<Entry>> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='follower_spec_kit')",
        [], |row| row.get(0),
    )?;
    // User-requested compatibility: unpublished/absent kit rows use legacy slots.
    if !exists {
        return Ok(Vec::new());
    }
    let mut query = db.prepare("SELECT spec_id,spell_id,min_level,max_level,role_slot,conditions FROM follower_spec_kit ORDER BY priority,spell_id,min_level")?;
    query
        .query_map([], |row| {
            let slot: String = row.get(4)?;
            let conditions: String = row.get(5)?;
            let parse_error = |column, error| {
                rusqlite::Error::FromSqlConversionFailure(
                    column,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            };
            Ok(Entry {
                spec_id: row.get(0)?,
                spell_id: row.get(1)?,
                min_level: row.get(2)?,
                max_level: row.get(3)?,
                slot: serde_json::from_value(serde_json::Value::String(slot))
                    .map_err(|e| parse_error(4, e))?,
                conditions: serde_json::from_str(&conditions).map_err(|e| parse_error(5, e))?,
            })
        })?
        .collect()
}
