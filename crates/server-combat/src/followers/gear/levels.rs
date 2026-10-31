//! Resolve dungeon loot levels from the active DB2 bonus trees, without an ilvl table.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use shared::item_scaling::ItemScalingTables;

use crate::item_instance::{
    ItemInstanceModifiers,
    bonuses::{ItemBonusTables, apply_item_bonuses},
    item_instance_stats,
};
use crate::item_templates::ItemTemplate;

#[derive(Deserialize)]
struct ItemTree {
    #[serde(rename = "ItemID")]
    item: u32,
    #[serde(rename = "ItemBonusTreeID")]
    tree: u32,
}

#[derive(Deserialize)]
struct Tree {
    #[serde(rename = "ID")]
    id: u32,
    #[serde(rename = "Flags")]
    flags: u32,
    #[serde(rename = "InventoryTypeSlotMask")]
    slots: i64,
}

#[derive(Deserialize)]
struct Node {
    #[serde(rename = "ParentItemBonusTreeID")]
    parent: u32,
    #[serde(rename = "ItemContext")]
    context: u32,
    #[serde(rename = "ChildItemBonusTreeID")]
    child: u32,
    #[serde(rename = "ChildItemBonusListID")]
    list: u32,
    #[serde(rename = "ChildItemLevelSelectorID")]
    selector: u32,
    #[serde(rename = "ChildItemBonusListGroupID")]
    group: u32,
    #[serde(rename = "ItemCreationContextGroupID")]
    context_group: u32,
    #[serde(rename = "MinMythicPlusLevel")]
    min_key: u32,
    #[serde(rename = "Flags")]
    flags: u32,
}

#[derive(Deserialize)]
struct PickerEntry {
    #[serde(rename = "ItemContextPickerID")]
    picker: u32,
    #[serde(rename = "ItemCreationContext")]
    context: u32,
    #[serde(rename = "OrderIndex")]
    order: u32,
    #[serde(rename = "PVal")]
    probability: i32,
    #[serde(rename = "LabelID")]
    label: u32,
    #[serde(rename = "Flags")]
    flags: u32,
    #[serde(rename = "PlayerConditionID")]
    condition: u32,
}

/// `ItemContextPickerEntry` flag 0x1: the entry applies when its PlayerCondition fails.
const PICKER_NEGATE_CONDITION: u32 = 0x1;

/// The native tree and context-picker inputs. No per-difficulty ilvl table is stored.
pub struct DungeonItemLevels {
    item_trees: HashMap<u32, Vec<u32>>,
    trees: HashMap<u32, Tree>,
    nodes: HashMap<u32, Vec<Node>>,
    pickers: HashMap<u32, Vec<PickerEntry>>,
    expressions: crate::world_state_expression::WorldStateExpressions,
}

impl DungeonItemLevels {
    pub fn read(db2: &Path) -> Result<Self, String> {
        let mut item_trees: HashMap<u32, Vec<u32>> = HashMap::new();
        for row in super::read_rows::<ItemTree>(db2, "ItemXBonusTree.csv")? {
            item_trees.entry(row.item).or_default().push(row.tree);
        }
        let trees = super::read_rows::<Tree>(db2, "ItemBonusTree.csv")?
            .into_iter()
            .map(|tree| (tree.id, tree))
            .collect();
        let mut nodes: HashMap<u32, Vec<Node>> = HashMap::new();
        for row in super::read_rows::<Node>(db2, "ItemBonusTreeNode.csv")? {
            nodes.entry(row.parent).or_default().push(row);
        }
        let mut pickers: HashMap<u32, Vec<PickerEntry>> = HashMap::new();
        for row in super::read_rows::<PickerEntry>(db2, "ItemContextPickerEntry.csv")? {
            if row.probability > 0 {
                pickers.entry(row.picker).or_default().push(row);
            }
        }
        Ok(Self {
            item_trees,
            trees,
            nodes,
            pickers,
            expressions: crate::world_state_expression::WorldStateExpressions::read(db2)?,
        })
    }

