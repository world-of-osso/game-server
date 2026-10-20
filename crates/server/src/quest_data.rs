//! Static quest content from world.db `content_quest_*` (Retail-first mapped
//! AzerothCore rows, see docs/wiki/systems/content-import.md).
//!
//! Previous-quest lists follow AzerothCore `ObjectMgr::LoadQuests`: a quest's
//! `PrevQuestID` plus every quest naming it as `NextQuestID` (sign kept: positive
//! = must be turned in, negative = must be in the log), and `RewardNextQuest`
//! links form the chain checked by `SatisfyQuestPrevChain`/`NextChain`.
use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags, Row};
use shared::protocol::{
    QuestMarkerClass, QuestObjectiveKind, QuestPoiPoint, QuestPoiSnapshot, QuestRepeatability,
};

use crate::maps::store::AreaTriggerShape;

/// `Flags` bits (AzerothCore `QuestFlags`).
const QUEST_FLAGS_DAILY: u32 = 0x1000;
const QUEST_FLAGS_WEEKLY: u32 = 0x8000;
/// `FlagsEx` bit (TrinityCore `QUEST_FLAGS_EX_LEGENDARY`, QuestDef.h:260).
const QUEST_FLAGS_EX_LEGENDARY: u32 = 0x100;
/// `QuestInfo.Modifiers` bits (`Quest::IsImportant` / `Quest::IsMeta`, QuestDef.cpp:509-523).
const QUEST_INFO_MODIFIER_WORLD_QUEST: u32 = 0x100;
const QUEST_INFO_MODIFIER_IMPORTANT: u32 = 0x400;
const QUEST_INFO_MODIFIER_META: u32 = 0x800;
/// `QuestInfo.Type` `QuestTagType::CovenantCalling` (QuestDef.h:349).
const QUEST_TAG_TYPE_COVENANT_CALLING: u8 = 15;
/// `SpecialFlags` bit (AzerothCore `QUEST_SPECIAL_FLAGS_REPEATABLE`).
const QUEST_SPECIAL_FLAGS_REPEATABLE: u32 = 0x1;

/// `SpecialFlags` bit (AzerothCore `QUEST_SPECIAL_FLAGS_EXPLORATION_OR_EVENT`): the
/// quest also needs an area explored or a scripted event (`AreaExploredOrEventHappens`).
const QUEST_SPECIAL_FLAGS_EXPLORATION_OR_EVENT: u32 = 0x2;
/// `SpecialFlags` bits (TrinityCore QuestDef.h:323-324 `DF_QUEST`, `MONTHLY`).
const QUEST_SPECIAL_FLAGS_DF_QUEST: u32 = 0x8;
const QUEST_SPECIAL_FLAGS_MONTHLY: u32 = 0x10;

/// What an objective counts. The protocol kinds plus AzerothCore's exploration/event
/// requirement (TrinityCore `QUEST_OBJECTIVE_AREATRIGGER` or a completion flag, mapped
/// by content import), which the protocol has no kind for and the client never sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectiveKind {
    Monster,
    ProgressBar,
    TalkTo,
    PlayerKills,
    CriteriaTree,
    AreaTriggerEnter,
    KillWithLabel,
    GameObject,
    Item,
    /// Set by entering one of the quest's `areatrigger_involvedrelation` triggers.
    ExplorationOrEvent,
}

