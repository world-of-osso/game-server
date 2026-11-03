//! Profile effects resolve native catalogs, never reference-report stat or damage amounts.
use super::inputs::Profile;
use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};
#[cfg(test)]
pub(super) use server::consumables::Potion;

fn database() -> Result<Connection, String> {
    let path = std::env::var_os("DPS_PARITY_WORLD_DB").ok_or("native effect database absent")?;
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())
}

/// Omnium is a separate expansion loadout, not a consumable.
pub(super) fn install(world: &mut World, actor: Entity, profile: &Profile) -> Result<(), String> {
    let db = database()?;
    install_expansion(world, actor, profile, &db)
}

fn install_expansion(
    world: &mut World,
    actor: Entity,
    profile: &Profile,
    db: &Connection,
) -> Result<(), String> {
    let Some(entries) = &profile.omnium_talents else {
        return Ok(());
    };
    // unique_gear.cpp3677–3705: numeric tokens are TraitNodeEntry IDs, not nodes or class talents.
    for entry in entries.split('/') {
        let (entry, _) = entry.split_once(':').unwrap_or((entry, "1"));
        let entry = resolve_expansion_entry(entry)?;
        let spell: u32 = db.query_row("SELECT d.SpellID FROM trait_node_entry e JOIN trait_definition d ON d.ID=e.TraitDefinitionID WHERE e.ID=?1",[entry],|r|r.get(0)).map_err(|e| format!("native expansion entry{entry}: {e}"))?;
        server::consumables::apply(world, actor, spell, None)?;
    }
    Ok(())
}

// SimC76f8118 unique_gear.cpp3681–3698 resolves tokenized expansion names or
// numeric TraitNodeEntry IDs. These identities are native70448 Omnium tree1186.
const NAMED_EXPANSION_ENTRIES: &[(&str, u32)] = &[
    ("rune_of_unleashed_fire", 136822),
    ("rune_of_selfmending", 136819),
    ("rune_of_lingering", 136817),
    ("rune_of_critical_power", 136815),
    ("rune_of_masterful_cunning", 136818),
    ("rune_of_overload", 136814),
    ("rune_of_lynxlike_reflexes", 136823),
    ("rune_of_burning_haste", 136821),
];

fn resolve_expansion_entry(token: &str) -> Result<u32, String> {
    if let Some((_, entry)) = NAMED_EXPANSION_ENTRIES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(token))
    {
        return Ok(*entry);
    }
    token
        .parse()
        .map_err(|error| format!("native expansion entry {token:?}: {error}"))
}

pub(super) fn supported(kind: &str, value: &str) -> bool {
    let values = [(kind.to_owned(), value.to_owned())].into_iter().collect();
    server::followers::consumables::simc(&values).is_ok()
}
