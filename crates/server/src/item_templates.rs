//! Retail item attributes from world.db `content_item` (item + item_sparse).
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags, Row};

use crate::item_stats::{ItemScaling, ItemStatAllocation};
#[path = "item_templates/loot_need.rs"]
mod loot_need;
pub use loot_need::LootNeedMetadata;
mod equip_limit;
pub use equip_limit::EquipLimit;

/// Item attributes inventory validation and item stats need.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ItemTemplate {
    /// Retail `Enum.InventoryType`; 0 = not equippable.
    pub inventory_type: u8,
    /// `Stackable`: maximum stack size.
    pub max_stack: u32,
    /// `MaxCount`: maximum owned quantity; non-positive means unlimited.
    pub max_count: i32,
    /// `Bonding`: 0 none, 1 on pickup, 2 on equip, 3 on use, 4 quest.
    pub bonding: u8,
    pub required_level: u16,
    /// One-based class bits; -1 = every class.
    pub allowable_class: i64,
    /// `ChrRaces.PlayableRaceBit` bits (`AllowableRaces_1` high word, `_0` low word).
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
    /// `ContainerSlots`: bag capacity, 0 for non-containers.
    pub container_slots: u8,
    /// `BagFamily`: `BAG_FAMILY_MASK_*` bits a profession bag accepts.
    pub bag_family: u32,
    /// ItemSparse.Flags_0: ITEM_FLAG_UNIQUE_EQUIPPABLE (TrinityCore ItemTemplate.h).
    pub unique_equipped: bool,
    /// ItemSparse.LimitCategory and its authored ItemLimitCategory.Quantity.
    pub equip_limit: Option<EquipLimit>,
}

impl ItemTemplate {
    /// CRAFTING_ORIGIN_NOT_RECORDED_ON_STACKABLE_ITEMS: stackable commodities carry no
    /// crafter identity, so only max-stack-1 items keep a crafting origin.
    pub fn records_crafting_origin(&self) -> bool {
        self.max_stack <= 1
    }
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

/// TrinityCore a352b1fa ITEM_FLAG_UNIQUE_EQUIPPABLE.
const ITEM_FLAG_UNIQUE_EQUIPPABLE: i64 = 0x0008_0000;
const ITEM_CLASS_ARMOR: u8 = 4;
const ITEM_SUBCLASS_COSMETIC: u8 = 5;
const ITEM_CLASS_CONTAINER: u8 = 1;
const ITEM_SUBCLASS_REAGENT_CONTAINER: u8 = 11;
/// `ITEM_FLAG2_USED_IN_A_TRADESKILL` (ItemTemplate.h:257), `IsCraftingReagent`.
const ITEM_FLAG2_USED_IN_A_TRADESKILL: i64 = 0x8000_0000;
/// `BAG_FAMILY_MASK_*` a profession container subclass requires (ItemTemplate.h:336-355).
const CONTAINER_FAMILIES: [(u8, u32); 10] = [
    (1, 0x4),      // soul: soul shards
    (2, 0x20),     // herb
    (3, 0x40),     // enchanting
    (4, 0x80),     // engineering
    (5, 0x200),    // gem
    (6, 0x400),    // mining
    (7, 0x8),      // leatherworking
    (8, 0x10),     // inscription
    (9, 0x8000),   // tackle: fishing
    (10, 0x10000), // cooking
];

impl ItemTemplate {
    pub fn allows(&self, class: u8, race: u8) -> bool {
        let class_ok = self.allowable_class == -1
            || one_based_bit(class).is_some_and(|bit| self.allowable_class as u64 & bit != 0);
        // TrinityCore `Player::CanUseItem` (Player.cpp:11221): `GetAllowableRace().HasRace(race)`.
        let race_ok = crate::race_class::race_bit(race)
            .is_some_and(|bit| self.allowable_race & (1 << bit) != 0);
        class_ok && race_ok
    }