impl ObjectiveKind {
    pub fn protocol(self) -> Option<QuestObjectiveKind> {
        match self {
            Self::Monster => Some(QuestObjectiveKind::Monster),
            Self::ProgressBar => Some(QuestObjectiveKind::ProgressBar),
            Self::TalkTo => Some(QuestObjectiveKind::TalkTo),
            Self::PlayerKills => Some(QuestObjectiveKind::PlayerKills),
            Self::CriteriaTree => Some(QuestObjectiveKind::CriteriaTree),
            Self::AreaTriggerEnter => Some(QuestObjectiveKind::AreaTriggerEnter),
            Self::KillWithLabel => Some(QuestObjectiveKind::KillWithLabel),
            Self::GameObject => Some(QuestObjectiveKind::GameObject),
            Self::Item => Some(QuestObjectiveKind::Item),
            Self::ExplorationOrEvent => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ObjectiveDef {
    pub kind: ObjectiveKind,
    pub object_id: u32,
    pub required: u32,
    pub text: String,
}

/// `RewardMoneyDifficulty`, `RewardMoneyMultiplier` and the quest's
/// ContentTuning levels (`content_tuning_level`, DB2Manager::GetContentTuningData).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct QuestMoneyScaling {
    pub difficulty: u8,
    pub multiplier: f32,
    /// `MinLevelWithDelta`; 0 when the ContentTuning row is missing.
    pub min_level: u8,
    pub max_level: u8,
    /// ContentTuningEntry::GetScalingFactionGroup: 3 Alliance, 5 Horde, 0 both.
    pub faction_group: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct QuestTemplate {
    pub id: u32,
    pub title: String,
    /// -1 scales to the player's level.
    pub level: i32,
    pub min_level: u8,
    /// 0 = no maximum.
    pub max_level: u8,
    pub sort_id: i32,
    pub suggested_group: u8,
    pub flags: u32,
    /// Authored TimeAllowed seconds (content_quest_template).
    pub time_allowed: u32,
    /// TrinityCore `FlagsEx` (tdb_quest_template); 0 for quests TrinityCore lacks.
    pub flags_ex: u32,
    /// The `QuestInfo` DB2 row of `QuestInfoID`: `Type` (`QuestTagType`) and
    /// `Modifiers`; 0 without one.
    pub quest_info_type: u8,
    pub quest_info_modifiers: u32,
    pub special_flags: u32,
    /// 0 = any; legacy race-ID bits or explicitly labelled PlayableRaceBit masks.
    pub allowable_races: u64,
    pub race_mask_playable: bool,
    /// Captured objectives not implemented by the native quest system cannot auto-complete.
    pub completion_blocked: bool,
    /// Class bits (`1 << (class - 1)`); 0 = any.
    pub allowable_classes: u32,
    pub exclusive_group: i32,
    pub reward_next_quest: u32,
    /// Copper; negative = the quest costs money to turn in. TrinityCore quests
    /// store `Quest::MaxMoneyValue` here and pay `money_scaling` per player.
    pub reward_money: i32,
    /// Authored selected-TC RewardBonusMoney; legacy/captured rows have no bonus.
    pub reward_bonus_money: u32,
    /// TrinityCore quests (`ContentTuningID` != 0): `Quest::MoneyValue` inputs and the
    /// ContentTuning range `Player::GetQuestLevel` clamps to.
    pub money_scaling: Option<QuestMoneyScaling>,
    /// `RewardXPDifficulty`: column of the QuestXP DB2 row.
    pub reward_xp_difficulty: u8,
    /// Fixed captured reward; Some(0) also prevents guessed XP for unknown reconstruction data.
    pub captured_reward_xp: Option<u32>,
    /// Item given on accept (`StartItem` x `ProvidedItemCount`).
    pub start_item: u32,
    pub provided_item_count: u32,
    /// Authored TDB rewards whose CurrencyTypes is a profession TraitCurrency.
    pub profession_knowledge_rewards: Vec<(u32, u32)>,
    pub reward_items: Vec<(u32, u32)>,
    pub choice_items: Vec<(u32, u32)>,
    pub objectives: Vec<ObjectiveDef>,
    pub world_objective_rules: Option<Vec<crate::world_quest_objectives::ObjectiveRule>>,
    pub description: String,
    pub objectives_text: String,
    pub completion_log: String,
    pub offer_reward_text: String,
    pub request_items_text: String,
}

impl QuestTemplate {
    pub fn repeatability(&self) -> QuestRepeatability {
        if self.flags & QUEST_FLAGS_DAILY != 0 {
            QuestRepeatability::Daily
        } else if self.flags & QUEST_FLAGS_WEEKLY != 0 {
            QuestRepeatability::Weekly
        } else {
            QuestRepeatability::Normal
        }
    }

    /// The giver marker kind TrinityCore `Player::GetQuestDialogStatus` picks
    /// (Player.cpp:16167-16241): important, meta, covenant calling, legendary, then
    /// daily/weekly. Campaign (`QuestMgr::IsCampaignQuestStatusVisibleForPlayer`)
    /// needs PlayerCondition evaluation, which the server lacks, so it is never picked.
    pub fn marker_class(&self) -> QuestMarkerClass {
        if self.quest_info_modifiers & QUEST_INFO_MODIFIER_IMPORTANT != 0 {
            QuestMarkerClass::Important
        } else if self.quest_info_modifiers & QUEST_INFO_MODIFIER_META != 0 {
            QuestMarkerClass::Meta
        } else if self.quest_info_type == QUEST_TAG_TYPE_COVENANT_CALLING {
            QuestMarkerClass::Calling
        } else if self.flags_ex & QUEST_FLAGS_EX_LEGENDARY != 0 {
            QuestMarkerClass::Legendary
        } else if self.repeatability() != QuestRepeatability::Normal {
            QuestMarkerClass::Repeatable
        } else {
            QuestMarkerClass::Normal
        }
    }

    /// Normal repeatables can be taken immediately; daily/weekly use reward timestamps.
    pub fn is_repeatable(&self) -> bool {
        self.special_flags & QUEST_SPECIAL_FLAGS_REPEATABLE != 0
            && self.repeatability() == QuestRepeatability::Normal
    }

    /// TrinityCore `Quest::CanIncreaseRewardedQuestCounters` (QuestDef.cpp:644-649):
    /// Dungeon Finder, daily and plain repeatable quests are never counted as rewarded.
    pub fn increases_rewarded_counters(&self) -> bool {
        let repeatable = self.special_flags & QUEST_SPECIAL_FLAGS_REPEATABLE != 0;
        self.special_flags & QUEST_SPECIAL_FLAGS_DF_QUEST == 0
            && self.flags & QUEST_FLAGS_DAILY == 0
            && (!repeatable
                || self.flags & QUEST_FLAGS_WEEKLY != 0
                || self.special_flags & QUEST_SPECIAL_FLAGS_MONTHLY != 0)
    }
}

/// `content_item` attributes a granted item needs.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct QuestItemInfo {
    pub name: String,
    pub quality: u8,
    pub required_level: u16,
    pub sell_price: u32,
}

#[derive(Resource, Debug, Default, Clone)]
pub(crate) struct QuestDb {
    /// Runtime evaluation clock, refreshed before quest requests and markers.
    pub now: Option<i64>,
    pub world_quests: Vec<crate::world_quests::Definition>,
    pub world_quest_areas: HashMap<u32, Vec<crate::world_quests::Area>>,
    pub world_quest_rewards: HashMap<u32, crate::world_quest_rewards::Rewards>,
    pub world_quest_xp: HashMap<(u8, u8), u32>,
    pub world_quest_windows: std::collections::BTreeMap<u32, crate::world_quests::Window>,
    pub quests: HashMap<u32, QuestTemplate>,
    /// Creature entry -> quests it starts.
    pub starters: HashMap<u32, Vec<u32>>,
    /// Creature entry -> quests it ends.
    pub enders: HashMap<u32, Vec<u32>>,
    /// Signed previous quests (see module docs).
    pub prev_quests: HashMap<u32, Vec<i32>>,
    /// Quests whose `RewardNextQuest` is the key.
    pub prev_chain_quests: HashMap<u32, Vec<u32>>,
    pub exclusive_groups: HashMap<i32, Vec<u32>>,
    /// Authentic item metadata for quest creation, including later quest references.
    pub items: HashMap<u32, QuestItemInfo>,
    pub pois: HashMap<u32, Vec<QuestPoiSnapshot>>,
    /// `quest_money_reward`: copper by quest level (ID) and RewardMoneyDifficulty.
    pub money_rewards: HashMap<u8, [u32; 10]>,
    /// `areatrigger_involvedrelation` triggers by map (`ObjectMgr::GetQuestsForAreaTrigger`).
    pub area_triggers: HashMap<u32, Vec<QuestAreaTrigger>>,
    /// Creature entry -> its `KillCredit1`/`KillCredit2` entries (`Player::KilledMonster`).
    pub kill_credits: HashMap<u32, Vec<u32>>,
    /// Authenticated CreatureLabel membership by entry and selected difficulty mode.
    pub creature_labels: HashMap<(u32, u32), Vec<u32>>,
    /// Single amount-one leaf task trees: root -> (authenticated criterion type, asset).
    pub task_criteria: HashMap<u32, (u32, u32)>,
}

/// An `AreaTrigger.db2` row that explores quests.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuestAreaTrigger {
    pub id: u32,
    /// WoW coordinates.
    pub center: Vec3,
    pub shape: AreaTriggerShape,
    pub quests: Vec<u32>,
}

impl QuestDb {
    pub fn item_name(&self, item_id: u32) -> String {
        self.items
            .get(&item_id)
            .map(|item| item.name.clone())
            .unwrap_or_default()
    }
}

const QUEST_QUERY: &str = "SELECT q.*, a.MaxLevel, a.AllowableClasses, a.PrevQuestID, \
     a.NextQuestID, a.ExclusiveGroup, a.ProvidedItemCount, a.SpecialFlags, \
     o.RewardText, r.CompletionText, t.MinLevelWithDelta AS TuningMinLevelWithDelta, \
     t.maxlevel AS TuningMaxLevel, t.ScalingFactionGroup AS TuningScalingFactionGroup, \
     tq.FlagsEx, qi.Type AS QuestInfoType, qi.Modifiers AS QuestInfoModifiers \
     FROM content_quest_template q \
     LEFT JOIN quest_info qi ON qi.ID = q.QuestInfoID \
     LEFT JOIN tdb_quest_template tq ON tq.ID = q.ID \
     LEFT JOIN content_tuning_level t ON t.ID = q.ContentTuningID \
     LEFT JOIN content_quest_template_addon a ON a.ID = q.ID \
     LEFT JOIN content_quest_offer_reward o ON o.ID = q.ID \
     LEFT JOIN content_quest_request_items r ON r.ID = q.ID";

/// Addon columns (`PrevQuestID`, `NextQuestID`) the link tables are built from.
struct QuestLinks {
    prev_quest: i32,
    next_quest: i32,
}

fn text(row: &Row, column: &str) -> rusqlite::Result<String> {
    Ok(row.get::<_, Option<String>>(column)?.unwrap_or_default())
}

fn int(row: &Row, column: &str) -> rusqlite::Result<i64> {
    Ok(row.get::<_, Option<i64>>(column)?.unwrap_or_default())
}

fn item_pairs(
    row: &Row,
    id_column: &str,
    count_column: &str,
    n: usize,
) -> rusqlite::Result<Vec<(u32, u32)>> {
    let mut pairs = Vec::new();
    for index in 1..=n {
        let id = int(row, &format!("{id_column}{index}"))?;
        let count = int(row, &format!("{count_column}{index}"))?;
        if id > 0 && count > 0 {
            pairs.push((id as u32, count as u32));
        }
    }
    Ok(pairs)
}

/// Creature/game object objectives (`RequiredNpcOrGo1-4`, negative = game
/// object) then item objectives (`RequiredItemId1-6`), Wrath objective order, then the
/// exploration/event requirement.
fn objectives_from_row(row: &Row, names: &ContentNames) -> rusqlite::Result<Vec<ObjectiveDef>> {
    let mut objectives = Vec::new();
    for index in 1..=4 {
        let id = int(row, &format!("RequiredNpcOrGo{index}"))?;
        let required = int(row, &format!("RequiredNpcOrGoCount{index}"))?;
        if id == 0 || required <= 0 {
            continue;
        }
        let authored = text(row, &format!("ObjectiveText{index}"))?;
        let (kind, object_id) = if id > 0 {
            (ObjectiveKind::Monster, id as u32)
        } else {
            (ObjectiveKind::GameObject, id.unsigned_abs() as u32)
        };
        let text = match (authored.is_empty(), names.creatures.get(&object_id)) {
            (true, Some(name)) if kind == ObjectiveKind::Monster => format!("{name} slain"),
            _ => authored,
        };
        objectives.push(ObjectiveDef {
            kind,
            object_id,
            required: required as u32,
            text,
        });
    }
    for (item_id, required) in item_pairs(row, "RequiredItemId", "RequiredItemCount", 6)? {
        objectives.push(ObjectiveDef {
            kind: ObjectiveKind::Item,
            object_id: item_id,
            required,
            text: names
                .items
                .get(&item_id)
                .map(|item| item.name.clone())
                .unwrap_or_default(),
        });
    }
    if int(row, "SpecialFlags")? as u32 & QUEST_SPECIAL_FLAGS_EXPLORATION_OR_EVENT != 0 {
        objectives.push(ObjectiveDef {
            kind: ObjectiveKind::ExplorationOrEvent,
            object_id: 0,
            required: 1,
            text: text(row, "AreaDescription")?,
        });
    }
    Ok(objectives)
}

fn quest_from_row(
    row: &Row,
    names: &ContentNames,
) -> rusqlite::Result<(QuestTemplate, QuestLinks)> {
    let quest = QuestTemplate {
        id: int(row, "ID")? as u32,
        title: text(row, "LogTitle")?,
        level: int(row, "QuestLevel")? as i32,
        min_level: int(row, "MinLevel")?.clamp(0, 255) as u8,
        max_level: int(row, "MaxLevel")?.clamp(0, 255) as u8,
        sort_id: int(row, "QuestSortID")? as i32,
        suggested_group: int(row, "SuggestedGroupNum")?.clamp(0, 255) as u8,
        flags: int(row, "Flags")? as u32,
        time_allowed: int(row, "TimeAllowed")? as u32,
        flags_ex: int(row, "FlagsEx")? as u32,
        quest_info_type: int(row, "QuestInfoType")?.clamp(0, 255) as u8,
        quest_info_modifiers: int(row, "QuestInfoModifiers")? as u32,
        special_flags: int(row, "SpecialFlags")? as u32,
        allowable_races: int(row, "AllowableRaces")? as u64,
        race_mask_playable: false,
        completion_blocked: false,
        allowable_classes: int(row, "AllowableClasses")? as u32,
        exclusive_group: int(row, "ExclusiveGroup")? as i32,
        reward_next_quest: int(row, "RewardNextQuest")? as u32,
        reward_money: int(row, "RewardMoney")? as i32,
        reward_bonus_money: 0,
        money_scaling: (int(row, "ContentTuningID")? != 0)
            .then(|| -> rusqlite::Result<QuestMoneyScaling> {
                Ok(QuestMoneyScaling {
                    difficulty: int(row, "RewardMoneyDifficulty")?.clamp(0, 255) as u8,
                    multiplier: row
                        .get::<_, Option<f64>>("RewardMoneyMultiplier")?
                        .unwrap_or(1.0) as f32,
                    min_level: int(row, "TuningMinLevelWithDelta")?.clamp(0, 255) as u8,
                    max_level: int(row, "TuningMaxLevel")?.clamp(0, 255) as u8,
                    faction_group: int(row, "TuningScalingFactionGroup")? as u32,
                })
            })
            .transpose()?,
        reward_xp_difficulty: int(row, "RewardXPDifficulty")?.clamp(0, 255) as u8,
        captured_reward_xp: None,
        start_item: int(row, "StartItem")? as u32,
        provided_item_count: int(row, "ProvidedItemCount")? as u32,
        profession_knowledge_rewards: Vec::new(),
        reward_items: item_pairs(row, "RewardItem", "RewardAmount", 4)?,
        choice_items: item_pairs(row, "RewardChoiceItemID", "RewardChoiceItemQuantity", 6)?,
        objectives: objectives_from_row(row, names)?,
        world_objective_rules: None,
        description: text(row, "QuestDescription")?,
        objectives_text: text(row, "LogDescription")?,
        completion_log: text(row, "QuestCompletionLog")?,
        offer_reward_text: text(row, "RewardText")?,
        request_items_text: text(row, "CompletionText")?,
    };
    let links = QuestLinks {
        prev_quest: int(row, "PrevQuestID")? as i32,
        next_quest: int(row, "NextQuestID")? as i32,
    };
    Ok((quest, links))
}

fn load_creature_names(conn: &Connection) -> rusqlite::Result<HashMap<u32, String>> {
    let mut statement = conn.prepare("SELECT entry, name FROM content_creature_template")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get(1)?)))?;
    rows.collect()
}

