//! AreaTable DB2 territory metadata used by city rest.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::Connection;

use crate::faction::FactionTemplates;

/// TC a352b1fa DBCEnums.h:113, formerly AREA_FLAG_CAPITAL.
const LINKED_CHAT: u32 = 0x100;
/// AreaFlags::NoPvP, AreaTableEntry::IsSanctuary.
const NO_PVP: u32 = 0x800;

#[derive(Debug, Clone)]
struct AreaEntry {
    parent: u32,
    map_id: u32,
    flags: u32,
    faction_group: u32,
}

#[derive(Resource, Default)]
pub(crate) struct AreaTable(HashMap<u32, AreaEntry>);

impl AreaTable {
    /// TC TerrainInfo::GetZoneId follows ParentAreaID to the top-level zone.
    fn zone(&self, mut area_id: u32, map_id: u32) -> Option<&AreaEntry> {
        loop {
            let area = self.0.get(&area_id)?;
            if area.map_id != map_id {
                return None;
            }
            if area.parent == 0 {
                return Some(area);
            }
            area_id = area.parent;
        }
    }

    /// Player::UpdateZone / UpdateHostileAreaState: a capital permits rest unless
    /// its territory is hostile to the player's faction (sanctuaries allow both).
    pub(crate) fn permits_city_rest(
        &self,
        area_id: u32,
        map_id: u32,
        race: u8,
        factions: &FactionTemplates,
    ) -> bool {
        let Some(zone) = self.zone(area_id, map_id) else {
            return false;
        };
        if zone.flags & LINKED_CHAT == 0 {
            return false;
        }
        if zone.flags & NO_PVP != 0 {
            return true;
        }
        let template = crate::faction::faction_template_for_race(race);
        let Some(faction) = factions.0.get(&template) else {
            return false;
        };
        faction.friend_group & zone.faction_group != 0
            || faction.enemy_group & zone.faction_group == 0
    }
}

pub(crate) fn load_area_table(conn: &Connection) -> rusqlite::Result<AreaTable> {
    let mut stmt = conn.prepare(
        "SELECT ID, ParentAreaID, ContinentID, Flags_0, FactionGroupMask FROM area_table",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get(0)?,
            AreaEntry {
                parent: row.get(1)?,
                map_id: row.get(2)?,
                flags: row.get(3)?,
                faction_group: row.get(4)?,
            },
        ))
    })?;
    Ok(AreaTable(rows.collect::<rusqlite::Result<_>>()?))
}
