//! New-character kit: the items and action buttons TrinityCore `ObjectMgr::LoadPlayerInfo`
//! gives each race/class (ObjectMgr.cpp:3938-4062, 4205-4247) and `Player::Create`
//! hands out (Player.cpp:509-554). Rules: `docs/wiki/systems/new-character-kit.md`.
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags};

use crate::inventory::inventory_model::InventoryModel;
use crate::item_templates::ItemTemplates;
use crate::merchant::VendorCatalog;
use crate::persistence::inventory_store::InventoryRecord;
use crate::persistence::{CharacterData, InventoryItemData};
use crate::persistence_progression::{ActionRefData, ProgressionData};
use crate::race_class::{playable_pairs, race_mask_has_race};

/// `CharacterLoadoutEntry::IsForNewCharacter` (DB2Structure.h:664).
const NEW_CHARACTER_LOADOUT: u32 = 9;
/// `ITEM_CLASS_CONSUMABLE` / `ITEM_SUBCLASS_FOOD_DRINK` (ItemTemplate.h:486).
const ITEM_CLASS_CONSUMABLE: u8 = 0;
const ITEM_SUBCLASS_FOOD_DRINK: u8 = 5;
/// `SPELL_CATEGORY_FOOD` / `SPELL_CATEGORY_DRINK` (SharedDefines.h:396-397).
const SPELL_CATEGORY_FOOD: u32 = 11;
const SPELL_CATEGORY_DRINK: u32 = 59;
const CLASS_DEATH_KNIGHT: u8 = 6;
/// `ACTION_BUTTON_SPELL` / `ACTION_BUTTON_ITEM` (Player.h:372-374).
const ACTION_BUTTON_SPELL: u32 = 0;
const ACTION_BUTTON_ITEM: u32 = 0x80;

/// One race/class's kit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerCreateInfo {
    /// `(item_id, count)` in TrinityCore order: loadout items, then overrides.
    pub items: Vec<(u32, u32)>,
    /// `(button, action)` ascending by button.
    pub actions: Vec<(u8, ActionRefData)>,
}

/// Kits of every playable race/class (`race_class::playable_pairs`).
#[derive(Resource, Debug, Default)]
pub struct PlayerCreateInfos(pub HashMap<(u8, u8), PlayerCreateInfo>);

impl PlayerCreateInfos {
    /// Test fixture: every playable pair with an empty kit.
    #[cfg(test)]
    pub fn bare() -> Self {
        Self(
            playable_pairs()
                .map(|pair| (pair, PlayerCreateInfo::default()))
                .collect(),
        )
    }

    pub fn get(&self, race: u8, class: u8) -> Option<&PlayerCreateInfo> {
        self.0.get(&(race, class))
    }
}

/// A Purpose 9 `CharacterLoadout` item with the item fields its count needs.
struct LoadoutItem {
    class_id: u8,
    race_masks: [u32; 2],
    item_id: u32,
    buy_count: u32,
    max_stack: u32,
    item_class: u8,
    item_subclass: u8,
    /// `SpellCategoryID` of the item's first effect (lowest `LegacySlotIndex`).
    effect_category: Option<u32>,
}

impl LoadoutItem {
    /// ObjectMgr.cpp:3966-3984: `BuyCount`, except food (Death Knight 10, others 4) and
    /// drink (2), capped at the stack size.
    fn count(&self) -> u32 {
        if self.item_class != ITEM_CLASS_CONSUMABLE
            || self.item_subclass != ITEM_SUBCLASS_FOOD_DRINK
        {
            return self.buy_count;
        }
        let count = match self.effect_category {
            Some(SPELL_CATEGORY_FOOD) if self.class_id == CLASS_DEATH_KNIGHT => 10,
            Some(SPELL_CATEGORY_FOOD) => 4,
            Some(SPELL_CATEGORY_DRINK) => 2,
            _ => self.buy_count,
        };
        count.min(self.max_stack)
    }
}

const LOADOUT_QUERY: &str = "SELECT l.ChrClassID, l.RaceMasks_0, l.RaceMasks_1, li.ItemID, \
     c.VendorStackCount, c.Stackable, c.ClassID, c.SubclassID, \
     (SELECT e.SpellCategoryID FROM item_x_item_effect x JOIN item_effect e ON e.ID = x.ItemEffectID \
      WHERE x.ItemID = li.ItemID ORDER BY e.LegacySlotIndex LIMIT 1) \
     FROM character_loadout l JOIN character_loadout_item li ON li.CharacterLoadoutID = l.ID \
     JOIN content_item c ON c.ID = li.ItemID \
     WHERE l.Purpose = ?1 ORDER BY l.ID, li.ID";

