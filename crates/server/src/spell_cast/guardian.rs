//! Local creature spell bindings for temporary actors.
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct GuardianSpells(pub HashMap<u32, Vec<u32>>);

pub fn load_guardian_spells(conn: &Connection) -> rusqlite::Result<GuardianSpells> {
    let mut statement = conn
        .prepare("SELECT CreatureID, Spell FROM guardian_spells ORDER BY CreatureID, SpellIndex")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))?;
    let mut result = GuardianSpells::default();
    for row in rows {
        let (entry, spell) = row?;
        let spells = result.0.entry(entry).or_default();
        if !spells.contains(&spell) {
            spells.push(spell);
        }
    }
    Ok(result)
}