fn load_kill_credits(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<u32>>> {
    let mut statement = conn.prepare(
        "SELECT entry, KillCredit1, KillCredit2 FROM content_creature_template \
         WHERE KillCredit1 <> 0 OR KillCredit2 <> 0",
    )?;
    let rows = statement.query_map([], |row| {
        let credits: [u32; 2] = [row.get(1)?, row.get(2)?];
        Ok((
            row.get::<_, u32>(0)?,
            credits.into_iter().filter(|&credit| credit != 0).collect(),
        ))
    })?;
    rows.collect()
}

fn load_relations(conn: &Connection, table: &str) -> rusqlite::Result<HashMap<u32, Vec<u32>>> {
    let mut statement =
        conn.prepare(&format!("SELECT id, quest FROM {table} ORDER BY id, quest"))?;
    let mut relations: HashMap<u32, Vec<u32>> = HashMap::new();
    let rows = statement.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get(1)?)))?;
    for row in rows {
        let (npc, quest) = row?;
        relations.entry(npc).or_default().push(quest);
    }
    Ok(relations)
}

fn load_all_items(conn: &Connection) -> rusqlite::Result<HashMap<u32, QuestItemInfo>> {
    let mut statement =
        conn.prepare("SELECT ID, Name, Quality, RequiredLevel, SellPrice FROM content_item")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            QuestItemInfo {
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                quality: row.get(2)?,
                required_level: row.get(3)?,
                sell_price: row.get::<_, i64>(4)?.max(0) as u32,
            },
        ))
    })?;
    rows.collect()
}

