//! Creature loot tables from world.db `content_creature_loot_template` and
//! `content_reference_loot_template` (Retail-mapped AzerothCore rows), and the
//! drop roll of AzerothCore `LootTemplate::Process` (LootMgr.cpp:1697):
//!
//! - ungrouped rows each roll their own `Chance` (`LootStoreItem::Roll`, :311);
//!   a reference row that hits processes the referenced template `MaxCount` times;
//! - every group yields at most one row (`LootGroup::Roll`, :1292): one roll walks
//!   the explicitly chanced rows, and when it misses, an equal-chance (`Chance` 0)
//!   row is picked at random; rows that already dropped 3 times (1 time for
//!   equippable items) in the same group are left out (`LootGroupInvalidSelector`, :59);
//! - `QuestRequired` rows drop only for a player whose quest needs the item;
//! - a dropped count above the stack size splits into stacks (`Loot::AddItem`, :512),
//!   and items without an item template are skipped.
//!
//! Only `LootMode` 1 (`LOOT_MODE_DEFAULT`) rows are loaded; the other modes are
//! script-set hard modes. Drop rates are the AzerothCore defaults (all 1).
//! Money is `Loot::generateMoneyLoot` (:838) over `creature_template.mingold` /
//! `maxgold`.

use std::collections::HashMap;

use bevy::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;
use rusqlite::Connection;

use crate::item_templates::{ItemTemplate, ItemTemplates};

/// AzerothCore `MAX_NR_LOOT_ITEMS` / `MAX_NR_QUEST_ITEMS` (LootMgr.h).
const MAX_NR_LOOT_ITEMS: usize = 16;
const MAX_NR_QUEST_ITEMS: usize = 32;
/// `LOOT_MODE_DEFAULT`.
const LOOT_MODE_DEFAULT: i64 = 1;

/// One loot template row (AzerothCore `LootStoreItem`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LootRow {
    pub item: u32,
    /// Referenced `reference_loot_template` entry; 0 for an item row.
    pub reference: u32,
    /// Percent, 0–100; 0 inside a group means equal chance.
    pub chance: f32,
    pub needs_quest: bool,
    pub group: u8,
    pub min_count: u32,
    /// For a reference row: how many times the reference is processed.
    pub max_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LootTemplate {
    pub entries: Vec<LootRow>,
    /// Group `n` is `groups[n - 1]`.
    pub groups: Vec<Vec<LootRow>>,
}

impl LootTemplate {
    /// AzerothCore `LootTemplate::AddEntry`: grouped rows (references included)
    /// go to their group, the rest are rolled one by one.
    fn add(&mut self, row: LootRow) {
        if row.group == 0 {
            self.entries.push(row);
            return;
        }
        let index = usize::from(row.group) - 1;
        if self.groups.len() <= index {
            self.groups.resize(index + 1, Vec::new());
        }
        self.groups[index].push(row);
    }
}

/// `creature_template.lootid`, `mingold`, `maxgold`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureLoot {
    pub loot_id: u32,
    pub min_gold: u32,
    pub max_gold: u32,
}

#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct LootTables {
    pub creature: HashMap<u32, LootTemplate>,
    pub game_object: HashMap<u32, LootTemplate>,
    pub reference: HashMap<u32, LootTemplate>,
    pub disenchant: HashMap<u32, LootTemplate>,
    pub gameobject: HashMap<u32, LootTemplate>,
    pub skinning: HashMap<u32, LootTemplate>,
    /// By creature template entry.
    pub money: HashMap<u32, CreatureLoot>,
    /// By (entry, dungeon difficulty): `content_creature_template_difficulty` loot.
    pub difficulty_money: HashMap<(u32, u32), CreatureLoot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DroppedItem {
    pub item_id: u32,
    pub count: u32,
    group: u8,
    pub(super) needs_quest: bool,
}

