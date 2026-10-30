//! Check Mythic+ follower profile candidates against native key-level upgrade tracks.
//! A season's ChallengeModeItemBonusOverride trees gate ItemBonusListGroups (upgrade
//! tracks) by key level; the exact rank inside a track comes from IblGroupPointsModSet,
//! which is not exported, so a key accepts every rank of each track it reaches.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use shared::item_data::ItemDefinitionSource;
use shared::item_scaling::ItemScalingTables;

use crate::item_instance::{
    ItemInstanceModifiers,
    bonuses::{ItemBonusTables, apply_item_bonuses},
    item_instance_stats,
};
use crate::item_templates::{ItemTemplate, ItemTemplates};
use crate::spell_info::ClassSpells;
use crate::trait_config::{Purchase, TraitContext, build_config};

/// Warcraft Logs Midnight Season 2 rankings are max-level characters.
const PROFILE_LEVEL: u8 = 90;
/// Mythic Keystone difficulty; its MapDifficulty row supplies the loot ItemContext.
const MYTHIC_KEYSTONE_DIFFICULTY: u32 = 8;
const INVENTORY_TYPE_SHIRT: u8 = 4;
const INVENTORY_TYPE_TABARD: u8 = 19;

#[derive(Deserialize)]
struct Override {
    #[serde(rename = "DstItemBonusTreeID")]
    tree: u32,
    #[serde(rename = "RequiredTimeEventPassed")]
    time_event: u32,
}

#[derive(Deserialize)]
struct Node {
    #[serde(rename = "ParentItemBonusTreeID")]
    parent: u32,
    #[serde(rename = "ItemContext")]
    context: u32,
    #[serde(rename = "ChildItemBonusTreeID")]
    child: u32,
    #[serde(rename = "ChildItemBonusListGroupID")]
    group: u32,
    #[serde(rename = "MinMythicPlusLevel")]
    min_key: i32,
    #[serde(rename = "MaxMythicPlusLevel")]
    max_key: i32,
}

impl Node {
    fn is_key_gated(&self) -> bool {
        self.min_key != 0 || self.max_key != 0
    }

    /// Zero minimum and non-positive maximum leave that side open.
    fn admits(&self, key: i32) -> bool {
        let above_min = self.min_key == 0 || key >= self.min_key;
        let below_max = self.max_key <= 0 || key <= self.max_key;
        above_min && below_max
    }
}

#[derive(Deserialize)]
struct GroupEntry {
    #[serde(rename = "ItemBonusListGroupID")]
    group: u32,
    #[serde(rename = "ItemBonusListID")]
    list: u32,
}

/// Key-gated upgrade tracks of one season's Mythic+ reward trees.
pub struct KeystoneTracks {
    gates: Vec<Node>,
    lists: HashMap<u32, Vec<u32>>,
}

impl KeystoneTracks {
    /// Every key-gated node of a tree, reached from the season's overrides, that gates
    /// the Mythic+ loot `context`. Trees gating only other contexts (Delve tiers) are skipped.
    pub fn read(db2: &Path, season_time_event: u32, context: u32) -> Result<Self, String> {
        let mut children: HashMap<u32, Vec<Node>> = HashMap::new();
        for node in super::read_rows::<Node>(db2, "ItemBonusTreeNode.csv")? {
            children.entry(node.parent).or_default().push(node);
        }
        let mut pending: Vec<u32> =
            super::read_rows::<Override>(db2, "ChallengeModeItemBonusOverride.csv")?
                .into_iter()
                .filter(|row| row.time_event == season_time_event)
                .map(|row| row.tree)
                .collect();
        let mut visited = HashSet::new();
        let mut gates = Vec::new();
        while let Some(tree) = pending.pop() {
            if !visited.insert(tree) {
                continue;
            }
            let Some(mut nodes) = children.remove(&tree) else {
                continue;
            };
            pending.extend(
                nodes
                    .iter()
                    .map(|node| node.child)
                    .filter(|&child| child != 0),
            );
            let gates_context = nodes
                .iter()
                .any(|node| node.is_key_gated() && node.context == context);
            if gates_context {
                nodes.retain(|node| node.is_key_gated() && node.group != 0);
                gates.extend(nodes);
            }
        }
        if gates.is_empty() {
            return Err(format!(
                "time event{season_time_event} has no key-gated tree for context{context}"
            ));
        }
        let mut lists: HashMap<u32, Vec<u32>> = HashMap::new();
        for entry in super::read_rows::<GroupEntry>(db2, "ItemBonusListGroupEntry.csv")? {
            lists.entry(entry.group).or_default().push(entry.list);
        }
        Ok(Self { gates, lists })
    }

    /// ItemBonusListGroups (upgrade tracks) a key of this level reaches.
    pub fn groups(&self, key: i32) -> BTreeSet<u32> {
        self.gates
            .iter()
            .filter(|node| node.admits(key))
            .map(|node| node.group)
            .collect()
    }

