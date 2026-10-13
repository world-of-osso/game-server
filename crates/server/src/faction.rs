//! Faction templates and unit reactions (docs/wiki/systems/factions.md).
//!
//! Every unit carries a Retail `FactionTemplate` ID in `UnitFactionTemplate`:
//! NPCs from `creature_template.faction`, players from `ChrRaces.FactionID`.
//! The reaction rules themselves live in the pure `faction_reaction` module;
//! this module holds the ID lookups, race table and world.db loader.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, params};
use shared::components::UnitFlags;

pub use crate::faction_reaction::Reaction;
use crate::faction_reaction::{self, FactionTemplateEntry};

pub(crate) fn faction_for_race(race: u8) -> u8 {
    match race {
        1 | 3 | 4 | 7 | 11 | 22 | 25 | 29 | 30 | 32 | 34 | 37 | 52 | 85 | 86 | 95 => 1,
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 | 84 | 91 | 96 => 2,
        _ => 0,
    }
}

/// `FactionTemplate.FactionGroup` of each race's `ChrRaces.FactionID` template
/// (build 12.1.0.69933): 3 (player | Alliance) for Alliance races, 5 (player |
/// Horde) for Horde races, 1 (player) for neutral Pandaren.
pub(crate) fn faction_group_for_race(race: u8) -> u32 {
    match faction_for_race(race) {
        1 => 3,
        2 => 5,
        _ => 1,
    }
}

/// `ChrRaces.FactionID` (a FactionTemplate ID) of each character race,
/// build 12.1.0.69933. 0 = unknown race (no template: neutral to everyone).
pub(crate) fn faction_template_for_race(race: u8) -> u32 {
    match race {
        1 | 32 | 52 => 1,      // Human, Kul Tiran, Dracthyr (Alliance)
        2 | 36 | 70 | 84 => 2, // Orc, Mag'har, Dracthyr (Horde), Earthen (Horde)
        3 | 34 | 85 => 3,      // Dwarf, Dark Iron, Earthen (Alliance)
        4 | 29 | 86 => 4,      // Night Elf, Void Elf, Haranir (Alliance)
        5 => 5,                // Undead
        6 | 28 => 6,           // Tauren, Highmountain
        7 | 37 => 115,         // Gnome, Mechagnome
        8 | 31 | 91 => 116,    // Troll, Zandalari, Haranir (Horde)
        9 | 35 => 2204,        // Goblin, Vulpera
        10 | 27 => 1610,       // Blood Elf, Nightborne
        11 | 30 => 1629,       // Draenei, Lightforged
        22 => 2203,            // Worgen
        24 => 2395,            // Pandaren (neutral)
        25 => 2401,            // Pandaren (Alliance)
        26 => 2402,            // Pandaren (Horde)
        95 => 3629,            // High Order Skyborne (Forever 1.60.1.70205)
        96 => 3628,            // Windshaper Skyborne (Forever 1.60.1.70205)
        _ => 0,
    }
}

/// `UNIT_FLAG_PLAYER_CONTROLLED`: a unit a player controls, such as its pet
/// (`Unit::SetMinion`).
pub(crate) const UNIT_FLAG_PLAYER_CONTROLLED: u32 = 0x0000_0008;

/// `Unit::GetAffectingPlayer` is set: a player, or a unit a player controls. Such a unit
/// takes the player side of `can_attack` (`_IsValidAttackTarget`'s PvC rules).
pub(crate) fn player_affected(is_player: bool, flags: Option<&UnitFlags>) -> bool {
    is_player || flags.is_some_and(|flags| flags.0 & UNIT_FLAG_PLAYER_CONTROLLED != 0)
}

/// Faction identity of one unit for attack/assist checks.
#[derive(Debug, Clone, Copy)]
pub struct UnitFaction {
    pub template: u32,
    pub is_player: bool,
}

/// All `faction_template` rows by ID.
#[derive(Resource, Debug, Default, Clone)]
pub struct FactionTemplates(pub HashMap<u32, FactionTemplateEntry>);

impl FactionTemplates {
    fn unit(&self, unit: UnitFaction) -> faction_reaction::Unit<'_> {
        faction_reaction::Unit {
            template: self.0.get(&unit.template),
            is_player: unit.is_player,
        }
    }

    /// Reaction of a unit with template `a` towards one with template `b`.
    pub fn reaction(&self, a: u32, b: u32) -> Reaction {
        faction_reaction::reaction(self.0.get(&a), self.0.get(&b))
    }

    pub fn can_attack(&self, a: UnitFaction, b: UnitFaction) -> bool {
        faction_reaction::can_attack(self.unit(a), self.unit(b))
    }

    pub fn can_assist(&self, a: UnitFaction, b: UnitFaction) -> bool {
        faction_reaction::can_assist(self.unit(a), self.unit(b))
    }
}

pub fn load_faction_templates(conn: &Connection) -> rusqlite::Result<FactionTemplates> {
    let mut stmt = conn.prepare(
        "SELECT ID, Faction, Flags, FactionGroup, FriendGroup, EnemyGroup,
                Enemies_0, Enemies_1, Enemies_2, Enemies_3,
                Enemies_4, Enemies_5, Enemies_6, Enemies_7,
                Friend_0, Friend_1, Friend_2, Friend_3,
                Friend_4, Friend_5, Friend_6, Friend_7
         FROM faction_template",
    )?;
    let rows = stmt.query_map(params![], |row| {
        let column = |index: usize| row.get::<_, i64>(index).map(|value| value as u32);
        let mut entry = FactionTemplateEntry {
            id: column(0)?,
            faction: column(1)?,
            flags: column(2)?,
            faction_group: column(3)?,
            friend_group: column(4)?,
            enemy_group: column(5)?,
            ..default()
        };
        for slot in 0..8 {
            entry.enemies[slot] = column(6 + slot)?;
            entry.friends[slot] = column(14 + slot)?;
        }
        Ok((entry.id, entry))
    })?;
    rows.collect::<rusqlite::Result<_>>().map(FactionTemplates)
}

/// Real templates from `data/world.db`.
#[cfg(test)]
pub(crate) fn world_db_faction_templates() -> Option<FactionTemplates> {
    static TEMPLATES: std::sync::OnceLock<Option<FactionTemplates>> = std::sync::OnceLock::new();
    TEMPLATES
        .get_or_init(|| {
            let path = crate::test_data::world_db_path();
            let conn =
                Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
            Some(load_faction_templates(&conn).unwrap())
        })
        .clone()
}

#[cfg(test)]
#[path = "faction_tests.rs"]
mod tests;