    pub fn is_cosmetic(&self) -> bool {
        self.scaling.class_id == ITEM_CLASS_ARMOR
            && self.scaling.subclass_id == ITEM_SUBCLASS_COSMETIC
    }

    pub fn is_reagent_bag(&self) -> bool {
        self.scaling.class_id == ITEM_CLASS_CONTAINER
            && self.scaling.subclass_id == ITEM_SUBCLASS_REAGENT_CONTAINER
    }

    /// TrinityCore `ItemCanGoIntoBag` (Item.cpp:250-322) for containers: a plain bag
    /// takes anything, a reagent bag crafting reagents, a profession bag items of its
    /// bag family.
    pub fn can_go_into_bag(&self, bag: &ItemTemplate) -> bool {
        if bag.scaling.class_id != ITEM_CLASS_CONTAINER {
            return false;
        }
        match bag.scaling.subclass_id {
            0 => true,
            ITEM_SUBCLASS_REAGENT_CONTAINER => {
                self.scaling.flags2 & ITEM_FLAG2_USED_IN_A_TRADESKILL != 0
            }
            subclass => CONTAINER_FAMILIES
                .iter()
                .find(|(family_subclass, _)| *family_subclass == subclass)
                .is_some_and(|(_, mask)| self.bag_family & mask != 0),
        }
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
pub struct ItemTemplates {
    pub retail: HashMap<u32, ItemTemplate>,
    pub forever: HashMap<u32, ItemTemplate>,
    pub loot_need: LootNeedMetadata,
    /// Retail ItemEffect TriggerType6 (ON_LEARN), keyed by the owning item.
    pub(crate) retail_learning_spells: HashMap<u32, Vec<u32>>,
}

impl ItemTemplates {
    pub fn get(
        &self,
        source: shared::item_data::ItemDefinitionSource,
        item_id: u32,
    ) -> Option<&ItemTemplate> {
        match source {
            shared::item_data::ItemDefinitionSource::Retail => self.retail.get(&item_id),
            shared::item_data::ItemDefinitionSource::Forever70205 => self.forever.get(&item_id),
        }
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
     u.SpellID, u.Charges, u.SpellCategoryID, u.CategoryCoolDownMSec, \
     s.ContainerSlots, s.BagFamily, s.Flags_0, s.LimitCategory, c.MaxCount \
     FROM content_item c JOIN item_sparse s ON s.ID = c.ID \
     LEFT JOIN (SELECT x.ItemID, e.SpellID, e.Charges, e.SpellCategoryID, e.CategoryCoolDownMSec, \
      ROW_NUMBER() OVER (PARTITION BY x.ItemID ORDER BY CASE WHEN e.TriggerType = 6 THEN 0 ELSE 1 END, e.LegacySlotIndex, e.ID) AS slot \
      FROM item_x_item_effect x JOIN item_effect e ON e.ID = x.ItemEffectID \
      WHERE e.SpellID > 0 AND (e.TriggerType = 0 OR (e.TriggerType = 6 AND EXISTS \
      (SELECT 1 FROM spell_effect pet WHERE pet.SpellID = e.SpellID AND pet.Effect = 28 \
       AND pet.EffectMiscValue_1 = 3221) OR EXISTS \
      (SELECT 1 FROM mount m WHERE m.SourceSpellID = e.SpellID)))) u ON u.ItemID = c.ID AND u.slot = 1";
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
            // -1 keeps the spell's own category cooldown (`SpellHistory::GetCooldownDurations`
            // takes the item's only when >= 0), which the cast starts anyway.
            category_ms: row.get::<_, i32>(APPEARANCE_COLUMN + 4)?.max(0) as u32,
        },
    }))
}