/// ObjectMgr.cpp:3938-3994: items of every `CharacterLoadout` with Purpose 9, for
/// each race in its `RaceMask` (`RaceMask::HasRace`). Items without a `content_item`
/// row (TrinityCore: no item template) are skipped.
fn load_loadout_items(conn: &Connection) -> rusqlite::Result<Vec<LoadoutItem>> {
    let mut statement = conn.prepare(LOADOUT_QUERY)?;
    let rows = statement.query_map([NEW_CHARACTER_LOADOUT], |row| {
        let amount = |column| -> rusqlite::Result<u32> {
            Ok(row.get::<_, i64>(column)?.clamp(1, i64::from(u32::MAX)) as u32)
        };
        Ok(LoadoutItem {
            class_id: row.get(0)?,
            race_masks: [row.get::<_, i64>(1)? as u32, row.get::<_, i64>(2)? as u32],
            item_id: row.get(3)?,
            buy_count: amount(4)?,
            max_stack: amount(5)?,
            item_class: row.get(6)?,
            item_subclass: row.get(7)?,
            effect_category: row.get(8)?,
        })
    })?;
    rows.collect()
}

/// `playercreateinfo_item` rows: race and class 0 mean every race or class
/// (ObjectMgr.cpp:3996-4062). A positive amount adds the item; -1 removes it
/// (`PlayerCreateInfoAddItemHelper`, ObjectMgr.cpp:3788-3812); 0 is ignored.
fn apply_item_overrides(
    conn: &Connection,
    infos: &mut HashMap<(u8, u8), PlayerCreateInfo>,
) -> rusqlite::Result<()> {
    let mut statement = conn.prepare(
        "SELECT race, class, itemid, amount FROM tdb_playercreateinfo_item \
         WHERE itemid IN (SELECT ID FROM content_item) ORDER BY race, class, itemid",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u8>(0)?,
            row.get::<_, u8>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (race, class, item_id, amount) = row?;
        for ((pair_race, pair_class), info) in infos.iter_mut() {
            if (race != 0 && race != *pair_race) || (class != 0 && class != *pair_class) {
                continue;
            }
            match amount {
                0 => {}
                1.. => info.items.push((item_id, amount as u32)),
                _ => info.items.retain(|&(id, _)| id != item_id),
            }
        }
    }
    Ok(())
}

/// `playercreateinfo_action` (ObjectMgr.cpp:4205-4247) validated like
/// `Player::IsActionButtonDataValid` (Player.cpp:6047-6080): the button is below
/// `ACTION_BAR_SLOTS`, a spell button's spell and an item button's item exist.
fn load_actions(conn: &Connection) -> rusqlite::Result<Vec<(u8, u8, u8, ActionRefData)>> {
    let mut statement = conn.prepare(
        "SELECT race, class, button, action, type, \
         EXISTS (SELECT 1 FROM spell_name WHERE ID = action), \
         EXISTS (SELECT 1 FROM content_item WHERE ID = action) \
         FROM tdb_playercreateinfo_action ORDER BY race, class, button",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u8>(0)?,
            row.get::<_, u8>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, u32>(3)?,
            row.get::<_, u32>(4)?,
            row.get::<_, bool>(5)?,
            row.get::<_, bool>(6)?,
        ))
    })?;
    let mut actions = Vec::new();
    for row in rows {
        let (race, class, button, action, kind, spell_exists, item_exists) = row?;
        let valid = match kind {
            ACTION_BUTTON_SPELL if spell_exists => Some(ActionRefData::Spell(action)),
            ACTION_BUTTON_ITEM if item_exists => Some(ActionRefData::Item(action)),
            _ => None,
        };
        match (valid, u8::try_from(button)) {
            (Some(action), Ok(slot)) if slot < crate::class_progression::ACTION_BAR_SLOTS => {
                actions.push((race, class, slot, action));
            }
            _ => warn!(
                "playercreateinfo_action race {race} class {class} button {button}: \
                 action {action} type {kind} is not a valid button"
            ),
        }
    }
    Ok(actions)
}

pub fn load_player_create_infos(conn: &Connection) -> rusqlite::Result<PlayerCreateInfos> {
    let mut infos: HashMap<(u8, u8), PlayerCreateInfo> = playable_pairs()
        .map(|pair| (pair, PlayerCreateInfo::default()))
        .collect();
    for item in load_loadout_items(conn)? {
        for ((race, class), info) in infos.iter_mut() {
            if *class == item.class_id && race_mask_has_race(item.race_masks, *race) {
                info.items.push((item.item_id, item.count()));
            }
        }
    }
    apply_item_overrides(conn, &mut infos)?;
    for (race, class, slot, action) in load_actions(conn)? {
        if let Some(info) = infos.get_mut(&(race, class)) {
            info.actions.push((slot, action));
        }
    }
    Ok(PlayerCreateInfos(infos))
}

