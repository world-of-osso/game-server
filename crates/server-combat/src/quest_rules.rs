//! Pure quest rules over `QuestDb` + a character's `QuestLogData`, following
//! AzerothCore `Player::CanTakeQuest` / `CanSeeStartQuest` /
//! `GetQuestDialogStatus` / `KilledMonsterCredit`.
use shared::protocol::{
    MAX_QUEST_LOG_SIZE, QuestFailedReason, QuestGiverQuestEntry, QuestGiverQuestState,
    QuestGiverStatus, QuestMarkerClass,
};

use crate::persistence::quest_store::{ActiveQuestData, QuestLogData};
use crate::quest_data::{ObjectiveKind, QuestDb, QuestMoneyScaling, QuestTemplate};

/// Quests this many levels below the player are trivial
/// (`CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF` default).
const LOW_LEVEL_HIDE_DIFF: i32 = 4;
/// Quests up to this many levels above the player show a grey `!`
/// (`CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF` default).
const HIGH_LEVEL_HIDE_DIFF: i32 = 7;

/// Character attributes quest checks read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuestPlayer {
    pub level: u8,
    pub race: u8,
    pub class: u8,
}

/// Player::GetQuestLevel (Player.cpp:14339-14353) of a TrinityCore quest: the
/// player's level clamped to the quest's ContentTuning range. The minimum is
/// GetQuestMinLevel (:14315-14329): `MinLevelWithDelta`, or the maximum for a
/// player outside the quest's scaling faction group.
fn tuned_quest_level(scaling: &QuestMoneyScaling, player: QuestPlayer) -> u8 {
    let faction_group = crate::faction::faction_group_for_race(player.race);
    let min_level = if scaling.faction_group != 0 && faction_group != scaling.faction_group {
        scaling.max_level
    } else {
        scaling.min_level
    };
    if player.level >= min_level {
        player.level.min(scaling.max_level)
    } else {
        min_level
    }
}

/// Copper the quest pays this player: Quest::MoneyValue (QuestDef.cpp:478-484),
/// `QuestMoneyReward[GetQuestLevel].Difficulty[RewardMoneyDifficulty] ×
/// RewardMoneyMultiplier` truncated, 0 without a row; AzerothCore quests pay
/// their fixed `RewardMoney` (negative costs money).
pub(crate) fn quest_money(quests: &QuestDb, quest: &QuestTemplate, player: QuestPlayer) -> i32 {
    let Some(scaling) = &quest.money_scaling else {
        return quest.reward_money;
    };
    quests
        .money_rewards
        .get(&tuned_quest_level(scaling, player))
        .and_then(|money| money.get(usize::from(scaling.difficulty)))
        .map_or(0, |&copper| (copper as f32 * scaling.multiplier) as i32)
}

/// A quest's level for this player: Player::GetQuestLevel's ContentTuning clamp for
/// TrinityCore quests, else the AzerothCore level (-1 scales to the player).
pub(crate) fn quest_level(quest: &QuestTemplate, player: QuestPlayer) -> i32 {
    if let Some(scaling) = &quest.money_scaling {
        i32::from(tuned_quest_level(scaling, player))
    } else if quest.level < 0 {
        i32::from(player.level)
    } else {
        quest.level
    }
}

fn mask_allows(mask: u32, id: u8) -> bool {
    mask == 0
        || id
            .checked_sub(1)
            .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
            .is_some_and(|bit| mask & bit != 0)
}

fn race_mask_allows(quest: &QuestTemplate, race: u8) -> bool {
    if quest.allowable_races == 0 {
        return true;
    }
    let shift = if quest.race_mask_playable {
        crate::race_class::race_bit(race)
    } else {
        race.checked_sub(1)
    };
    shift
        .and_then(|shift| 1_u64.checked_shl(u32::from(shift)))
        .is_some_and(|bit| quest.allowable_races & bit != 0)
}