/// What one player's kill yields: items in slot order, then coins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RolledLoot {
    pub items: Vec<DroppedItem>,
    pub copper: u64,
}

/// `Loot::generateMoneyLoot` with `Rate.Drop.Money` 1.
pub(crate) fn money_loot(min: u32, max: u32, rng: &mut impl Rng) -> u64 {
    if max == 0 {
        return 0;
    }
    if max <= min {
        return u64::from(max);
    }
    if max - min < 32700 {
        return u64::from(rng.gen_range(min..=max));
    }
    u64::from(rng.gen_range(min >> 8..=max >> 8)) << 8
}

/// `roll_chance_f`.
fn roll_chance(chance: f32, rng: &mut impl Rng) -> bool {
    chance >= 100.0 || rng.gen_range(0.0..100.0) < chance
}

impl LootTables {
    /// Gathering shares authored reference/group/chance/count rolling with corpse loot.
    pub fn roll_gathering(
        &self,
        entry: u32,
        skinning: bool,
        items: &ItemTemplates,
        rng: &mut impl Rng,
    ) -> RolledLoot {
        let templates = if skinning {
            &self.skinning
        } else {
            &self.gameobject
        };
        let template = &templates[&entry];
        let mut drops = Vec::new();
        self.process(template, items, &|_| false, rng, &mut drops);
        RolledLoot {
            items: drops,
            copper: 0,
        }
    }

    /// Whether this entry can yield an item present in the active Retail catalog.
    pub fn disenchant_has_materials(&self, entry: u32, items: &ItemTemplates) -> bool {
        self.disenchant.get(&entry).is_some_and(|template| {
            self.misses(template, items)
                .values()
                .any(|miss| *miss < 1.0)
        })
    }

    pub fn roll_disenchant(
        &self,
        entry: u32,
        items: &ItemTemplates,
        rng: &mut impl Rng,
    ) -> Option<RolledLoot> {
        let template = &self.disenchant[&entry];
        let mut drops = Vec::new();
        self.process(template, items, &|_| false, rng, &mut drops);
        if drops.is_empty() {
            return None;
        }
        Some(RolledLoot {
            items: drops,
            copper: 0,
        })
    }

    /// The loot id and money of `entry` in a copy at `difficulty_id` (0 outside
    /// dungeons): its difficulty row's, else its template's.
    fn creature_loot(&self, entry: u32, difficulty_id: u32) -> Option<&CreatureLoot> {
        self.difficulty_money
            .get(&(entry, difficulty_id))
            .or_else(|| self.money.get(&entry))
    }

    /// One player's loot of a creature of a copy at `difficulty_id`: `needs_quest_item`
    /// says whether the player's quests still need an item (`LootItem::AllowedForPlayer`).
    pub fn roll_creature(
        &self,
        entry: u32,
        difficulty_id: u32,
        items: &ItemTemplates,
        needs_quest_item: impl Fn(u32) -> bool,
        rng: &mut impl Rng,
    ) -> RolledLoot {
        // A creature without a loot table drops nothing, coins included (user rule,
        // docs/specs/loot.md).
        let Some((info, template)) = self
            .creature_loot(entry, difficulty_id)
            .and_then(|info| Some((info, self.creature.get(&info.loot_id)?)))
        else {
            return RolledLoot::default();
        };
        let mut drops = Vec::new();
        self.process(template, items, &needs_quest_item, rng, &mut drops);
        let (mut normal, mut quest): (Vec<_>, Vec<_>) =
            drops.into_iter().partition(|drop| !drop.needs_quest);
        normal.truncate(MAX_NR_LOOT_ITEMS);
        quest.truncate(MAX_NR_QUEST_ITEMS);
        normal.extend(quest);
        RolledLoot {
            items: normal,
            copper: money_loot(info.min_gold, info.max_gold, rng),
        }
    }

