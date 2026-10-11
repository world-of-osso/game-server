//! AreaTable DB2 territory metadata used by city rest.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::Connection;

use crate::faction::FactionTemplates;

/// TC a352b1fa DBCEnums.h:113, formerly AREA_FLAG_CAPITAL.
const LINKED_CHAT: u32 = 0x100;
/// AreaFlags::NoPvP, AreaTableEntry::IsSanctuary.
const NO_PVP: u32 = 0x800;
/// TC a352b1fa DBCEnums.h AreaFlags2::AllowWarModeToggle.
const ALLOW_WAR_MODE_TOGGLE: u32 = 0x1000;

#[derive(Debug, Clone)]
struct AreaEntry {
    parent: u32,
    map_id: u32,
    flags: u32,
    flags2: u32,
    faction_group: u32,
}

#[derive(Resource, Default)]
pub(crate) struct AreaTable(HashMap<u32, AreaEntry>);

impl AreaTable {
    /// TC TerrainInfo::GetZoneId follows ParentAreaID to the top-level zone.
    pub(crate) fn zone_id(&self, mut area_id: u32, map_id: u32) -> Option<u32> {
        loop {
            let area = self.0.get(&area_id)?;
            if area.map_id != map_id {
                return None;
            }
            if area.parent == 0 {
                return Some(area_id);
            }
            area_id = area.parent;
        }
    }

    /// Player::CanEnableWarModeInArea: friendly zone, then any ancestor's toggle bit.
    pub(crate) fn permits_war_mode_enable(
        &self,
        mut area_id: u32,
        map_id: u32,
        race: u8,
        factions: &FactionTemplates,
    ) -> bool {
        let Some(zone_id) = self.zone_id(area_id, map_id) else {
            return false;
        };
        let template = crate::faction::faction_template_for_race(race);
        let Some(faction) = factions.0.get(&template) else {
            return false;
        };
        if faction.friend_group & self.0[&zone_id].faction_group == 0 {
            return false;
        }
        loop {
            let area = &self.0[&area_id];
            if area.flags2 & ALLOW_WAR_MODE_TOGGLE != 0 {
                return true;
            }
            if area.parent == 0 {
                return false;
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
        let Some(zone_id) = self.zone_id(area_id, map_id) else {
            return false;
        };
        let zone = &self.0[&zone_id];
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
        "SELECT ID, ParentAreaID, ContinentID, Flags_0, FactionGroupMask, Flags_1 FROM area_table",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get(0)?,
            AreaEntry {
                parent: row.get(1)?,
                map_id: row.get(2)?,
                // DB2 bitfield stored signed (top bits set): keep the 32-bit pattern.
                flags: row.get::<_, i64>(3)? as u32,
                faction_group: row.get(4)?,
                flags2: row.get::<_, i64>(5)? as u32,
            },
        ))
    })?;
    Ok(AreaTable(rows.collect::<rusqlite::Result<_>>()?))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::load_area_table;

    /// AreaTable flags with the top bit set are stored signed (e.g. -1610530816 = 0xA0014000);
    /// the loader keeps the bit pattern instead of failing at startup.
    #[test]
    fn signed_db2_flags_load_as_their_bit_pattern() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE area_table (ID INTEGER, ParentAreaID INTEGER, ContinentID INTEGER,
                                      Flags_0 INTEGER, FactionGroupMask INTEGER, Flags_1 INTEGER);
             INSERT INTO area_table VALUES (1519, 0, 0, -1610530816, 2, 0);",
        )
        .unwrap();
        let table = load_area_table(&conn).unwrap();
        assert_eq!(table.0[&1519].flags, (-1_610_530_816_i32) as u32);
    }
}
