//! Creature vignettes (TrinityCore a352b1fa `Vignettes::VignetteData`, Unit.cpp
//! `Unit::SetVignette`): a creature whose `creature_template.VignetteID` names a
//! `Vignette` DB2 row carries [`UnitVignette`] while alive. TrinityCore sends
//! `SMSG_VIGNETTE_UPDATE` Added when the creature becomes visible to a player
//! (`Player::SendInitialVisiblePackets`) and Removed when it leaves visibility
//! (`BeforeVisibilityDestroy`) or dies (`Unit::setDeathState`, unless
//! `PersistsThroughDeath`); a replicated component on the creature does both through
//! the interest radius. Infinite-AOI vignettes (map- or zone-wide) are not modelled.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, params};
use shared::components::UnitVignette;

/// `VignetteFlags` (TrinityCore DBCEnums.h).
const INFINITE_AOI: u32 = 0x1;
const ZONE_INFINITE_AOI: u32 = 0x40;
const PERSISTS_THROUGH_DEATH: u32 = 0x80;

/// The `Vignette` DB2 row of a creature template.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CreatureVignette {
    pub(crate) id: u32,
    pub(crate) flags: u32,
}

impl CreatureVignette {
    /// `VignetteEntry::IsInfiniteAOI`.
    fn is_infinite_aoi(self) -> bool {
        self.flags & (INFINITE_AOI | ZONE_INFINITE_AOI) != 0
    }
}

/// `tdb_creature_template.VignetteID` joined with the `vignette` DB2 table, keyed by
/// template entry. Infinite-AOI vignettes are left out (logged), since a per-creature
/// replicated component cannot reach players outside the interest radius.
pub(crate) fn load_creature_vignettes(conn: &Connection) -> HashMap<u32, CreatureVignette> {
    let mut stmt = conn
        .prepare(
            "SELECT t.entry, v.ID, v.Flags FROM tdb_creature_template t
             JOIN vignette v ON v.ID = t.VignetteID",
        )
        .expect("Failed to prepare creature vignette query; run scripts/db2.py import");
    let rows = stmt
        .query_map(params![], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                CreatureVignette {
                    id: row.get::<_, i64>(1)? as u32,
                    flags: row.get::<_, i64>(2)? as u32,
                },
            ))
        })
        .expect("Failed to query creature vignettes");
    let mut vignettes = HashMap::new();
    let mut infinite = 0;
    for row in rows {
        let (entry, vignette) = row.expect("Failed to read creature vignette row");
        if vignette.is_infinite_aoi() {
            infinite += 1;
        } else {
            vignettes.insert(entry, vignette);
        }
    }
    if infinite > 0 {
        warn!("{infinite} creature templates have infinite-AOI vignettes, which are not shown");
    }
    vignettes
}

/// Spawn (`Creature::Create`) and respawn (`Creature::Respawn`): the vignette exists.
pub(crate) fn show(entity: &mut EntityCommands, vignette: CreatureVignette) {
    entity.insert((vignette, UnitVignette(vignette.id)));
}

/// Death (`Unit::setDeathState(JUST_DIED)`): the vignette goes unless it persists.
pub(crate) fn on_death(entity: &mut EntityWorldMut) {
    if let Some(vignette) = entity.get::<CreatureVignette>()
        && vignette.flags & PERSISTS_THROUGH_DEATH == 0
    {
        entity.remove::<UnitVignette>();
    }
}
