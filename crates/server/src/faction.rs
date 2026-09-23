//! Faction templates and unit reactions (docs/wiki/systems/factions.md).
//!
//! Every unit carries a Retail `FactionTemplate` ID in `UnitFactionTemplate`:
//! NPCs from `creature_template.faction`, players from `ChrRaces.FactionID`.
//! `reaction` is AzerothCore's template-only `Unit::GetFactionReactionTo`
//! check; player reputation (forced ranks, at-war) is not consulted.

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, params};

pub(crate) fn faction_for_race(race: u8) -> u8 {
    match race {
        1 | 3 | 4 | 7 | 11 | 22 | 25 | 29 | 30 | 32 | 34 | 37 => 1,
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 => 2,
        _ => 0,
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
        _ => 0,
    }
}

/// AzerothCore `FACTION_TEMPLATE_FLAG_HATES_ALL_EXCEPT_FRIENDS`.
const HOSTILE_BY_DEFAULT: u32 = 0x2000;

/// One `faction_template` row (DB2 FactionTemplate).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FactionTemplateEntry {
    pub faction: u32,
    pub flags: u32,
    pub faction_group: u32,
    pub friend_group: u32,
    pub enemy_group: u32,
    pub enemies: [u32; 8],
    pub friends: [u32; 8],
}

impl FactionTemplateEntry {
    /// AzerothCore `FactionTemplateEntry::IsFriendlyTo`.
    fn is_friendly_to(&self, other: &Self) -> bool {
        if self.faction == other.faction {
            return true;
        }
        if other.faction != 0 {
            if self.enemies.contains(&other.faction) {
                return false;
            }
            if self.friends.contains(&other.faction) {
                return true;
            }
        }
        self.friend_group & other.faction_group != 0 || self.faction_group & other.friend_group != 0
    }

    /// AzerothCore `FactionTemplateEntry::IsHostileTo`.
    fn is_hostile_to(&self, other: &Self) -> bool {
        if other.faction != 0 {
            if self.enemies.contains(&other.faction) {
                return true;
            }
            if self.friends.contains(&other.faction) {
                return false;
            }
        }
        self.enemy_group & other.faction_group != 0
    }
}

/// How one unit regards another. Unfriendly (a reputation rank) needs the
/// reputation model and is not produced yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reaction {
    Hostile,
    Neutral,
    Friendly,
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
    /// Reaction of a unit with template `a` towards one with template `b`
    /// (template checks of `GetFactionReactionTo`). A missing
    /// or unknown template is neutral.
    pub fn reaction(&self, a: u32, b: u32) -> Reaction {
        let (Some(own), Some(other)) = (self.0.get(&a), self.0.get(&b)) else {
            return Reaction::Neutral;
        };
        if a == b {
            return Reaction::Friendly;
        }
        if own.is_hostile_to(other) {
            Reaction::Hostile
        } else if own.is_friendly_to(other) || other.is_friendly_to(own) {
            Reaction::Friendly
        } else if own.flags & HOSTILE_BY_DEFAULT != 0 {
            Reaction::Hostile
        } else {
            Reaction::Neutral
        }
    }

    /// AzerothCore `_IsValidAttackTarget`, faction part: never a friendly
    /// unit; creature vs creature only when one is hostile to the other.
    /// There is no PvP flag or duel model, so player vs player is rejected.
    pub fn can_attack(&self, a: UnitFaction, b: UnitFaction) -> bool {
        let forward = self.reaction(a.template, b.template);
        let backward = self.reaction(b.template, a.template);
        if forward == Reaction::Friendly || backward == Reaction::Friendly {
            return false;
        }
        match (a.is_player, b.is_player) {
            (true, true) => false,
            (false, false) => forward == Reaction::Hostile || backward == Reaction::Hostile,
            _ => true,
        }
    }

    /// Assist (friendly spell) target: the caster regards the target as
    /// friendly. Stands in for `_IsValidAssistTarget`'s reaction check plus
    /// its PvC rule (creature `type_flags` CAN_ASSIST is not imported).
    pub fn can_assist(&self, a: UnitFaction, b: UnitFaction) -> bool {
        self.reaction(a.template, b.template) == Reaction::Friendly
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
        Ok((column(0)?, entry))
    })?;
    rows.collect::<rusqlite::Result<_>>().map(FactionTemplates)
}

/// Real templates from `data/world.db`; `None` (test skipped) when absent.
#[cfg(test)]
pub(crate) fn world_db_faction_templates() -> Option<FactionTemplates> {
    static TEMPLATES: std::sync::OnceLock<Option<FactionTemplates>> = std::sync::OnceLock::new();
    TEMPLATES
        .get_or_init(|| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
            if !path.exists() {
                eprintln!("skipping: {} not found", path.display());
                return None;
            }
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
