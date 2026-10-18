//! Protocol views of quest state and the bag edits quests make.
use std::collections::BTreeMap;

use redb::Database;
use shared::item_data::ItemDefinitionSource::Retail;
use shared::protocol::{
    QuestEntrySnapshot, QuestGiverQuestDetails, QuestLogSnapshot, QuestLogUpdate,
    QuestObjectiveSnapshot, QuestRewardItem, QuestRewards,
};

use crate::persistence::inventory_store::load_item_definition_source;
use crate::persistence::quest_store::{ActiveQuestData, QuestLogData};
use crate::persistence::{self, InventoryData, InventoryItemData};
use crate::quest_data::{ObjectiveKind, QuestDb, QuestTemplate};
use crate::quest_rules::{QuestPlayer, objective_counts, objectives_done, quest_money};

const QUEST_FLAGS_SHARABLE: u32 = 0x8;

/// Quest log entries by quest id, as last sent to a client.
pub(crate) type QuestLogView = BTreeMap<u32, QuestEntrySnapshot>;

/// Retail quest requirements never count a same-ID Forever GUID.
pub(crate) fn bag_count(db: &Database, inventory: &InventoryData, item_id: u32) -> u32 {
    inventory
        .items
        .iter()
        .filter(|item| {
            item.item_id == item_id && load_item_definition_source(db, item.item_guid) == Retail
        })
        .map(|item| item.stack_count)
        .sum()
}

/// Remove up to `count` of a Retail quest item, stacks first-to-last.
pub(crate) fn remove_items(
    db: &Database,
    inventory: &mut InventoryData,
    item_id: u32,
    mut count: u32,
) {
    for item in inventory.items.iter_mut().filter(|item| {
        item.item_id == item_id && load_item_definition_source(db, item.item_guid) == Retail
    }) {
        let taken = item.stack_count.min(count);
        item.stack_count -= taken;
        count -= taken;
    }
    inventory.items.retain(|item| item.stack_count > 0);
}

/// Quest starters/rewards use the same ownership and storage checks as other creation.
pub(crate) fn grant_item(
    inventory: &mut crate::inventory::inventory_model::InventoryModel,
    db: &Database,
    quests: &QuestDb,
    templates: &crate::item_templates::ItemTemplates,
    item_id: u32,
    count: u32,
) -> Result<(), shared::protocol::InventoryError> {
    let missing = || {
        crate::inventory::inventory_model::error(
            shared::protocol::InventoryErrorReason::ItemNotFound,
            format!("quest item {item_id} has no Retail definition"),
        )
    };
    let info = quests.items.get(&item_id).ok_or_else(missing)?;
    let template = *templates.get(Retail, item_id).ok_or_else(missing)?;
    let item = InventoryItemData {
        item_guid: persistence::next_id(db, "item_instance"),
        item_id,
        name: info.name.clone(),
        quality: info.quality,
        required_level: info.required_level,
        stack_count: count,
        vendor_sell_price: info.sell_price,
        unique_auction: false,
    };
    inventory.store_item(item, Retail, template, &mut || {
        persistence::next_id(db, "item_instance")
    })
}

/// The objectives the protocol has a kind for; the exploration/event requirement only
/// shows through the entry's `completed`.
pub(crate) fn objective_snapshots(
    quest: &QuestTemplate,
    counts: &[u32],
) -> Vec<QuestObjectiveSnapshot> {
    quest
        .objectives
        .iter()
        .zip(counts)
        .enumerate()
        .filter_map(|(index, (objective, &current))| {
            if !crate::world_quest_objectives::visible(quest, counts, index) {
                return None;
            }
            Some(QuestObjectiveSnapshot {
                text: objective.text.clone(),
                current,
                required: objective.required,
                completed: current >= objective.required,
                optional: crate::world_quest_objectives::rule(quest, index).optional(),
                kind: objective.kind.protocol()?,
                object_id: objective.object_id,
            })
        })
        .collect()
}