/// `SatisfyQuestExclusiveGroup`: no other quest of a positive group was taken.
fn exclusive_group_ok(db: &QuestDb, log: &QuestLogData, quest: &QuestTemplate) -> bool {
    if quest.exclusive_group <= 0 {
        return true;
    }
    db.exclusive_groups[&quest.exclusive_group]
        .iter()
        .filter(|&&id| id != quest.id)
        .all(|&id| {
            log.active(id).is_none()
                && db
                    .quests
                    .get(&id)
                    .is_none_or(|other| !reward_locked(db, log, other))
        })
}

/// Every quest of a negative ("all of") exclusive group other than `quest_id`
/// is turned in.
fn whole_group_rewarded(db: &QuestDb, log: &QuestLogData, group: i32, quest_id: u32) -> bool {
    group >= 0
        || db.exclusive_groups[&group]
            .iter()
            .all(|&id| id == quest_id || log.is_rewarded(id))
}

/// `SatisfyQuestPreviousQuest`: any positive previous quest turned in, or any
/// negative one in the log.
fn previous_quest_ok(db: &QuestDb, log: &QuestLogData, quest: &QuestTemplate) -> bool {
    let Some(prev_quests) = db.prev_quests.get(&quest.id) else {
        return true;
    };
    prev_quests.iter().any(|&signed| {
        let prev_id = signed.unsigned_abs();
        let Some(prev) = db.quests.get(&prev_id) else {
            return false;
        };
        let taken = if signed > 0 {
            log.is_rewarded(prev_id)
        } else {
            log.active(prev_id).is_some()
        };
        taken && whole_group_rewarded(db, log, prev.exclusive_group, prev_id)
    })
}

/// `SatisfyQuestNextChain` + `SatisfyQuestPrevChain`: the chain's next quest was
/// not taken, and no quest leading into this one is still in the log.
fn chain_ok(db: &QuestDb, log: &QuestLogData, quest: &QuestTemplate) -> bool {
    let next_taken = quest.reward_next_quest != 0
        && (log.active(quest.reward_next_quest).is_some()
            || log.is_rewarded(quest.reward_next_quest));
    let prev_active = db
        .prev_chain_quests
        .get(&quest.id)
        .is_some_and(|prevs| prevs.iter().any(|&id| log.active(id).is_some()));
    !next_taken && !prev_active
}

fn reward_locked(db: &QuestDb, log: &QuestLogData, quest: &QuestTemplate) -> bool {
    if quest.quest_info_modifiers & 0x100 != 0 {
        return db.world_quest_windows.get(&quest.id).is_none_or(|window| {
            log.completed_at
                .get(&quest.id)
                .is_some_and(|&time| time >= window.starts_at)
        });
    }
    if !log.is_rewarded(quest.id) || quest.is_repeatable() {
        return false;
    }
    let interval = match quest.repeatability() {
        shared::protocol::QuestRepeatability::Normal => return true,
        shared::protocol::QuestRepeatability::Daily => crate::maps::store::ResetInterval::Daily,
        shared::protocol::QuestRepeatability::Weekly => crate::maps::store::ResetInterval::Weekly,
    };
    let now = db
        .now
        .unwrap_or_else(crate::persistence::quest_store::unix_now);
    log.completed_at
        .get(&quest.id)
        .is_some_and(|&completed| now < crate::maps::locks::next_reset_time(interval, completed))
}

/// Everything `CanTakeQuest` checks except level.
fn requirements(
    db: &QuestDb,
    log: &QuestLogData,
    player: QuestPlayer,
    quest: &QuestTemplate,
) -> Result<(), QuestFailedReason> {
    if log.active(quest.id).is_some() {
        return Err(QuestFailedReason::AlreadyOn);
    }
    if reward_locked(db, log, quest) {
        return Err(QuestFailedReason::AlreadyDone);
    }
    if !mask_allows(quest.allowable_classes, player.class) {
        return Err(QuestFailedReason::WrongClass);
    }
    if !race_mask_allows(quest, player.race) {
        return Err(QuestFailedReason::WrongRace);
    }
    let linked = exclusive_group_ok(db, log, quest)
        && previous_quest_ok(db, log, quest)
        && chain_ok(db, log, quest);
    if !linked {
        return Err(QuestFailedReason::DontHaveRequirement);
    }
    Ok(())
}