    /// Lowest and highest native item level the key's tracks give `item`.
    fn item_level_range(
        &self,
        key: i32,
        item: &ItemTemplate,
        bonuses: &ItemBonusTables,
        stats: &ItemScalingTables,
    ) -> Result<(u16, u16), String> {
        let mut levels = Vec::new();
        for group in self.groups(key) {
            for &list in self.lists.get(&group).into_iter().flatten() {
                levels.push(resolve_item_level(item, &[list], bonuses, stats)?);
            }
        }
        let low = levels
            .iter()
            .min()
            .ok_or(format!("+{key} reaches no track"))?;
        let high = levels
            .iter()
            .max()
            .ok_or(format!("+{key} reaches no track"))?;
        Ok((*low, *high))
    }
}

/// Native item level of `item` with `bonus_list` applied.
pub fn resolve_item_level(
    item: &ItemTemplate,
    bonus_list: &[u32],
    bonuses: &ItemBonusTables,
    stats: &ItemScalingTables,
) -> Result<u16, String> {
    let resolved = apply_item_bonuses(&item.scaling, bonus_list, bonuses, PROFILE_LEVEL)?;
    let instance = ItemInstanceModifiers::default();
    Ok(item_instance_stats(&resolved.scaling, item.inventory_type, &instance, stats)?.item_level)
}

/// One equipped item as Warcraft Logs reports it; bonus IDs arrive as strings.
#[derive(Deserialize)]
pub struct SourceItem {
    pub id: u32,
    #[serde(rename = "bonusIDs", default)]
    pub bonus_ids: Vec<String>,
}

/// A ranked character: actual key level, gear and the engine-decoded talent purchases.
#[derive(Deserialize)]
pub struct Candidate {
    pub key: String,
    pub class_id: u8,
    pub spec_id: u32,
    pub actual_key: i32,
    pub gear: Vec<SourceItem>,
    /// `(node, entry, bought ranks)` from `decode_follower_talents`.
    pub purchases: Vec<(u32, u32, u32)>,
}

#[derive(Debug, Serialize)]
pub struct Verdict {
    pub key: String,
    /// `(item, native item level)` for every resolved equipped item.
    pub item_levels: Vec<(u32, u16)>,
    /// Empty when the candidate qualifies.
    pub rejections: Vec<String>,
}

pub struct NativeProfileData {
    templates: ItemTemplates,
    bonuses: ItemBonusTables,
    stats: ItemScalingTables,
    classes: ClassSpells,
    tracks: KeystoneTracks,
}

impl NativeProfileData {
    /// `map` is the ranked dungeon; its Mythic Keystone MapDifficulty gives the loot context.
    pub fn read(
        world_db: &Path,
        db2: &Path,
        season_time_event: u32,
        map: u32,
    ) -> Result<Self, String> {
        let db = Connection::open_with_flags(world_db, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("open {}: {error}", world_db.display()))?;
        let context: u32 = db
            .query_row(
                "SELECT ItemContext FROM map_difficulty WHERE MapID=? AND DifficultyID=?",
                (map, MYTHIC_KEYSTONE_DIFFICULTY),
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("read map difficulty: {error}"))?
            .ok_or_else(|| format!("map{map} has no Mythic Keystone difficulty"))?;
        let read = |what: &str, error: rusqlite::Error| format!("read {what}: {error}");
        Ok(Self {
            templates: crate::item_templates::load_item_templates(world_db)
                .map_err(|error| read("item templates", error))?,
            bonuses: crate::item_instance::load::load_bonus_tables(&db)
                .map_err(|error| read("item bonuses", error))?,
            stats: crate::item_stats::load_item_scaling_tables(world_db)
                .map_err(|error| read("item scaling", error))?,
            classes: crate::spell_info::load_spell_data(&db)
                .map_err(|error| read("class spells", error))?
                .1,
            tracks: KeystoneTracks::read(db2, season_time_event, context)?,
        })
    }

    pub fn validate(&self, candidate: &Candidate) -> Verdict {
        let mut verdict = Verdict {
            key: candidate.key.clone(),
            item_levels: Vec::new(),
            rejections: Vec::new(),
        };
        for item in candidate.gear.iter().filter(|item| item.id != 0) {
            match self.check_item(candidate.actual_key, item) {
                Ok(Some(level)) => verdict.item_levels.push((item.id, level)),
                Ok(None) => {}
                Err(reason) => verdict
                    .rejections
                    .push(format!("item{}: {reason}", item.id)),
            }
        }
        if let Err(reason) = self.check_talents(candidate) {
            verdict.rejections.push(format!("talents: {reason}"));
        }
        verdict
    }