fn load_poi_points(conn: &Connection) -> rusqlite::Result<HashMap<(u32, u32), Vec<QuestPoiPoint>>> {
    let mut statement = conn.prepare(
        "SELECT QuestID, Idx1, X, Y FROM content_quest_poi_points ORDER BY QuestID, Idx1, Idx2",
    )?;
    let mut points: HashMap<(u32, u32), Vec<QuestPoiPoint>> = HashMap::new();
    let rows = statement.query_map([], |row| {
        Ok((
            (row.get::<_, u32>(0)?, row.get::<_, u32>(1)?),
            QuestPoiPoint {
                x: row.get(2)?,
                y: row.get(3)?,
            },
        ))
    })?;
    for row in rows {
        let (key, point) = row?;
        points.entry(key).or_default().push(point);
    }
    Ok(points)
}

fn load_pois(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<QuestPoiSnapshot>>> {
    let mut points = load_poi_points(conn)?;
    let mut statement = conn.prepare(
        "SELECT QuestID, id, ObjectiveIndex, MapID, WorldMapAreaId, Floor, Priority, Flags \
         FROM content_quest_poi ORDER BY QuestID, id",
    )?;
    let mut pois: HashMap<u32, Vec<QuestPoiSnapshot>> = HashMap::new();
    let rows = statement.query_map([], |row| {
        let quest_id: u32 = row.get(0)?;
        let poi_id: u32 = row.get(1)?;
        Ok((
            quest_id,
            poi_id,
            QuestPoiSnapshot {
                objective_index: row.get(2)?,
                map_id: row.get(3)?,
                world_map_area_id: row.get(4)?,
                floor: row.get(5)?,
                priority: row.get(6)?,
                flags: row.get(7)?,
                points: Vec::new(),
            },
        ))
    })?;
    for row in rows {
        let (quest_id, poi_id, mut poi) = row?;
        poi.points = points.remove(&(quest_id, poi_id)).unwrap_or_default();
        pois.entry(quest_id).or_default().push(poi);
    }
    Ok(pois)
}

fn link_quests(db: &mut QuestDb, links: &[(u32, QuestLinks)]) {
    for (quest_id, link) in links {
        if link.prev_quest != 0 {
            db.prev_quests
                .entry(*quest_id)
                .or_default()
                .push(link.prev_quest);
        }
        if link.next_quest != 0 {
            let signed = if link.next_quest < 0 {
                -(*quest_id as i32)
            } else {
                *quest_id as i32
            };
            let next = link.next_quest.unsigned_abs();
            db.prev_quests.entry(next).or_default().push(signed);
        }
    }
    for quest in db.quests.values() {
        if quest.reward_next_quest != 0 {
            db.prev_chain_quests
                .entry(quest.reward_next_quest)
                .or_default()
                .push(quest.id);
        }
        if quest.exclusive_group != 0 {
            db.exclusive_groups
                .entry(quest.exclusive_group)
                .or_default()
                .push(quest.id);
        }
    }
}

/// Names objective text and granted items are built from.
struct ContentNames {
    creatures: HashMap<u32, String>,
    items: HashMap<u32, QuestItemInfo>,
}

#[cfg(test)]
#[test]
#[ignore = "requires WORLD_POPULATION_DB pointing to a promoted private copy"]
fn zone_population_real_copy_loads_quests_givers_and_objectives() {
    let path = std::env::var("WORLD_POPULATION_DB").expect("promoted copy path");
    let db = load_quest_db(Path::new(&path)).expect("promoted quest content loads");
    for (id, title, giver) in [
        (36626, "Host Howell", 86065),
        (48677, "Wicker Worship", 127296),
        (82747, "Crossroads Plaza", 224544),
    ] {
        let quest = db.quests.get(&id).expect("new zone quest");
        assert_eq!(quest.title, title);
        assert!(db.starters[&giver].contains(&id));
        assert!(!quest.completion_blocked);
    }
    let wolves = &db.quests[&47947];
    assert_eq!(wolves.title, "Big Bad Wolves");
    assert!(wolves.objectives.iter().any(|o| {
        o.kind == ObjectiveKind::Monster && o.object_id == 124814 && o.required == 10
    }));
}

pub(crate) fn load_quest_db(path: &Path) -> rusqlite::Result<QuestDb> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let names = ContentNames {
        creatures: load_creature_names(&conn)?,
        items: load_all_items(&conn)?,
    };
    let mut statement = conn.prepare(QUEST_QUERY)?;
    let rows = statement.query_map([], |row| quest_from_row(row, &names))?;
    let mut db = QuestDb::default();
    let mut links = Vec::new();
    for row in rows {
        let (quest, link) = row?;
        links.push((quest.id, link));
        db.quests.insert(quest.id, quest);
    }
    crate::quest_bonus_money::load(&path.with_file_name("quest-reward-bonus.json"), &mut db)?;
    let knowledge_rewards =
        crate::profession::specializations::load_knowledge_quest_rewards(&conn)?;
    for (quest_id, rewards) in knowledge_rewards {
        if let Some(quest) = db.quests.get_mut(&quest_id) {
            quest.profession_knowledge_rewards = rewards;
        }
    }
    load_reconstruction_metadata(&conn, &mut db)?;
    crate::world_quest_objectives::load(&conn, &mut db)?;
    crate::housing_quests::load(&conn, &mut db)?;
    link_quests(&mut db, &links);
    db.starters = load_relations(&conn, "content_creature_queststarter")?;
    db.enders = load_relations(&conn, "content_creature_questender")?;
    db.items = names.items;
    db.pois = load_pois(&conn)?;
    db.money_rewards = load_money_rewards(&conn)?;
    db.area_triggers = load_area_triggers(&conn)?;
    db.kill_credits = load_kill_credits(&conn)?;
    db.world_quests = load_world_quest_definitions(&conn, &db)?;
    db.world_quest_areas = crate::world_quests::load_areas(&conn)?;
    crate::world_quest_rewards::load(&conn, &mut db)?;
    Ok(db)
}