fn quest_entry(
    db: &Database,
    quests: &QuestDb,
    quest: &QuestTemplate,
    active: &ActiveQuestData,
    watched: bool,
    inventory: &InventoryData,
    player: QuestPlayer,
) -> QuestEntrySnapshot {
    let counts = objective_counts(quest, active, |item_id| bag_count(db, inventory, item_id));
    QuestEntrySnapshot {
        quest_id: quest.id,
        title: quest.title.clone(),
        zone: String::new(),
        completed: !active.failed && objectives_done(quest, &counts),
        expires_at: active.expires_at,
        timer_sampled_at: active.expires_at.map(|_| {
            quests
                .now
                .unwrap_or_else(crate::persistence::quest_store::unix_now)
        }),
        failed: active.failed,
        repeatability: quest.repeatability(),
        objectives: objective_snapshots(quest, &counts),
        level: quest.level,
        sort_id: quest.sort_id,
        objectives_text: quest.objectives_text.clone(),
        description: quest.description.clone(),
        rewards: quest_rewards(quests, quest, player),
        pushable: quest.flags & QUEST_FLAGS_SHARABLE != 0 && !active.failed,
        completion_text: quest.completion_log.clone(),
        watched,
        pois: quests.pois.get(&quest.id).cloned().unwrap_or_default(),
    }
}

/// Every quest in the log, in log order.
pub(crate) fn log_entries(
    db: &Database,
    quests: &QuestDb,
    log: &QuestLogData,
    inventory: &InventoryData,
    player: QuestPlayer,
) -> Vec<QuestEntrySnapshot> {
    log.active
        .iter()
        .filter_map(|active| {
            let quest = quests.quests.get(&active.quest_id)?;
            let watched = log.watched.contains(&active.quest_id);
            Some(quest_entry(
                db, quests, quest, active, watched, inventory, player,
            ))
        })
        .collect()
}

pub(crate) fn log_snapshot(
    db: &Database,
    quests: &QuestDb,
    log: &QuestLogData,
    inventory: &InventoryData,
    player: QuestPlayer,
) -> QuestLogSnapshot {
    QuestLogSnapshot {
        entries: log_entries(db, quests, log, inventory, player),
        watched_quest_ids: log.watched.clone(),
    }
}

pub(crate) fn view_of(entries: &[QuestEntrySnapshot]) -> QuestLogView {
    entries
        .iter()
        .map(|entry| (entry.quest_id, entry.clone()))
        .collect()
}

/// Entries that were added or changed and ids that left, or `None` when the
/// log and watch list are unchanged.
pub(crate) fn diff_log(
    old: &QuestLogView,
    old_watched: &[u32],
    new: &QuestLogView,
    new_watched: &[u32],
) -> Option<QuestLogUpdate> {
    let changed: Vec<_> = new
        .iter()
        .filter(|(id, entry)| old.get(id) != Some(entry))
        .map(|(_, entry)| entry.clone())
        .collect();
    let removed: Vec<_> = old
        .keys()
        .filter(|id| !new.contains_key(id))
        .copied()
        .collect();
    if changed.is_empty() && removed.is_empty() && old_watched == new_watched {
        return None;
    }
    Some(QuestLogUpdate {
        changed,
        removed,
        watched_quest_ids: new_watched.to_vec(),
    })
}

fn reward_items(quests: &QuestDb, items: &[(u32, u32)]) -> Vec<QuestRewardItem> {
    items
        .iter()
        .map(|&(item_id, count)| QuestRewardItem {
            item_id,
            name: quests.item_name(item_id),
            count,
        })
        .collect()
}

pub(crate) fn quest_rewards(
    quests: &QuestDb,
    quest: &QuestTemplate,
    player: QuestPlayer,
) -> QuestRewards {
    QuestRewards {
        money: quest_money(quests, quest, player).max(0) as u32,
        items: reward_items(quests, &quest.reward_items),
        choice_items: reward_items(quests, &quest.choice_items),
    }
}

/// Item objectives as `QuestRewardItem`s (the progress dialog's item list).
pub(crate) fn required_items(quests: &QuestDb, quest: &QuestTemplate) -> Vec<QuestRewardItem> {
    let items: Vec<(u32, u32)> = quest
        .objectives
        .iter()
        .filter(|objective| objective.kind == ObjectiveKind::Item)
        .map(|objective| (objective.object_id, objective.required))
        .collect();
    reward_items(quests, &items)
}

pub(crate) fn quest_details(
    quests: &QuestDb,
    quest: &QuestTemplate,
    npc: u64,
    player: QuestPlayer,
) -> QuestGiverQuestDetails {
    QuestGiverQuestDetails {
        npc,
        quest_id: quest.id,
        title: quest.title.clone(),
        description: quest.description.clone(),
        objectives_text: quest.objectives_text.clone(),
        level: quest.level,
        min_level: quest.min_level,
        suggested_group: quest.suggested_group,
        objectives: objective_snapshots(quest, &vec![0; quest.objectives.len()]),
        rewards: quest_rewards(quests, quest, player),
    }
}