    /// A chest's authored loot, filtered by the player's active item objectives.
    pub fn roll_game_object(
        &self,
        loot_id: u32,
        items: &ItemTemplates,
        needs_quest_item: impl Fn(u32) -> bool,
        rng: &mut impl Rng,
    ) -> RolledLoot {
        let mut drops = Vec::new();
        if let Some(template) = self.game_object.get(&loot_id) {
            self.process(template, items, &needs_quest_item, rng, &mut drops);
        }
        RolledLoot {
            items: drops,
            copper: 0,
        }
    }

    pub(super) fn process(
        &self,
        template: &LootTemplate,
        items: &ItemTemplates,
        needs_quest_item: &impl Fn(u32) -> bool,
        rng: &mut impl Rng,
        drops: &mut Vec<DroppedItem>,
    ) {
        for row in &template.entries {
            if !roll_chance(row.chance, rng) {
                continue;
            }
            self.drop_row(row, items, needs_quest_item, rng, drops);
        }
        for group in &template.groups {
            if let Some(row) = roll_group(group, items, drops, rng) {
                self.drop_row(&row, items, needs_quest_item, rng, drops);
            }
        }
    }

    fn drop_row(
        &self,
        row: &LootRow,
        items: &ItemTemplates,
        needs_quest_item: &impl Fn(u32) -> bool,
        rng: &mut impl Rng,
        drops: &mut Vec<DroppedItem>,
    ) {
        if row.reference != 0 {
            let Some(referenced) = self.reference.get(&row.reference) else {
                return;
            };
            for _ in 0..row.max_count {
                self.process(referenced, items, needs_quest_item, rng, drops);
            }
            return;
        }
        if row.needs_quest && !needs_quest_item(row.item) {
            return;
        }
        let Some(template) = items.get(shared::item_data::ItemDefinitionSource::Retail, row.item)
        else {
            return;
        };
        add_stacks(row, template, rng, drops);
    }
}

/// `Loot::AddItem`: `urand(MinCount, MaxCount)` items as full stacks.
fn add_stacks(
    row: &LootRow,
    template: &ItemTemplate,
    rng: &mut impl Rng,
    drops: &mut Vec<DroppedItem>,
) {
    let max_stack = template.max_stack.max(1);
    let mut count = rng.gen_range(row.min_count..=row.max_count.max(row.min_count));
    while count > 0 {
        let stack = count.min(max_stack);
        drops.push(DroppedItem {
            item_id: row.item,
            count: stack,
            group: row.group,
            needs_quest: row.needs_quest,
        });
        count -= stack;
    }
}

/// `LootGroupInvalidSelector`: an item row whose item already dropped 3 times
/// (1 time when equippable) from the same group, or has no template.
fn valid_in_group(row: &LootRow, items: &ItemTemplates, drops: &[DroppedItem]) -> bool {
    if row.reference != 0 {
        return true;
    }
    let Some(template) = items.get(shared::item_data::ItemDefinitionSource::Retail, row.item)
    else {
        return false;
    };
    let limit = if template.inventory_type == 0 { 3 } else { 1 };
    drops
        .iter()
        .filter(|drop| drop.item_id == row.item && drop.group == row.group)
        .count()
        < limit
}

/// `LootGroup::Roll`: the explicitly chanced rows share one roll; otherwise one
/// equal-chance row at random.
fn roll_group(
    group: &[LootRow],
    items: &ItemTemplates,
    drops: &[DroppedItem],
    rng: &mut impl Rng,
) -> Option<LootRow> {
    let (explicit, equal): (Vec<&LootRow>, Vec<&LootRow>) = group
        .iter()
        .filter(|row| valid_in_group(row, items, drops))
        .partition(|row| row.chance != 0.0);
    if !explicit.is_empty() {
        let mut roll: f32 = rng.gen_range(0.0..100.0);
        for row in explicit {
            if row.chance >= 100.0 {
                return Some(*row);
            }
            roll -= row.chance;
            if roll < 0.0 {
                return Some(*row);
            }
        }
    }
    equal.choose(rng).map(|row| **row)
}