/// `SatisfyQuestLevel`.
fn level_ok(player: QuestPlayer, quest: &QuestTemplate) -> Result<(), QuestFailedReason> {
    if player.level < quest.min_level {
        return Err(QuestFailedReason::LowLevel);
    }
    if quest.max_level > 0 && player.level > quest.max_level {
        return Err(QuestFailedReason::HighLevel);
    }
    Ok(())
}

/// `CanTakeQuest` + `CanAddQuest` (log size).
pub(crate) fn can_take(
    db: &QuestDb,
    log: &QuestLogData,
    player: QuestPlayer,
    quest: &QuestTemplate,
) -> Result<(), QuestFailedReason> {
    requirements(db, log, player, quest)?;
    level_ok(player, quest)?;
    if quest.time_allowed != 0
        && log.active.iter().any(|active| {
            !active.failed
                && db
                    .quests
                    .get(&active.quest_id)
                    .is_some_and(|template| template.time_allowed > 0)
        })
    {
        return Err(QuestFailedReason::OnlyOneTimedQuest);
    }
    if log.active.len() >= MAX_QUEST_LOG_SIZE {
        return Err(QuestFailedReason::QuestLogFull);
    }
    Ok(())
}

/// Objective progress: stored counts for kills, bag counts for items, capped
/// at the requirement. `bag_count` returns how many of an item the bags hold.
pub(crate) fn objective_counts(
    quest: &QuestTemplate,
    active: &ActiveQuestData,
    bag_count: impl Fn(u32) -> u32,
) -> Vec<u32> {
    let mut counts: Vec<_> = quest
        .objectives
        .iter()
        .enumerate()
        .map(|(index, objective)| {
            let current = match objective.kind {
                ObjectiveKind::Item
                    if !crate::world_quest_objectives::rule(quest, index).quest_bound_item() =>
                {
                    bag_count(objective.object_id)
                }
                _ => active.counts.get(index).copied().unwrap_or(0),
            };
            current.min(objective.required)
        })
        .collect();
    crate::world_quest_objectives::derive_progress_bars(quest, &mut counts);
    counts
}

pub(crate) fn objectives_done(quest: &QuestTemplate, counts: &[u32]) -> bool {
    if quest.completion_blocked {
        return false;
    }
    quest
        .objectives
        .iter()
        .zip(counts)
        .enumerate()
        .all(|(index, (objective, &count))| {
            !crate::world_quest_objectives::rule(quest, index).required()
                || count >= objective.required
        })
}

/// TrinityCore `.quest complete` for the counted objectives: every kill and object
/// objective of `quest_id` in the log reaches its requirement; item objectives stay
/// bag counts. Errs when the quest is not in the log.
pub(crate) fn complete_counted_objectives(
    db: &QuestDb,
    log: &mut QuestLogData,
    quest_id: u32,
) -> Result<(), String> {
    let quest = db
        .quests
        .get(&quest_id)
        .ok_or_else(|| format!("quest {quest_id} unknown"))?;
    let active = log
        .active
        .iter_mut()
        .find(|active| active.quest_id == quest_id)
        .ok_or_else(|| format!("quest {quest_id} is not in the log"))?;
    active.counts.resize(quest.objectives.len(), 0);
    for (objective, count) in quest.objectives.iter().zip(&mut active.counts) {
        if objective.kind != ObjectiveKind::Item {
            *count = objective.required;
        }
    }
    Ok(())
}