    /// `ItemBonusMgr::GetContextForPlayer`: the picker entry with the highest OrderIndex
    /// (first in store order on ties) whose PlayerCondition, negated by flag 0x1, holds
    /// for `level`. Entries are tried highest order first, so lower-order conditions the
    /// bounded PlayerCondition evaluator cannot answer are never needed once one passes.
    fn pick_context(
        &self,
        db: &Connection,
        picker: u32,
        context: u32,
        level: u8,
    ) -> Result<u32, String> {
        let mut entries: Vec<_> = self
            .pickers
            .get(&picker)
            .ok_or_else(|| format!("item context picker{picker} unavailable"))?
            .iter()
            .collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.order));
        for entry in entries {
            let met = match entry.condition {
                0 => true,
                id => crate::adventure_guide::conditions::load_conditions(
                    db,
                    BTreeSet::from([id]),
                    &self.expressions,
                )?[&id]
                    .evaluate(level, None)
                    .map_err(|reason| format!("item context picker{picker}: {reason}"))?,
            };
            if met == (entry.flags & PICKER_NEGATE_CONDITION != 0) {
                continue;
            }
            if entry.label != 0 {
                return Err(format!(
                    "FOLLOWER_ITEM_CONTEXT_PICKER_LABEL_UNSUPPORTED picker{picker} label{}",
                    entry.label
                ));
            }
            return Ok(entry.context);
        }
        Ok(context)
    }

    pub fn resolve(
        &self,
        db: &Connection,
        map: u32,
        difficulty: u32,
        level: u8,
        item_id: u32,
        item: &ItemTemplate,
        bonuses: &ItemBonusTables,
        stats: &ItemScalingTables,
    ) -> Result<u16, String> {
        if difficulty == 8 {
            return Err("FOLLOWER_MYTHIC_PLUS_SEASON_ITEM_LEVEL_UNSOURCED".into());
        }
        let (context, picker): (u32, u32) = db.query_row(
            "SELECT ItemContext,ItemContextPickerID FROM map_difficulty WHERE MapID=? AND DifficultyID=?",
            (map, difficulty), |row| Ok((row.get(0)?, row.get(1)?)))
            .optional().map_err(|error| format!("read map difficulty: {error}"))?
            .ok_or_else(|| format!("map{map} difficulty{difficulty} item context unavailable"))?;
        let context = match picker {
            0 => context,
            picker => self.pick_context(db, picker, context, level)?,
        };
        let lists = self.level_lists(item_id, item, context, bonuses)?;
        let resolved = apply_item_bonuses(&item.scaling, &lists, bonuses, level)?;
        let item_stats = item_instance_stats(
            &resolved.scaling,
            item.inventory_type,
            &ItemInstanceModifiers::default(),
            stats,
        )?;
        Ok(item_stats.item_level)
    }

    fn level_lists(
        &self,
        item_id: u32,
        item: &ItemTemplate,
        context: u32,
        bonuses: &ItemBonusTables,
    ) -> Result<Vec<u32>, String> {
        let roots = self
            .item_trees
            .get(&item_id)
            .ok_or_else(|| format!("item{item_id} bonus tree unavailable"))?;
        let mut pending = roots.clone();
        let mut visited = HashSet::new();
        let mut lists = Vec::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let tree = self
                .trees
                .get(&id)
                .ok_or_else(|| format!("item bonus tree{id} unavailable"))?;
            if tree.slots != 0 && tree.slots as u32 & (1_u32 << item.inventory_type) == 0 {
                continue;
            }
            let caster = item.scaling.flags2 & shared::item_scaling::FLAG2_CASTER_WEAPON != 0;
            if tree.flags & 8 != 0 && !caster || tree.flags & 16 != 0 && caster {
                continue;
            }
            if tree.flags & 0x60 != 0 {
                return Err(format!(
                    "item bonus tree{id} requires unsupported trinket flags"
                ));
            }
            let Some(nodes) = self.nodes.get(&id) else {
                continue;
            };
            if tree.flags & 4 == 0
                && nodes
                    .iter()
                    .filter(|node| {
                        node.min_key == 0 && (node.context == 0 || node.context == context)
                    })
                    .count()
                    > 1
            {
                continue;
            }
            for node in nodes {
                let matches = node.context == 0 || node.context == context;
                if node.context != 0 && matches == (node.flags & 1 != 0) {
                    continue;
                }
                if node.context_group != 0 || node.selector != 0 || node.group != 0 {
                    return Err(format!("FOLLOWER_ITEM_BONUS_TREE_UNRESOLVED tree{id}"));
                }
                if node.child != 0 {
                    pending.push(node.child);
                }
                if node.list != 0 {
                    let rows = bonuses
                        .entries
                        .get(&node.list)
                        .ok_or_else(|| format!("item bonus list{} unavailable", node.list))?;
                    if rows.iter().any(|row| row.kind == 53) {
                        return Err("FOLLOWER_SCALING_ITEM_LEVEL_BONUS_UNSUPPORTED".into());
                    }
                    if rows
                        .iter()
                        .any(|row| matches!(row.kind, 1 | 42 | 49 | 51 | 52))
                    {
                        lists.push(node.list);
                    }
                }
            }
        }
        Ok(lists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OpenFlags;
    use shared::item_data::ItemDefinitionSource;

    #[test]
    fn follower_gear_resolves_native_azure_difficulty_levels_without_a_table() {
        let path = crate::test_data::world_db_path();
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let db2 = path.parent().unwrap().join("db2/active");
        let source = DungeonItemLevels::read(&db2).unwrap();
        let items = crate::item_templates::load_item_templates(&path).unwrap();
        let item = items.get(ItemDefinitionSource::Retail, 193633).unwrap();
        let bonuses = crate::item_instance::load::load_bonus_tables(&db).unwrap();
        let stats = crate::item_stats::load_item_scaling_tables(&path).unwrap();
        assert_eq!(
            source
                .resolve(&db, 2515, 205, 70, 193633, item, &bonuses, &stats)
                .unwrap(),
            73
        );
        assert_eq!(
            source
                .resolve(&db, 2515, 23, 70, 193633, item, &bonuses, &stats)
                .unwrap(),
            76
        );
        assert!(
            source
                .resolve(&db, 2515, 8, 70, 193633, item, &bonuses, &stats)
                .unwrap_err()
                .contains("FOLLOWER_MYTHIC_PLUS_SEASON_ITEM_LEVEL_UNSOURCED")
        );
    }

    #[test]
    fn follower_gear_context_picker_follows_the_player_condition_for_darkflame() {
        let path = crate::test_data::world_db_path();
        let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let db2 = path.parent().unwrap().join("db2/active");
        let source = DungeonItemLevels::read(&db2).unwrap();
        let items = crate::item_templates::load_item_templates(&path).unwrap();
        // Ol' Waxbeard Journal loot; map2651/205 uses picker277 (entries 994/995 on
        // PlayerCondition120217, ContentTuning2462 level 80 or above).
        let item = items.get(ItemDefinitionSource::Retail, 221097).unwrap();
        let bonuses = crate::item_instance::load::load_bonus_tables(&db).unwrap();
        let stats = crate::item_stats::load_item_scaling_tables(&path).unwrap();
        let level = |player_level| {
            source
                .resolve(&db, 2651, 205, player_level, 221097, item, &bonuses, &stats)
                .unwrap()
        };
        // Level 80 meets 120217: entry995 (context1). Level 79 fails it: negated entry994 (context17).
        assert_eq!((level(80), level(79)), (79, 74));
    }
}