impl LootTables {
    /// Chance per item that one kill of a creature entry drops it at least once, in
    /// percent, highest first (the unit tooltip's drop list, game-engine
    /// `docs/specs/unit-tooltip.md`). It follows `roll_creature` without a quest filter:
    ///
    /// - an ungrouped row counts its `Chance`;
    /// - a group row counts its share of the group's one roll: explicitly chanced
    ///   rows in order until 100 % is used up, and the rest split evenly among the
    ///   equal-chance rows;
    /// - a reference row multiplies its own chance by the chance that one of the
    ///   reference's `MaxCount` passes drops the item;
    /// - several rows of one item combine as independent rolls.
    ///
    /// The 3-per-group (1 when equippable) repeat limit never applies within one roll
    /// of these tables and is ignored.
    pub fn drop_chances(
        &self,
        entry: u32,
        difficulty_id: u32,
        items: &ItemTemplates,
    ) -> Vec<(u32, f32)> {
        let template = self
            .creature_loot(entry, difficulty_id)
            .and_then(|info| self.creature.get(&info.loot_id));
        let misses = template.map_or_else(HashMap::new, |template| self.misses(template, items));
        let mut chances: Vec<(u32, f32)> = misses
            .into_iter()
            .map(|(item, miss)| (item, ((1.0 - miss) * 100.0) as f32))
            .filter(|(_, chance)| *chance > 0.0)
            .collect();
        chances.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        chances
    }

    /// Probability per item that one pass of `template` does not drop it.
    fn misses(&self, template: &LootTemplate, items: &ItemTemplates) -> HashMap<u32, f64> {
        let mut misses = HashMap::new();
        for row in &template.entries {
            self.count_row(
                row,
                f64::from(row.chance.min(100.0)) / 100.0,
                items,
                &mut misses,
            );
        }
        for group in &template.groups {
            for (row, chance) in group_shares(group, items) {
                self.count_row(row, chance / 100.0, items, &mut misses);
            }
        }
        misses
    }

    fn count_row(
        &self,
        row: &LootRow,
        chance: f64,
        items: &ItemTemplates,
        misses: &mut HashMap<u32, f64>,
    ) {
        if row.reference == 0 {
            if items
                .get(shared::item_data::ItemDefinitionSource::Retail, row.item)
                .is_some()
            {
                *misses.entry(row.item).or_insert(1.0) *= 1.0 - chance;
            }
            return;
        }
        let Some(referenced) = self.reference.get(&row.reference) else {
            return;
        };
        for (item, pass_miss) in self.misses(referenced, items) {
            let dropped = 1.0 - pass_miss.powi(row.max_count as i32);
            *misses.entry(item).or_insert(1.0) *= 1.0 - chance * dropped;
        }
    }
}

/// Each row's percent of a group's one roll (`roll_group`).
fn group_shares<'a>(group: &'a [LootRow], items: &ItemTemplates) -> Vec<(&'a LootRow, f64)> {
    let rows: Vec<&LootRow> = group
        .iter()
        .filter(|row| {
            row.reference != 0
                || items
                    .get(shared::item_data::ItemDefinitionSource::Retail, row.item)
                    .is_some()
        })
        .collect();
    let mut left = 100.0_f64;
    let mut shares = Vec::new();
    for row in rows.iter().filter(|row| row.chance != 0.0) {
        let share = f64::from(row.chance).min(left);
        left -= share;
        shares.push((*row, share));
    }
    let equal: Vec<&&LootRow> = rows.iter().filter(|row| row.chance == 0.0).collect();
    for row in &equal {
        shares.push((**row, left / equal.len() as f64));
    }
    shares
}