/// `Player::KilledMonster`: kill credit for the creature's entry, then for each of its
/// `KillCredit1`/`KillCredit2` entries (Player.cpp:16652-16662). Returns whether anything
/// changed.
pub(crate) fn credit_killed_creature(
    db: &QuestDb,
    log: &mut QuestLogData,
    creature_entry: u32,
) -> bool {
    let credits = db
        .kill_credits
        .get(&creature_entry)
        .map_or(&[][..], Vec::as_slice);
    std::iter::once(creature_entry)
        .chain(credits.iter().copied())
        .fold(false, |changed, entry| {
            credit_kill(db, log, entry) | changed
        })
}

/// `KilledMonsterCredit`: add one kill to every unfinished objective of a quest
/// in the log that counts `creature_entry`. Returns whether anything changed.
pub(crate) fn credit_kill(db: &QuestDb, log: &mut QuestLogData, creature_entry: u32) -> bool {
    credit_objective(db, log, ObjectiveKind::Monster, creature_entry)
}

/// An authored event advances every matching counted objective, never another Type.
pub(crate) fn credit_objective(
    db: &QuestDb,
    log: &mut QuestLogData,
    kind: ObjectiveKind,
    object_id: u32,
) -> bool {
    let mut changed = false;
    for active in &mut log.active {
        let Some(quest) = db.quests.get(&active.quest_id) else {
            continue;
        };
        active.counts.resize(quest.objectives.len(), 0);
        for (index, objective) in quest.objectives.iter().enumerate() {
            let counted_match = objective.kind == kind && objective.object_id == object_id;
            let criterion_match = kind == ObjectiveKind::Monster
                && objective.kind == ObjectiveKind::CriteriaTree
                && db.task_criteria.get(&objective.object_id) == Some(&(0, object_id));
            if (counted_match || criterion_match)
                && active.counts[index] < objective.required
                && crate::world_quest_objectives::creditable(quest, &active.counts, index)
            {
                active.counts[index] += 1;
                changed = true;
            }
        }
    }
    changed
}

/// Pinned KilledPlayerCredit: personal counter; flagged objectives require the victim's effective team.
pub(crate) fn credit_player_kill(db: &QuestDb, log: &mut QuestLogData, same_team: bool) -> bool {
    const SAME_FACTION: u32 = 0x80;
    let mut changed = false;
    for active in &mut log.active {
        if active.failed
            || active
                .expires_at
                .is_some_and(|end| db.now.is_some_and(|now| now >= end))
        {
            continue;
        }
        let Some(quest) = db.quests.get(&active.quest_id) else {
            continue;
        };
        active.counts.resize(quest.objectives.len(), 0);
        for (index, objective) in quest.objectives.iter().enumerate() {
            let flags = crate::world_quest_objectives::rule(quest, index).flags;
            let team_allowed = flags & SAME_FACTION == 0 || same_team;
            if objective.kind == ObjectiveKind::PlayerKills
                && team_allowed
                && active.counts[index] < objective.required
                && crate::world_quest_objectives::creditable(quest, &active.counts, index)
            {
                active.counts[index] += 1;
                changed = true;
            }
        }
    }
    changed
}

/// `WorldSession::HandleAreaTriggerOpcode` (MiscHandler.cpp:521-558) for an entered
/// trigger: every quest in the log the trigger explores (`GetQuestsForAreaTrigger`)
/// whose exploration is not done yet gets it (`AreaExploredOrEventHappens`). Returns
/// whether anything changed.
pub(crate) fn credit_area_trigger(db: &QuestDb, log: &mut QuestLogData, quests: &[u32]) -> bool {
    let mut changed = false;
    for active in log
        .active
        .iter_mut()
        .filter(|active| quests.contains(&active.quest_id))
    {
        let Some(quest) = db.quests.get(&active.quest_id) else {
            continue;
        };
        active.counts.resize(quest.objectives.len(), 0);
        for (objective, count) in quest.objectives.iter().zip(&mut active.counts) {
            if objective.kind == ObjectiveKind::ExplorationOrEvent && *count < objective.required {
                *count = objective.required;
                changed = true;
            }
        }
    }
    changed
}