/// Test fixture owning a kit: empty (`bare`) or the real data/world.db one.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct OwnedKit {
    pub(crate) templates: ItemTemplates,
    pub(crate) catalog: VendorCatalog,
    pub(crate) create_info: PlayerCreateInfos,
}

#[cfg(test)]
impl OwnedKit {
    pub(crate) fn bare() -> Self {
        Self {
            create_info: PlayerCreateInfos::bare(),
            ..Default::default()
        }
    }

    /// The data/world.db kit; `None` (test skipped) when the DB is absent.
    pub(crate) fn from_world_db() -> Option<Self> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
        if !path.exists() {
            eprintln!("skipping: {} not found", path.display());
            return None;
        }
        let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        Some(Self {
            create_info: load_player_create_infos(&conn).expect("player create info"),
            templates: crate::item_templates::load_item_templates(&path).expect("item templates"),
            catalog: crate::merchant::load_vendor_catalog(&conn).expect("vendor catalog"),
        })
    }

    pub(crate) fn kit(&self) -> NewCharacterKit<'_> {
        NewCharacterKit {
            create_info: &self.create_info,
            templates: &self.templates,
            catalog: &self.catalog,
        }
    }
}

/// What character creation reads besides the database.
pub struct NewCharacterKit<'a> {
    pub create_info: &'a PlayerCreateInfos,
    pub templates: &'a ItemTemplates,
    pub catalog: &'a VendorCatalog,
}

impl NewCharacterKit<'_> {
    /// `Player::Create` items (Player.cpp:521-554): each kit item through
    /// `StoreNewItemInBestSlots`, then the backpack second pass. An item that does not
    /// fit is logged and skipped, like TrinityCore.
    pub(crate) fn starting_inventory(
        &self,
        character: CharacterData,
        mint: &mut impl FnMut() -> u64,
    ) -> Result<InventoryRecord, String> {
        let (race, class) = (character.race, character.class);
        let info = self.info(race, class)?;
        let empty = InventoryRecord {
            character,
            inventory: Default::default(),
            placements: Vec::new(),
            equipped: Vec::new(),
            instances: HashMap::new(),
        };
        let mut model = InventoryModel::from_record(empty, self.templates, mint);
        for &(item_id, count) in &info.items {
            let (item, template) = self.new_item(item_id, count, mint())?;
            if let Err(error) = model.store_new_item_in_best_slots(item, template, mint) {
                error!(
                    "race {race} class {class} cannot equip or store kit item {item_id}: {error:?}"
                );
            }
        }
        if let Err(error) = model.equip_backpack_items() {
            error!("race {race} class {class} kit second pass: {error:?}");
        }
        Ok(model.into_record())
    }

    /// `Player::Create` action bar (Player.cpp:509-511) for the Initial spec.
    pub(crate) fn starting_progression(
        &self,
        race: u8,
        class: u8,
    ) -> Result<ProgressionData, String> {
        Ok(ProgressionData {
            spec_id: 0,
            action_bar: self.info(race, class)?.actions.clone(),
        })
    }

    fn info(&self, race: u8, class: u8) -> Result<&PlayerCreateInfo, String> {
        self.create_info
            .get(race, class)
            .ok_or_else(|| format!("race {race} class {class} has no player create info"))
    }

    fn new_item(
        &self,
        item_id: u32,
        count: u32,
        item_guid: u64,
    ) -> Result<(InventoryItemData, crate::item_templates::ItemTemplate), String> {
        let template = *self
            .templates
            .get(item_id)
            .ok_or_else(|| format!("kit item {item_id} has no template"))?;
        let goods = self
            .catalog
            .goods
            .get(&item_id)
            .ok_or_else(|| format!("kit item {item_id} is not in content_item"))?;
        let item = InventoryItemData {
            item_guid,
            item_id,
            name: goods.name.clone(),
            quality: goods.quality,
            required_level: template.required_level,
            stack_count: count,
            vendor_sell_price: goods.sell_price,
            unique_auction: false,
        };
        Ok((item, template))
    }
}

pub struct PlayerCreateInfoPlugin;

impl Plugin for PlayerCreateInfoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_player_create_infos);
    }
}

fn startup_load_player_create_infos(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
    let infos = load_player_create_infos(&conn).unwrap_or_else(|error| {
        panic!(
            "load character_loadout/tdb_playercreateinfo_* from {}: {error}; \
             run scripts/import-azerothcore.sh",
            path.display()
        )
    });
    info!(
        "Loaded player create info for {} race/class pairs",
        infos.0.len()
    );
    commands.insert_resource(infos);
}

#[cfg(test)]
#[path = "player_create_info_tests.rs"]
mod tests;