fn load_world_quest_definitions(
    conn: &Connection,
    db: &QuestDb,
) -> rusqlite::Result<Vec<crate::world_quests::Definition>> {
    let definitions = crate::world_quests::load_definitions(conn)?;
    for definition in &definitions {
        let has_world_quest_template = db
            .quests
            .get(&definition.quest_id)
            .is_some_and(|quest| quest.quest_info_modifiers & QUEST_INFO_MODIFIER_WORLD_QUEST != 0);
        if !has_world_quest_template {
            error!(
                "World quest {} lacks a loaded authentic world-quest template",
                definition.quest_id
            );
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    Ok(definitions)
}

/// Optional source overlay metadata; base Retail/Wrath rows retain their existing rules.
fn load_reconstruction_metadata(conn: &Connection, db: &mut QuestDb) -> rusqlite::Result<()> {
    let present: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='zephras_quest_meta')",
        [], |row| row.get(0),
    )?;
    if !present {
        return Ok(());
    }
    let mut statement = conn.prepare(
        "SELECT quest_id, reward_xp, completion_blocked, race_mask_playable FROM zephras_quest_meta",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, Option<u32>>(1)?,
            row.get::<_, bool>(2)?,
            row.get::<_, bool>(3)?,
        ))
    })?;
    for row in rows {
        apply_reconstruction_metadata(db, row?)?;
    }
    Ok(())
}