/// Pinned ItemAddedQuestCheck before StoreNewItem: authored bound objectives
/// store credit. Only a single updated non-source objective suppresses creation.
pub(crate) fn credit_bound_item(
    db: &QuestDb,
    log: &mut QuestLogData,
    item_id: u32,
    amount: u32,
) -> (bool, bool) {
    let mut updated = Vec::new();
    for active in &mut log.active {
        let Some(quest) = db.quests.get(&active.quest_id) else {
            continue;
        };
        if active.failed
            || active
                .expires_at
                .is_some_and(|end| db.now.is_some_and(|now| now >= end))
        {
            continue;
        }
        active.counts.resize(quest.objectives.len(), 0);
        for (index, objective) in quest.objectives.iter().enumerate() {
            let matches = objective.kind == ObjectiveKind::Item && objective.object_id == item_id;
            let rule = crate::world_quest_objectives::rule(quest, index);
            if !matches || !rule.quest_bound_item() || active.counts[index] >= objective.required {
                continue;
            }
            if !crate::world_quest_objectives::creditable(quest, &active.counts, index) {
                continue;
            }
            active.counts[index] = active.counts[index]
                .saturating_add(amount)
                .min(objective.required);
            updated.push(quest.start_item != item_id);
        }
    }
    (!updated.is_empty(), updated.as_slice() == [true])
}

/// AzerothCore `Player::HasQuestForItem`: a quest in the log still needs more
/// of `item_id` for one of its item objectives.
pub(crate) fn quest_needs_item(
    db: &QuestDb,
    log: &QuestLogData,
    item_id: u32,
    bag_count: impl Fn(u32) -> u32,
) -> bool {
    log.active.iter().any(|active| {
        db.quests.get(&active.quest_id).is_some_and(|quest| {
            let counts = objective_counts(quest, active, &bag_count);
            quest
                .objectives
                .iter()
                .enumerate()
                .any(|(index, objective)| {
                    objective.kind == ObjectiveKind::Item
                        && objective.object_id == item_id
                        && counts[index] < objective.required
                        && crate::world_quest_objectives::creditable(quest, &counts, index)
                })
        })
    })
}

/// Is the quest in the log with every objective done.
pub(crate) fn is_complete_in_log(
    db: &QuestDb,
    log: &QuestLogData,
    quest_id: u32,
    bag_count: impl Fn(u32) -> u32,
) -> bool {
    let (Some(quest), Some(active)) = (db.quests.get(&quest_id), log.active(quest_id)) else {
        return false;
    };
    !active.failed
        && active.expires_at.is_none_or(|end| {
            end > db
                .now
                .unwrap_or_else(crate::persistence::quest_store::unix_now)
        })
        && objectives_done(quest, &objective_counts(quest, active, bag_count))
}

/// How a starter quest appears to this player, if at all
/// (`CanSeeStartQuest` then `SatisfyQuestLevel` and the trivial check,
/// TrinityCore Player.cpp:16221-16250).
fn start_state(
    db: &QuestDb,
    log: &QuestLogData,
    player: QuestPlayer,
    quest: &QuestTemplate,
) -> Option<QuestGiverStatus> {
    requirements(db, log, player, quest).ok()?;
    if i32::from(player.level) + HIGH_LEVEL_HIDE_DIFF < i32::from(quest.min_level) {
        return None;
    }
    let class = quest.marker_class();
    if level_ok(player, quest).is_err() {
        // Only important, legendary and campaign quests have future kinds.
        return Some(QuestGiverStatus::Future(match class {
            QuestMarkerClass::Important
            | QuestMarkerClass::Legendary
            | QuestMarkerClass::Campaign => class,
            _ => QuestMarkerClass::Normal,
        }));
    }
    // Covenant calling quests have no trivial kind.
    if class != QuestMarkerClass::Calling
        && i32::from(player.level) > quest_level(quest, player) + LOW_LEVEL_HIDE_DIFF
    {
        Some(QuestGiverStatus::Trivial(class))
    } else {
        Some(QuestGiverStatus::Available(class))
    }
}

