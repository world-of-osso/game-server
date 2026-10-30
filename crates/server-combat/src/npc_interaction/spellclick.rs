//! TDB npc_spellclick_spells: the imported source subset, no gossip-frame substitution.
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct ClickSpell {
    spell_id: u32,
    cast_flags: u8,
    user_type: u16,
}

#[derive(Resource, Default)]
pub(crate) struct SpellClickCatalog(BTreeMap<u32, Vec<ClickSpell>>);

pub(crate) fn load_spellclick_catalog(conn: &Connection) -> rusqlite::Result<SpellClickCatalog> {
    let mut stmt=conn.prepare("SELECT npc_entry,spell_id,cast_flags,user_type FROM content_npc_spellclick_spells ORDER BY npc_entry,spell_id")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            ClickSpell {
                spell_id: row.get(1)?,
                cast_flags: row.get(2)?,
                user_type: row.get(3)?,
            },
        ))
    })?;
    let mut catalog = SpellClickCatalog::default();
    for row in rows {
        let (entry, spell) = row?;
        catalog.0.entry(entry).or_default().push(spell);
    }
    Ok(catalog)
}

pub(crate) fn cast_spellclick(world: &mut World, player: Entity, npc: Entity, entry: u32) {
    if entry == crate::taxi::RUBY_FERRY_ENTRY {
        crate::taxi::start_ruby_ferry(world, player, npc);
        return;
    }
    let spells = world
        .resource::<SpellClickCatalog>()
        .0
        .get(&entry)
        .cloned()
        .unwrap_or_default();
    if spells.is_empty() {
        error!("Spellclick NPC {entry} has no imported source spellclick rows");
        return;
    }
    for spell in spells {
        // The implemented source row is CAST_FLAG_CASTER_CLICKER, unrestricted user.
        if spell.cast_flags != 1 || spell.user_type != 0 {
            error!(
                "Unsupported spellclick NPC {entry} spell {} flags {} user {}",
                spell.spell_id, spell.cast_flags, spell.user_type
            );
            continue;
        }
        if let Err(failure) =
            crate::spell_cast::start_spellclick_cast(world, player, spell.spell_id, npc)
        {
            warn!(
                "Spellclick NPC {entry} spell {} failed: {failure:?}",
                spell.spell_id
            );
        }
    }
}
