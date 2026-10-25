//! Retail OverrideSpellData, FDID1002186, layout297CBDC8, ten ordered slots.
//! TC a352b1fa HandleAuraOverrideSpells adds/removes these temporary spells.
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct OverrideSpellData(pub HashMap<u32, [u32; 10]>);

pub(crate) fn load(conn: &Connection) -> rusqlite::Result<OverrideSpellData> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='override_spell_data')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(OverrideSpellData::default());
    }
    let mut statement = conn.prepare("SELECT ID, Spells_0, Spells_1, Spells_2, Spells_3, Spells_4, Spells_5, Spells_6, Spells_7, Spells_8, Spells_9 FROM override_spell_data")?;
    let rows = statement.query_map([], |row| {
        let mut spells = [0; 10];
        for (index, spell) in spells.iter_mut().enumerate() {
            *spell = row.get(index + 1)?;
        }
        Ok((row.get::<_, u32>(0)?, spells))
    })?;
    Ok(OverrideSpellData(rows.collect::<rusqlite::Result<_>>()?))
}

pub(super) fn load_startup(mut commands: Commands) {
    #[cfg(any(test, feature = "test-support"))]
    let path = crate::test_data::world_db_path();
    #[cfg(not(any(test, feature = "test-support")))]
    let path = std::path::PathBuf::from(crate::spell_info::WORLD_DB_PATH);
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("open world.db for OverrideSpellData");
    let data = load(&conn).expect("load OverrideSpellData");
    if data.0.is_empty() {
        warn!(
            "OverrideSpellData is not imported: aura293 toys remain unavailable; scripts/db2.py extract/import"
        );
    }
    commands.insert_resource(data);
}

pub(super) fn spells(world: &World, id: u32) -> Option<&[u32; 10]> {
    world.get_resource::<OverrideSpellData>()?.0.get(&id)
}

pub(crate) fn supported(world: &World, id: u32) -> bool {
    let store = world.resource::<crate::spell_info::SpellStore>();
    spells(world, id).is_some_and(|spells| {
        spells.iter().any(|&id| id != 0)
            && spells.iter().all(|&id| id == 0 || store.get(id).is_some())
    })
}