fn apply_reconstruction_metadata(
    db: &mut QuestDb,
    (id, xp, blocked, playable): (u32, Option<u32>, bool, bool),
) -> rusqlite::Result<()> {
    let quest = db.quests.get_mut(&id).ok_or_else(|| {
        error!("Zephras metadata references unloaded quest {id}");
        rusqlite::Error::InvalidQuery
    })?;
    quest.captured_reward_xp = Some(xp.unwrap_or(0));
    quest.completion_blocked = blocked;
    quest.race_mask_playable = playable;
    Ok(())
}

const AREA_TRIGGER_QUERY: &str = "SELECT a.ID, a.ContinentID, a.Pos_0, a.Pos_1, a.Pos_2, \
     a.ShapeType, a.Radius, a.Box_length, a.Box_width, a.Box_height, a.Box_yaw, r.quest \
     FROM content_areatrigger_involvedrelation r JOIN area_trigger a ON a.ID = r.id \
     ORDER BY a.ID, r.quest";

/// `ObjectMgr::LoadQuestAreaTriggers`; a trigger of an unmodelled shape is skipped.
fn load_area_triggers(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<QuestAreaTrigger>>> {
    let mut statement = conn.prepare(AREA_TRIGGER_QUERY)?;
    let mut triggers: HashMap<u32, Vec<QuestAreaTrigger>> = HashMap::new();
    let rows = statement.query_map([], |row| {
        let float =
            |index: usize| -> rusqlite::Result<f32> { Ok(row.get::<_, f64>(index)? as f32) };
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            Vec3::new(float(2)?, float(3)?, float(4)?),
            AreaTriggerShape::from_db2(
                row.get(5)?,
                float(6)?,
                [float(7)?, float(8)?, float(9)?, float(10)?],
            ),
            row.get::<_, u32>(11)?,
        ))
    })?;
    for row in rows {
        let (id, map_id, center, shape, quest) = row?;
        let Some(shape) = shape else {
            warn!("Quest area trigger {id} has an unsupported shape; skipped");
            continue;
        };
        let on_map = triggers.entry(map_id).or_default();
        match on_map.last_mut() {
            Some(trigger) if trigger.id == id => trigger.quests.push(quest),
            _ => on_map.push(QuestAreaTrigger {
                id,
                center,
                shape,
                quests: vec![quest],
            }),
        }
    }
    Ok(triggers)
}