    /// The item's native level, or `None` for shirt and tabard slots.
    fn check_item(&self, key: i32, item: &SourceItem) -> Result<Option<u16>, String> {
        let template = self
            .templates
            .get(ItemDefinitionSource::Retail, item.id)
            .ok_or("not in native item templates")?;
        if matches!(
            template.inventory_type,
            INVENTORY_TYPE_SHIRT | INVENTORY_TYPE_TABARD
        ) {
            return Ok(None);
        }
        let bonus_list = item
            .bonus_ids
            .iter()
            .map(|id| {
                id.parse()
                    .map_err(|_| format!("bonus ID {id:?} is not a number"))
            })
            .collect::<Result<Vec<u32>, _>>()?;
        let level = resolve_item_level(template, &bonus_list, &self.bonuses, &self.stats)?;
        let (low, high) =
            self.tracks
                .item_level_range(key, template, &self.bonuses, &self.stats)?;
        if !(low..=high).contains(&level) {
            return Err(format!(
                "native item level {level} outside +{key} track range {low}-{high}"
            ));
        }
        Ok(Some(level))
    }

    fn check_talents(&self, candidate: &Candidate) -> Result<(), String> {
        let class = self
            .classes
            .classes
            .iter()
            .find(|class| class.class_id == candidate.class_id)
            .ok_or_else(|| format!("class{} has no native talent tree", candidate.class_id))?;
        let context = TraitContext {
            spec_id: candidate.spec_id,
            level: PROFILE_LEVEL,
        };
        let purchases: Vec<_> = candidate
            .purchases
            .iter()
            .map(|&(node_id, entry_id, ranks)| Purchase {
                node_id,
                entry_id,
                ranks,
            })
            .collect();
        build_config(&class.traits, context, &purchases).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MythicPlusSeason120 StartTimeEvent: Kings' Rest loot tree5953 overrides only here.
    const SEASON_TIME_EVENT: u32 = 1552;
    const RUBY_LIFE_POOLS: u32 = 2521;

    fn native() -> NativeProfileData {
        let world_db = crate::test_data::world_db_path();
        let db2 = world_db.parent().unwrap().join("db2/active");
        NativeProfileData::read(&world_db, &db2, SEASON_TIME_EVENT, RUBY_LIFE_POOLS).unwrap()
    }

    fn item(id: u32, bonus_ids: &[&str]) -> SourceItem {
        SourceItem {
            id,
            bonus_ids: bonus_ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    #[test]
    fn keystone_tracks_follow_native_key_gates() {
        let data = native();
        // Tree5935: Veteran616 to +5, Champion617 from +6 (and to +9), Hero618 from +10.
        assert_eq!(data.tracks.groups(3), BTreeSet::from([616, 617]));
        assert_eq!(data.tracks.groups(7), BTreeSet::from([617]));
        assert_eq!(data.tracks.groups(11), BTreeSet::from([617, 618]));
        assert_eq!(data.tracks.groups(21), BTreeSet::from([617, 618]));
    }

    #[test]
    fn wcl_candidate_items_resolve_natively_and_outliers_are_rejected() {
        let data = native();
        // Warcraft Logs Vayrados, +11 Ruby Life Pools: Hero6 (12854) and Champion6 (12846).
        let gear = vec![
            item(268265, &["6652", "13668", "13334", "13987", "12854"]),
            item(271463, &["6652", "13440", "13694", "13697", "12846"]),
            item(52019, &[]),
        ];
        let candidate = |actual_key| Candidate {
            key: "paladin-retribution/test".into(),
            class_id: 2,
            spec_id: 70,
            actual_key,
            gear: gear
                .iter()
                .map(|item| SourceItem {
                    id: item.id,
                    bonus_ids: item.bonus_ids.clone(),
                })
                .collect(),
            purchases: Vec::new(),
        };
        let verdict = data.validate(&candidate(11));
        assert_eq!(verdict.item_levels, [(268265, 334), (271463, 321)]);
        assert!(verdict.rejections.is_empty(), "{verdict:?}");
        let verdict = data.validate(&candidate(7));
        assert_eq!(
            verdict.rejections[0],
            "item268265: native item level 334 outside +7 track range 305-328"
        );
    }

    #[test]
    fn wcl_candidate_talents_use_strict_native_validation() {
        let data = native();
        let candidate = |purchases| Candidate {
            key: "hunter-beast-mastery/test".into(),
            class_id: 3,
            spec_id: 253,
            actual_key: 11,
            gear: Vec::new(),
            purchases,
        };
        assert!(data.validate(&candidate(Vec::new())).rejections.is_empty());
        let unknown = data.validate(&candidate(vec![(1, 1, 1)]));
        assert_eq!(unknown.rejections.len(), 1, "{unknown:?}");
        assert!(unknown.rejections[0].starts_with("talents: "));
    }
}
