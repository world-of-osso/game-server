//! Retail item attributes from world.db `content_item` (item + item_sparse).
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags, Row};

use crate::item_stats::{ItemScaling, ItemStatAllocation};

/// Item attributes inventory validation and item stats need.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
    /// Stat, armor and weapon inputs (`item_stats::item_stats`).
    pub scaling: ItemScaling,
    /// `ItemAppearance` id equipping or binding the item teaches the account's
    /// appearance collection: its `item_modified_appearance` row with the lowest
    /// `ItemAppearanceModifierID` (Retail's default source). `None` for items
    /// without an appearance (cloth, potions, rings, trinkets).
    pub appearance_id: Option<u32>,
    /// What using the item casts: its first `ITEM_SPELLTRIGGER_ON_USE` effect by
    /// `LegacySlotIndex` (`ItemXItemEffect` -> `ItemEffect`, TriggerType 0).
    pub on_use: Option<OnUseSpell>,
}

/// An item's on-use `ItemEffect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnUseSpell {
    pub spell_id: u32,
    /// Negative: the item is expendable and a cast uses one up (`ItemEffect.Charges`,
    /// `Spell::TakeCastItem`).
    pub charges: i32,
    /// `SpellCategoryID` / `CategoryCoolDownMSec`.
    pub cooldown: crate::spell_cast::ItemCooldown,
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

const ITEM_QUERY: &str = "SELECT c.ID, c.InventoryType, c.Stackable, c.Bonding, c.RequiredLevel, \
     c.AllowableClass, c.AllowableRaces_0, c.AllowableRaces_1, \
     c.ClassID, c.SubclassID, c.Quality, c.ItemLevel, s.ItemSquishEraID, c.ItemDelay, \
     c.DmgVariance, c.Flags_1, \
     c.StatModifier_bonusStat_0, c.StatPercentEditor_0, s.StatPercentageOfSocket_0, \
     c.StatModifier_bonusStat_1, c.StatPercentEditor_1, s.StatPercentageOfSocket_1, \
     c.StatModifier_bonusStat_2, c.StatPercentEditor_2, s.StatPercentageOfSocket_2, \
     c.StatModifier_bonusStat_3, c.StatPercentEditor_3, s.StatPercentageOfSocket_3, \
     c.StatModifier_bonusStat_4, c.StatPercentEditor_4, s.StatPercentageOfSocket_4, \
     c.StatModifier_bonusStat_5, c.StatPercentEditor_5, s.StatPercentageOfSocket_5, \
     c.StatModifier_bonusStat_6, c.StatPercentEditor_6, s.StatPercentageOfSocket_6, \
     c.StatModifier_bonusStat_7, c.StatPercentEditor_7, s.StatPercentageOfSocket_7, \
     c.StatModifier_bonusStat_8, c.StatPercentEditor_8, s.StatPercentageOfSocket_8, \
     c.StatModifier_bonusStat_9, c.StatPercentEditor_9, s.StatPercentageOfSocket_9, \
     (SELECT m.ItemAppearanceID FROM item_modified_appearance m WHERE m.ItemID = c.ID \
      AND m.ItemAppearanceID != 0 ORDER BY m.ItemAppearanceModifierID, m.OrderIndex LIMIT 1), \
     u.SpellID, u.Charges, u.SpellCategoryID, u.CategoryCoolDownMSec \
     FROM content_item c JOIN item_sparse s ON s.ID = c.ID \
     LEFT JOIN (SELECT x.ItemID, e.SpellID, e.Charges, e.SpellCategoryID, e.CategoryCoolDownMSec, \
      ROW_NUMBER() OVER (PARTITION BY x.ItemID ORDER BY e.LegacySlotIndex, e.ID) AS slot \
      FROM item_x_item_effect x JOIN item_effect e ON e.ID = x.ItemEffectID \
      WHERE e.TriggerType = 0 AND e.SpellID > 0) u ON u.ItemID = c.ID AND u.slot = 1";
const FIRST_STAT_COLUMN: usize = 16;
const APPEARANCE_COLUMN: usize = FIRST_STAT_COLUMN + 10 * 3;

fn scaling_from_row(row: &Row) -> rusqlite::Result<ItemScaling> {
    let mut stats = [ItemStatAllocation::default(); 10];
    for (index, stat) in stats.iter_mut().enumerate() {
        let column = FIRST_STAT_COLUMN + index * 3;
        *stat = ItemStatAllocation {
            stat: row.get(column)?,
            allocation: row.get(column + 1)?,
            socket_multiplier: row.get::<_, f64>(column + 2)? as f32,
        };
    }
    Ok(ItemScaling {
        class_id: row.get(8)?,
        subclass_id: row.get(9)?,
        quality: row.get(10)?,
        item_level: row.get(11)?,
        squish_era: row.get(12)?,
        delay_ms: row.get(13)?,
        damage_variance: row.get::<_, f64>(14)? as f32,
        flags2: row.get(15)?,
        stats,
    })
}

fn on_use_from_row(row: &Row) -> rusqlite::Result<Option<OnUseSpell>> {
    let Some(spell_id) = row.get::<_, Option<u32>>(APPEARANCE_COLUMN + 1)? else {
        return Ok(None);
    };
    Ok(Some(OnUseSpell {
        spell_id,
        charges: row.get(APPEARANCE_COLUMN + 2)?,
        cooldown: crate::spell_cast::ItemCooldown {
            category: row.get(APPEARANCE_COLUMN + 3)?,
            category_ms: row.get(APPEARANCE_COLUMN + 4)?,
        },
    }))
}

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
        scaling: scaling_from_row(row)?,
        appearance_id: row.get(APPEARANCE_COLUMN)?,
        on_use: on_use_from_row(row)?,
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