fn load_money_rewards(conn: &Connection) -> rusqlite::Result<HashMap<u8, [u32; 10]>> {
    let columns: Vec<String> = (0..10).map(|n| format!("Difficulty_{n}")).collect();
    let mut statement = conn.prepare(&format!(
        "SELECT ID, {} FROM quest_money_reward WHERE ID BETWEEN 0 AND 255",
        columns.join(", ")
    ))?;
    let rows = statement.query_map([], |row| {
        let mut money = [0; 10];
        for (index, value) in money.iter_mut().enumerate() {
            *value = row.get(index + 1)?;
        }
        Ok((row.get::<_, u8>(0)?, money))
    })?;
    rows.collect()
}

pub struct QuestDataPlugin;

impl Plugin for QuestDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_quest_db);
    }
}

fn startup_load_quest_db(mut commands: Commands) {
    let path = Path::new(crate::spell_info::WORLD_DB_PATH);
    let quests = load_quest_db(path)
        .unwrap_or_else(|error| panic!("load quests from {}: {error}", path.display()));
    info!("Loaded {} quests", quests.quests.len());
    commands.insert_resource(quests);
}

/// Quest content from the real `data/world.db`, loaded once; `None` when absent.
#[cfg(test)]
pub(crate) fn test_quest_db() -> Option<&'static QuestDb> {
    static DB: std::sync::OnceLock<Option<QuestDb>> = std::sync::OnceLock::new();
    DB.get_or_init(|| {
        let path = crate::test_data::world_db_path();
        Some(load_quest_db(&path).expect("load quests"))
    })
    .as_ref()
}
