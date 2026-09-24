//! Retail item attributes from world.db `content_item` (item + item_sparse).
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags, Row};

/// Item attributes inventory validation needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemTemplate {
    /// Retail `Enum.InventoryType`; 0 = not equippable.
    pub inventory_type: u8,
    /// `Stackable`: maximum stack size.
    pub max_stack: u32,
    /// `Bonding`: 0 none, 1 on pickup, 2 on equip, 3 on use, 4 quest.
    pub bonding: u8,
    pub required_level: u16,
    /// One-based class bits; -1 = every class.
    pub allowable_class: i64,
    /// One-based race bits (`AllowableRaces_1` high word, `_0` low word).
    pub allowable_race: u64,
}

pub const BIND_ON_PICKUP: u8 = 1;
pub const BIND_ON_EQUIP: u8 = 2;
pub const BIND_QUEST: u8 = 4;

impl ItemTemplate {
    pub fn allows(&self, class: u8, race: u8) -> bool {
        let class_ok = self.allowable_class == -1
            || one_based_bit(class).is_some_and(|bit| self.allowable_class as u64 & bit != 0);
        let race_ok = one_based_bit(race).is_some_and(|bit| self.allowable_race & bit != 0);
        class_ok && race_ok
    }

    /// Soulbound as soon as it is owned.
    pub fn binds_on_pickup(&self) -> bool {
        matches!(self.bonding, BIND_ON_PICKUP | BIND_QUEST)
    }
}

fn one_based_bit(id: u8) -> Option<u64> {
    id.checked_sub(1)
        .and_then(|shift| 1u64.checked_shl(u32::from(shift)))
}

#[derive(Resource, Debug, Default, Clone)]
pub struct ItemTemplates(pub HashMap<u32, ItemTemplate>);

impl ItemTemplates {
    pub fn get(&self, item_id: u32) -> Option<&ItemTemplate> {
        self.0.get(&item_id)
    }
}

const ITEM_QUERY: &str = "SELECT ID, InventoryType, Stackable, Bonding, RequiredLevel, \
     AllowableClass, AllowableRaces_0, AllowableRaces_1 FROM content_item";

fn template_from_row(row: &Row) -> rusqlite::Result<(u32, ItemTemplate)> {
    let low: i64 = row.get(6)?;
    let high: i64 = row.get(7)?;
    let template = ItemTemplate {
        inventory_type: row.get(1)?,
        max_stack: row.get::<_, i64>(2)?.clamp(1, i64::from(u32::MAX)) as u32,
        bonding: row.get(3)?,
        required_level: row.get(4)?,
        allowable_class: row.get(5)?,
        allowable_race: (u64::from(high as u32) << 32) | u64::from(low as u32),
    };
    Ok((row.get(0)?, template))
}

pub fn load_item_templates(path: &Path) -> rusqlite::Result<ItemTemplates> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = conn.prepare(ITEM_QUERY)?;
    let rows = statement.query_map([], template_from_row)?;
    Ok(ItemTemplates(rows.collect::<rusqlite::Result<_>>()?))
}

pub struct ItemTemplatesPlugin;

impl Plugin for ItemTemplatesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_item_templates);
    }
}

fn startup_load_item_templates(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let templates = load_item_templates(path)
        .unwrap_or_else(|error| panic!("load content_item from {}: {error}", path.display()));
    info!("Loaded {} item templates", templates.0.len());
    commands.insert_resource(templates);
}

#[cfg(test)]
#[path = "item_templates_tests.rs"]
mod tests;