fn end_state(
    db: &QuestDb,
    log: &QuestLogData,
    quest_id: u32,
    bag_count: &impl Fn(u32) -> u32,
) -> Option<QuestGiverQuestState> {
    log.active(quest_id)?;
    if is_complete_in_log(db, log, quest_id, bag_count) {
        Some(QuestGiverQuestState::Complete)
    } else {
        Some(QuestGiverQuestState::Incomplete)
    }
}

fn quests_of(relations: &std::collections::HashMap<u32, Vec<u32>>, entry: u32) -> &[u32] {
    relations.get(&entry).map_or(&[], Vec::as_slice)
}

/// Marker over a creature for this player: the highest status among the
/// quests it ends (reward / incomplete) and starts.
pub(crate) fn giver_status(
    db: &QuestDb,
    log: &QuestLogData,
    player: QuestPlayer,
    creature_entry: u32,
    bag_count: impl Fn(u32) -> u32,
) -> QuestGiverStatus {
    let ending = quests_of(&db.enders, creature_entry)
        .iter()
        .filter_map(|&id| Some((db.quests.get(&id)?, end_state(db, log, id, &bag_count)?)))
        .map(|(quest, state)| match state {
            QuestGiverQuestState::Complete => QuestGiverStatus::Reward(quest.marker_class()),
            _ => QuestGiverStatus::Incomplete(quest.marker_class()),
        });
    let starting = quests_of(&db.starters, creature_entry)
        .iter()
        .filter_map(|id| db.quests.get(id))
        .filter_map(|quest| start_state(db, log, player, quest));
    ending
        .chain(starting)
        .max()
        .unwrap_or(QuestGiverStatus::None)
}

fn list_entry(quest: &QuestTemplate, state: QuestGiverQuestState) -> QuestGiverQuestEntry {
    QuestGiverQuestEntry {
        quest_id: quest.id,
        title: quest.title.clone(),
        level: quest.level,
        state,
    }
}

/// `PrepareQuestMenu`: quests in the log this creature ends, then quests it
/// starts that the player can take now.
pub(crate) fn giver_quest_list(
    db: &QuestDb,
    log: &QuestLogData,
    player: QuestPlayer,
    creature_entry: u32,
    bag_count: impl Fn(u32) -> u32,
) -> Vec<QuestGiverQuestEntry> {
    let ending = quests_of(&db.enders, creature_entry)
        .iter()
        .filter_map(|&id| Some((db.quests.get(&id)?, end_state(db, log, id, &bag_count)?)))
        .map(|(quest, state)| list_entry(quest, state));
    let starting = quests_of(&db.starters, creature_entry)
        .iter()
        .filter_map(|id| db.quests.get(id))
        .filter_map(|quest| match start_state(db, log, player, quest)? {
            QuestGiverStatus::Available(_) => {
                Some(list_entry(quest, QuestGiverQuestState::Available))
            }
            QuestGiverStatus::Trivial(_) => {
                Some(list_entry(quest, QuestGiverQuestState::LowLevelAvailable))
            }
            _ => None,
        });
    ending.chain(starting).collect()
}

pub(crate) fn starts(db: &QuestDb, creature_entry: u32, quest_id: u32) -> bool {
    quests_of(&db.starters, creature_entry).contains(&quest_id)
}

pub(crate) fn ends(db: &QuestDb, creature_entry: u32, quest_id: u32) -> bool {
    quests_of(&db.enders, creature_entry).contains(&quest_id)
}

#[cfg(test)]
#[path = "quest_rules_tests.rs"]
mod tests;
