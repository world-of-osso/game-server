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
    pub reference: HashMap<u32, LootTemplate>,
    /// By creature template entry.
    pub money: HashMap<u32, CreatureLoot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DroppedItem {
    pub item_id: u32,
    pub count: u32,
    group: u8,
    needs_quest: bool,
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
    /// One player's loot of a creature: `needs_quest_item` says whether the
    /// player's quests still need an item (`LootItem::AllowedForPlayer`).
    pub fn roll_creature(
        &self,
        entry: u32,
        items: &ItemTemplates,
        needs_quest_item: impl Fn(u32) -> bool,
        rng: &mut impl Rng,
    ) -> RolledLoot {
        let Some(info) = self.money.get(&entry) else {
            return RolledLoot::default();
        };
        let mut drops = Vec::new();
        if let Some(template) = self.creature.get(&info.loot_id) {
            self.process(template, items, &needs_quest_item, rng, &mut drops);
        }
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
        let Some(template) = items.get(row.item) else {
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
    let Some(template) = items.get(row.item) else {
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

pub(crate) fn load_loot_tables(conn: &Connection) -> rusqlite::Result<LootTables> {
    Ok(LootTables {
        creature: load_templates(conn, "content_creature_loot_template")?,
        reference: load_templates(conn, "content_reference_loot_template")?,
        money: load_money(conn)?,
    })
}

fn load_templates(conn: &Connection, table: &str) -> rusqlite::Result<HashMap<u32, LootTemplate>> {
    let mut statement = conn.prepare(&format!(
        "SELECT Entry, Item, Reference, Chance, QuestRequired, GroupId, MinCount, MaxCount \
         FROM {table} WHERE LootMode & {LOOT_MODE_DEFAULT} != 0 \
         ORDER BY Entry, GroupId, Item, Reference"
    ))?;
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

fn load_money(conn: &Connection) -> rusqlite::Result<HashMap<u32, CreatureLoot>> {
    let mut statement =
        conn.prepare("SELECT entry, lootid, mingold, maxgold FROM creature_template")?;
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
