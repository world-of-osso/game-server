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
        // Explicit lead-requested early-merge compatibility, retired after mandatory import.
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