/// TDB references remain separate from the creature content-policy tables.
pub(crate) fn load_disenchant_tables(conn: &Connection) -> rusqlite::Result<LootTables> {
    Ok(LootTables {
        disenchant: load_tdb_templates(conn, "tdb_disenchant_loot_template")?,
        reference: load_tdb_templates(conn, "tdb_reference_loot_template")?,
        ..LootTables::default()
    })
}

pub(crate) fn load_loot_tables(conn: &Connection) -> rusqlite::Result<LootTables> {
    Ok(LootTables {
        creature: load_templates(conn, "content_creature_loot_template")?,
        game_object: load_templates(conn, "content_gameobject_loot_template")?,
        reference: load_templates(conn, "content_reference_loot_template")?,
        disenchant: HashMap::new(),
        gameobject: load_templates(conn, "content_gameobject_loot_template")?,
        skinning: load_templates(conn, "content_skinning_loot_template")?,
        money: load_money(conn)?,
        difficulty_money: load_difficulty_money(conn)?,
    })
}

fn load_tdb_templates(
    conn: &Connection,
    table: &str,
) -> rusqlite::Result<HashMap<u32, LootTemplate>> {
    read_templates(
        conn,
        &format!(
            "SELECT Entry, Item, CASE ItemType WHEN 1 THEN Item ELSE 0 END AS Reference, Chance, QuestRequired, GroupId, MinCount, MaxCount FROM {table} WHERE LootMode & {LOOT_MODE_DEFAULT} != 0 AND ItemType IN (0, 1) ORDER BY Entry, GroupId, Item, Reference"
        ),
    )
}

fn load_templates(conn: &Connection, table: &str) -> rusqlite::Result<HashMap<u32, LootTemplate>> {
    read_templates(
        conn,
        &format!(
            "SELECT Entry, Item, Reference, Chance, QuestRequired, GroupId, MinCount, MaxCount \
         FROM {table} WHERE LootMode & {LOOT_MODE_DEFAULT} != 0 \
         ORDER BY Entry, GroupId, Item, Reference"
        ),
    )
}

fn read_templates(conn: &Connection, query: &str) -> rusqlite::Result<HashMap<u32, LootTemplate>> {
    let mut statement = conn.prepare(query)?;
    let rows = statement.query_map([], |row| {
        let entry: u32 = row.get(0)?;
        let loot_row = LootRow {
            item: row.get(1)?,
            reference: row.get(2)?,
            chance: row.get::<_, f64>(3)? as f32,
            needs_quest: row.get::<_, i64>(4)? != 0,
            group: row.get(5)?,
            min_count: row.get(6)?,
            max_count: row.get(7)?,
        };
        Ok((entry, loot_row))
    })?;
    let mut templates: HashMap<u32, LootTemplate> = HashMap::new();
    for row in rows {
        let (entry, loot_row) = row?;
        templates.entry(entry).or_default().add(loot_row);
    }
    Ok(templates)
}

fn load_difficulty_money(conn: &Connection) -> rusqlite::Result<HashMap<(u32, u32), CreatureLoot>> {
    let mut statement = conn.prepare(
        "SELECT entry, DifficultyID, lootid, mingold, maxgold FROM content_creature_template_difficulty",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            (row.get(0)?, row.get(1)?),
            CreatureLoot {
                loot_id: row.get(2)?,
                min_gold: row.get(3)?,
                max_gold: row.get(4)?,
            },
        ))
    })?;
    rows.collect()
}

fn load_money(conn: &Connection) -> rusqlite::Result<HashMap<u32, CreatureLoot>> {
    let mut statement =
        conn.prepare("SELECT entry, lootid, mingold, maxgold FROM content_creature_template")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            CreatureLoot {
                loot_id: row.get(1)?,
                min_gold: row.get(2)?,
                max_gold: row.get(3)?,
            },
        ))
    })?;
    rows.collect()
}

#[cfg(test)]
#[path = "tables_tests.rs"]
mod tests;