fn template_from_row(row: &Row) -> rusqlite::Result<(u32, ItemTemplate)> {
    let low: i64 = row.get(6)?;
    let high: i64 = row.get(7)?;
    let scaling = scaling_from_row(row)?;
    // Retail ITEM_CLASS_MISCELLANEOUS / ITEM_SUBCLASS_MISCELLANEOUS_MOUNT.
    // Mount items are collection unlocks, never appearance-bearing equipment.
    let appearance_id = if scaling.class_id == 15 && scaling.subclass_id == 5 {
        None
    } else {
        row.get(APPEARANCE_COLUMN)?
    };
    let template = ItemTemplate {
        inventory_type: row.get(1)?,
        max_stack: row.get::<_, i64>(2)?.clamp(1, i64::from(u32::MAX)) as u32,
        max_count: row.get(APPEARANCE_COLUMN + 9)?,
        bonding: row.get(3)?,
        required_level: row.get(4)?,
        allowable_class: row.get(5)?,
        allowable_race: (u64::from(high as u32) << 32) | u64::from(low as u32),
        scaling,
        appearance_id,
        on_use: on_use_from_row(row)?,
        container_slots: row.get(APPEARANCE_COLUMN + 5)?,
        bag_family: row.get(APPEARANCE_COLUMN + 6)?,
        unique_equipped: row.get::<_, i64>(APPEARANCE_COLUMN + 7)? & ITEM_FLAG_UNIQUE_EQUIPPABLE
            != 0,
        equip_limit: equip_limit::limit_from_category(row.get(APPEARANCE_COLUMN + 8)?)?,
    };
    Ok((row.get(0)?, template))
}

pub fn load_item_templates(path: &Path) -> rusqlite::Result<ItemTemplates> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    Ok(ItemTemplates {
        retail: load_item_template_rows(&conn, ITEM_QUERY)?,
        forever: HashMap::new(),
        loot_need: loot_need::load(&conn)?,
        retail_learning_spells: load_item_learning_spells(&conn)?,
    })
}

fn load_item_learning_spells(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<u32>>> {
    let mut query = conn.prepare("SELECT DISTINCT x.ItemID, e.SpellID FROM item_x_item_effect x JOIN item_effect e ON e.ID=x.ItemEffectID JOIN spell s ON s.ID=e.SpellID WHERE e.TriggerType=6 AND e.SpellID>0 ORDER BY x.ItemID, e.SpellID")?;
    let mut spells: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in query.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))? {
        let (item, spell) = row?;
        spells.entry(item).or_default().push(spell);
    }
    Ok(spells)
}

/// Production catalog: both namespaces, no substitution.
pub fn load_runtime_item_templates(path: &Path) -> rusqlite::Result<ItemTemplates> {
    let mut templates = load_item_templates(path)?;
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    templates.forever = load_skyborne_item_templates(&conn)?;
    Ok(templates)
}

pub(crate) fn load_skyborne_item_templates(
    conn: &Connection,
) -> rusqlite::Result<HashMap<u32, ItemTemplate>> {
    let query = ITEM_QUERY
        .replace("content_item", "skyborne_content_item")
        .replace("item_sparse", "skyborne_item_sparse")
        .replace(
            "item_modified_appearance",
            "skyborne_item_modified_appearance",
        )
        .replace("item_x_item_effect", "skyborne_item_x_item_effect")
        .replace("JOIN item_effect", "JOIN skyborne_item_effect");
    load_item_template_rows(conn, &query)
}

fn load_item_template_rows(
    conn: &Connection,
    query: &str,
) -> rusqlite::Result<HashMap<u32, ItemTemplate>> {
    let mut statement = conn.prepare(query)?;
    let rows = statement.query_map([], template_from_row)?;
    rows.collect()
}

pub struct ItemTemplatesPlugin;

impl Plugin for ItemTemplatesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_item_templates);
    }
}

fn startup_load_item_templates(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let templates = load_runtime_item_templates(path)
        .unwrap_or_else(|error| panic!("load content_item from {}: {error}", path.display()));
    info!(
        "Loaded {} Retail and {} Forever70205 item templates",
        templates.retail.len(),
        templates.forever.len()
    );
    commands.insert_resource(templates);
}

#[cfg(test)]
#[path = "item_templates_tests.rs"]
mod tests;
