//! TC a352b1fa SpellMgr::LoadSpellInfoImmunities and SpellInfo::_LoadImmunityInfo.
use std::collections::HashMap;

use bevy::prelude::warn;
use rusqlite::{Connection, Row};
use serde::Serialize;

use super::SpellInfo;

const MECHANIC_IMMUNITY_MASK: u32 = 147;

/// One authentic creature_immunities row; aura147 MiscValue0 references its ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpellImmunityDefinition {
    pub id: i32,
    pub school_mask: u32,
    pub dispel_type_mask: u32,
    pub mechanics_mask: u64,
    pub effects: Vec<u32>,
    pub auras: Vec<u32>,
    pub immune_aoe: bool,
    pub immune_chain: bool,
}

impl SpellImmunityDefinition {
    /// This batch consumes mechanic/effect fields; other nonzero fields stay gaps.
    pub(crate) fn has_unmodeled_fields(&self) -> bool {
        self.school_mask != 0
            || self.dispel_type_mask != 0
            || !self.auras.is_empty()
            || self.immune_aoe
            || self.immune_chain
    }
}

fn read_ids(row: &Row, column: usize) -> rusqlite::Result<Vec<u32>> {
    let value: String = row.get(column)?;
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|part| {
            part.trim().parse::<u32>().map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    column,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .collect()
}

fn read_definition(row: &Row) -> rusqlite::Result<SpellImmunityDefinition> {
    Ok(SpellImmunityDefinition {
        id: row.get(0)?,
        school_mask: row.get(1)?,
        dispel_type_mask: row.get(2)?,
        // mysql_dump stores unsigned64 masks as signed two's-complement SQLite integers.
        mechanics_mask: row.get::<_, i64>(3)? as u64,
        effects: read_ids(row, 4)?,
        auras: read_ids(row, 5)?,
        immune_aoe: row.get(6)?,
        immune_chain: row.get(7)?,
    })
}

fn load_definitions(conn: &Connection) -> rusqlite::Result<HashMap<i32, SpellImmunityDefinition>> {
    let present: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='creature_immunities')",
        [], |row| row.get(0),
    )?;
    if !present {
        // Explicit lead-requested early-merge compatibility; never hide malformed tables.
        warn!("creature_immunities absent: native aura147 definitions remain unavailable");
        return Ok(HashMap::new());
    }
    let mut query = conn.prepare(
        "SELECT ID, SchoolMask, DispelTypeMask, MechanicsMask, Effects, Auras,
                ImmuneAoE, ImmuneChain FROM creature_immunities",
    )?;
    let definitions = query.query_map([], read_definition)?;
    definitions
        .map(|result| result.map(|definition| (definition.id, definition)))
        .collect()
}

pub(super) fn attach_definitions(
    conn: &Connection,
    spells: &mut [SpellInfo],
) -> rusqlite::Result<()> {
    let definitions = load_definitions(conn)?;
    for spell in spells {
        for effect in &mut spell.effects {
            if effect.aura == MECHANIC_IMMUNITY_MASK {
                effect.immunity = definitions
                    .get(&effect.misc_values[0])
                    .cloned()
                    .map(Box::new);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE creature_immunities (
            ID INTEGER PRIMARY KEY, SchoolMask INTEGER, DispelTypeMask INTEGER,
            MechanicsMask INTEGER, Effects TEXT, Auras TEXT,
            ImmuneAoE INTEGER, ImmuneChain INTEGER);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn class_completeness_native_immunity_loader_preserves_u64_and_csv_fields() {
        let conn = database();
        // Exact TDB1210 row1630; the mechanic mask exceeds32 bits.
        conn.execute_batch(
            "INSERT INTO creature_immunities VALUES
            (1630,0,0,69954075814,'114','5,7,11,12,26,33,455',0,0);",
        )
        .unwrap();
        let definitions = load_definitions(&conn).unwrap();
        let native = &definitions[&1630];
        assert_eq!(native.mechanics_mask, 69954075814);
        assert_eq!(native.effects, [114]);
        assert_eq!(native.auras, [5, 7, 11, 12, 26, 33, 455]);
        assert!(native.has_unmodeled_fields());
    }

    #[test]
    fn class_completeness_malformed_present_immunity_table_is_not_empty_compatibility() {
        let conn = database();
        conn.execute_batch(
            "INSERT INTO creature_immunities VALUES
            (1808,0,0,0,'98,not-an-effect','',0,0);",
        )
        .unwrap();
        assert!(load_definitions(&conn).is_err());
    }
}
