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
/// TC TerrainMgr.cpp650–656 only replaces flagged subzones with their immediate parent.
const IS_SUBZONE: u32 = 0x4000_0000;

#[derive(Debug, Clone)]
struct AreaEntry {
    parent: u32,
    map_id: u32,
    flags: u32,
    flags2: u32,
    faction_group: u32,
    name: String,
}

#[derive(Resource, Default)]
pub(crate) struct AreaTable(HashMap<u32, AreaEntry>);

impl AreaTable {
    /// TC DB2Manager::IsInArea: the area itself or any ParentAreaID ancestor.
    pub(crate) fn is_in_area(&self, mut area_id: u32, required: u32) -> bool {
        for _ in 0..=self.0.len() {
            if area_id == required {
                return true;
            }
            let Some(area) = self.0.get(&area_id) else {
                return false;
            };
            if area.parent == 0 {
                return false;
            }
            area_id = area.parent;
        }
        false
    }

    /// TC TerrainInfo::GetZoneId: one flagged parent, never a recursive ancestry walk.
    pub(crate) fn zone_id(&self, area_id: u32, map_id: u32) -> Option<u32> {
        let area = self.0.get(&area_id)?;
        if area.map_id != map_id {
            return None;
        }
        let is_subzone = area.flags & IS_SUBZONE != 0 && area.parent != 0;
        Some(if is_subzone { area.parent } else { area_id })
    }

    pub(crate) fn zone_name(&self, area_id: u32, map_id: u32) -> Option<&str> {
        let zone_id = self.zone_id(area_id, map_id)?;
        let name = &self.0.get(&zone_id)?.name;
        (!name.is_empty()).then_some(name.as_str())
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
        let Some(zone) = self.0.get(&zone_id) else {
            return false; // An unavailable parent row cannot establish friendly territory.
        };
        if faction.friend_group & zone.faction_group == 0 {
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
        let Some(zone) = self.0.get(&zone_id) else {
            return false; // An unavailable parent row cannot establish city rest.
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
        "SELECT ID, ParentAreaID, ContinentID, Flags_0, FactionGroupMask, Flags_1, AreaName_lang FROM area_table",
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
                name: row.get(6)?,
            },
        ))
    })?;
    Ok(AreaTable(rows.collect::<rusqlite::Result<_>>()?))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::load_area_table;

    #[test]
    fn toyfx21_source_ancestry_does_not_infer_membership_from_continent_or_require_parent_rows() {
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        let areas = load_area_table(&conn).unwrap();
        // TC DB2Stores.cpp1985: IDs and parent links, without a continent test.
        assert!(areas.is_in_area(6797, 6719));
        // Genuine69933 parent16311 has no row; matching that ID precedes lookup.
        assert!(areas.is_in_area(16328, 16311));
        assert!(!areas.is_in_area(16328, 6719));
    }

    #[test]
    fn achievements_criteria_top_level_zone_uses_subzone_flag_not_recursive_parent() {
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        let areas = load_area_table(&conn).unwrap();
        // TC TerrainMgr.cpp650–656: one flagged parent, otherwise this area itself.
        assert_eq!(areas.zone_id(13862, 2444), Some(13862));
        assert_eq!(areas.zone_id(14489, 2444), Some(13862));
        assert_eq!(areas.zone_id(87, 0), Some(12));
        assert_eq!(areas.zone_id(13862, 0), None);
    }

    #[test]
    fn achievements_criteria_top_level_zone_missing_parent_does_not_infer_territory() {
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        let areas = load_area_table(&conn).unwrap();
        let factions = crate::faction::load_faction_templates(&conn).unwrap();
        // Authenticated69933 has this subzone, but no16311 parent row. Keep its ID, not flags.
        assert_eq!(areas.zone_id(16328, 2907), Some(16311));
        assert!(!areas.permits_city_rest(16328, 2907, 1, &factions));
        assert!(!areas.permits_war_mode_enable(16328, 2907, 1, &factions));
    }

    #[test]
    fn guild_online_zone_names_use_loaded_parent_rows_without_map_or_missing_parent_fallback() {
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        let mut areas = load_area_table(&conn).unwrap();
        assert_eq!(areas.zone_name(12, 0), Some("Elwynn Forest"));
        assert_eq!(areas.zone_name(87, 0), Some("Elwynn Forest"));
        assert_eq!(areas.zone_name(40, 0), Some("Westfall"));
        assert_eq!(areas.zone_name(87, 1), None);
        assert_eq!(areas.zone_name(u32::MAX, 0), None);
        areas.0.remove(&12);
        assert_eq!(areas.zone_name(87, 0), None);
    }

    /// AreaTable flags with the top bit set are stored signed (e.g. -1610530816 = 0xA0014000);
    /// the loader keeps the bit pattern instead of failing at startup.
    #[test]
    fn signed_db2_flags_load_as_their_bit_pattern() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE area_table (ID INTEGER, ParentAreaID INTEGER, ContinentID INTEGER,
                                      Flags_0 INTEGER, FactionGroupMask INTEGER, Flags_1 INTEGER,
                                      AreaName_lang TEXT);
             INSERT INTO area_table VALUES (1519, 0, 0, -1610530816, 2, 0, 'Stormwind City');",
        )
        .unwrap();
        let table = load_area_table(&conn).unwrap();
        assert_eq!(table.0[&1519].flags, (-1_610_530_816_i32) as u32);
    }
}
