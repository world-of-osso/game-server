//! Quest definitions: objectives, rewards, prerequisites.
//!
//! Static quest data loaded at startup. Runtime quest progress lives on
//! player components (not here).
//!
//! Ref: AzerothCore `QuestDef.h`, `quest_template` table.

use serde::{Deserialize, Serialize};

/// What a player must do to complete a quest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuestObjective {
    /// Kill N creatures of a specific template.
    Kill { creature_id: u32, count: u16 },
    /// Collect N items.
    Collect { item_id: u32, count: u16 },
    /// Interact with a specific game object or NPC.
    Interact { target_id: u32 },
    /// Escort an NPC to a destination (scripted event).
    Escort { npc_id: u32 },
    /// Reach a location (x, y, z within radius).
    ReachLocation { x: f32, y: f32, z: f32, radius: f32 },
}

/// A reward granted on quest completion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum QuestReward {
    /// XP reward (may be scaled by level).
    Xp(u32),
    /// Gold reward in copper.
    Gold(u32),
    /// A specific item (guaranteed).
    Item { item_id: u32, count: u16 },
    /// Reputation with a faction.
    Reputation { faction_id: u32, amount: i32 },
}

/// An item the player can choose from as a reward.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuestRewardChoice {
    pub item_id: u32,
    pub count: u16,
}

/// How often a quest can be repeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestRepeat {
    /// One-time quest, cannot be repeated.
    None,
    /// Resets daily (server time, typically 3 AM).
    Daily,
    /// Resets weekly (Tuesday for US, Wednesday for EU).
    Weekly,
}

/// Static quest definition.
///
/// Immutable data loaded at startup from the quest database.
/// Runtime state (accepted, in-progress, progress counts) lives on
/// the player entity.
///
/// Ref: AzerothCore `quest_template`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestData {
    /// Unique quest ID.
    pub id: u32,
    /// Quest title displayed in the log.
    pub title: String,
    /// Full description shown when accepting.
    pub description: String,
    /// Brief objective summary shown in the tracker.
    pub objective_text: String,
    /// What must be done to complete the quest.
    pub objectives: Vec<QuestObjective>,
    /// Guaranteed rewards on completion.
    pub rewards: Vec<QuestReward>,
    /// Items the player chooses one from (0 = no choice).
    pub reward_choices: Vec<QuestRewardChoice>,
    /// Minimum player level to accept.
    pub required_level: u8,
    /// Suggested level (for difficulty display).
    pub suggested_level: u8,
    /// Quest IDs that must be completed before this one is available.
    pub prerequisites: Vec<u32>,
    /// NPC ID that offers this quest.
    pub quest_giver: u32,
    /// NPC ID to turn in the quest (0 = same as giver).
    pub turn_in_npc: u32,
    /// Quest ID automatically offered on turn-in (chain continuation). 0 = none.
    pub next_quest_id: u32,
    /// Exclusive group ID. Completing any quest in a group blocks the others.
    /// 0 = no exclusive group.
    pub exclusive_group: u32,
    /// How often this quest can be repeated.
    pub repeat: QuestRepeat,
}

impl QuestData {
    /// Whether a player meets the level requirement.
    pub fn meets_level_req(&self, player_level: u8) -> bool {
        player_level >= self.required_level
    }

    /// Whether all prerequisite quests are completed.
    pub fn prerequisites_met(&self, completed_quests: &[u32]) -> bool {
        self.prerequisites
            .iter()
            .all(|req| completed_quests.contains(req))
    }

    /// Whether this quest is blocked by an exclusive group member being completed.
    pub fn excluded_by(&self, completed_quests: &[u32], all_quests: &[QuestData]) -> bool {
        if self.exclusive_group == 0 {
            return false;
        }
        all_quests.iter().any(|q| {
            q.id != self.id
                && q.exclusive_group == self.exclusive_group
                && completed_quests.contains(&q.id)
        })
    }

    /// Whether a player can accept this quest (level + prereqs + not excluded).
    pub fn can_accept(
        &self,
        player_level: u8,
        completed_quests: &[u32],
        all_quests: &[QuestData],
    ) -> bool {
        self.meets_level_req(player_level)
            && self.prerequisites_met(completed_quests)
            && !self.excluded_by(completed_quests, all_quests)
    }
}

// --- Objective progress tracking ---

/// Progress on a single quest objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveProgress {
    /// Kill objective: current / required count.
    Kill { current: u16, required: u16 },
    /// Collect objective: current / required count.
    Collect { current: u16, required: u16 },
    /// Interact: done or not.
    Interact { done: bool },
    /// Escort: done or not.
    Escort { done: bool },
    /// Reach location: done or not.
    ReachLocation { done: bool },
}

impl ObjectiveProgress {
    /// Create initial progress from a quest objective definition.
    pub fn from_objective(obj: &QuestObjective) -> Self {
        match *obj {
            QuestObjective::Kill { count, .. } => Self::Kill {
                current: 0,
                required: count,
            },
            QuestObjective::Collect { count, .. } => Self::Collect {
                current: 0,
                required: count,
            },
            QuestObjective::Interact { .. } => Self::Interact { done: false },
            QuestObjective::Escort { .. } => Self::Escort { done: false },
            QuestObjective::ReachLocation { .. } => Self::ReachLocation { done: false },
        }
    }

    /// Whether this objective is complete.
    pub fn is_complete(&self) -> bool {
        match *self {
            Self::Kill { current, required } => current >= required,
            Self::Collect { current, required } => current >= required,
            Self::Interact { done } | Self::Escort { done } | Self::ReachLocation { done } => done,
        }
    }

    /// Record a kill of a creature. Returns `true` if the count changed.
    pub fn record_kill(&mut self, creature_id: u32, obj: &QuestObjective) -> bool {
        if let (
            Self::Kill { current, required },
            QuestObjective::Kill {
                creature_id: cid, ..
            },
        ) = (self, obj)
            && creature_id == *cid
            && *current < *required
        {
            *current += 1;
            return true;
        }
        false
    }

    /// Record collecting an item. Returns `true` if the count changed.
    pub fn record_collect(&mut self, item_id: u32, count: u16, obj: &QuestObjective) -> bool {
        if let (Self::Collect { current, required }, QuestObjective::Collect { item_id: iid, .. }) =
            (self, obj)
            && item_id == *iid
            && *current < *required
        {
            *current = (*current + count).min(*required);
            return true;
        }
        false
    }

    /// Mark an interact/escort/reach objective as done.
    pub fn mark_done(&mut self) {
        match self {
            Self::Interact { done } | Self::Escort { done } | Self::ReachLocation { done } => {
                *done = true;
            }
            _ => {}
        }
    }
}

/// Quest lifecycle state.
///
/// ```text
/// Available → Accepted → InProgress → Complete → TurnedIn
///                ↑______________|
///          (objectives not yet done stay InProgress)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestState {
    /// Quest is available but not yet accepted.
    Available,
    /// Quest was just accepted (transitions to InProgress on first tick).
    Accepted,
    /// Objectives are being worked on.
    InProgress,
    /// All objectives complete, ready to turn in.
    Complete,
    /// Quest has been turned in and rewards claimed.
    TurnedIn,
}

/// Per-quest progress for a player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestProgress {
    pub quest_id: u32,
    pub state: QuestState,
    pub objectives: Vec<ObjectiveProgress>,
}

impl QuestProgress {
    /// Create initial progress when accepting a quest.
    pub fn new(quest: &QuestData) -> Self {
        Self {
            quest_id: quest.id,
            state: QuestState::Accepted,
            objectives: quest
                .objectives
                .iter()
                .map(ObjectiveProgress::from_objective)
                .collect(),
        }
    }

    /// Whether all objectives are complete.
    pub fn objectives_complete(&self) -> bool {
        self.objectives.iter().all(|o| o.is_complete())
    }

    /// Update the state based on objective progress.
    ///
    /// Call after any objective update. Transitions:
    /// - Accepted → InProgress (always, on first update)
    /// - InProgress → Complete (when all objectives done)
    pub fn update_state(&mut self) {
        match self.state {
            QuestState::Accepted => {
                self.state = if self.objectives_complete() {
                    QuestState::Complete
                } else {
                    QuestState::InProgress
                };
            }
            QuestState::InProgress => {
                if self.objectives_complete() {
                    self.state = QuestState::Complete;
                }
            }
            _ => {}
        }
    }

    /// Turn in the quest. Returns `true` if successful (was Complete).
    pub fn turn_in(&mut self) -> bool {
        if self.state == QuestState::Complete {
            self.state = QuestState::TurnedIn;
            true
        } else {
            false
        }
    }

    /// Abandon the quest, resetting to Available.
    pub fn abandon(&mut self) {
        self.state = QuestState::Available;
        for obj in &mut self.objectives {
            *obj = match *obj {
                ObjectiveProgress::Kill { required, .. } => ObjectiveProgress::Kill {
                    current: 0,
                    required,
                },
                ObjectiveProgress::Collect { required, .. } => ObjectiveProgress::Collect {
                    current: 0,
                    required,
                },
                ObjectiveProgress::Interact { .. } => ObjectiveProgress::Interact { done: false },
                ObjectiveProgress::Escort { .. } => ObjectiveProgress::Escort { done: false },
                ObjectiveProgress::ReachLocation { .. } => {
                    ObjectiveProgress::ReachLocation { done: false }
                }
            };
        }
    }
}

// --- Reward claiming ---

/// Collected rewards from a quest turn-in.
///
/// The server uses this to apply XP, gold, items, and reputation to the player.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClaimedRewards {
    pub xp: u32,
    pub gold: u32,
    pub items: Vec<(u32, u16)>,
    pub reputation: Vec<(u32, i32)>,
}

/// Process quest rewards on turn-in.
///
/// `choice_index` selects which reward choice the player picks (if any).
/// Returns `None` if the choice index is invalid.
pub fn claim_rewards(quest: &QuestData, choice_index: Option<usize>) -> Option<ClaimedRewards> {
    // Validate choice if there are choices
    if !quest.reward_choices.is_empty() {
        let idx = choice_index?;
        if idx >= quest.reward_choices.len() {
            return None;
        }
    }

    let mut result = ClaimedRewards::default();

    for reward in &quest.rewards {
        match *reward {
            QuestReward::Xp(amount) => result.xp += amount,
            QuestReward::Gold(amount) => result.gold += amount,
            QuestReward::Item { item_id, count } => result.items.push((item_id, count)),
            QuestReward::Reputation { faction_id, amount } => {
                result.reputation.push((faction_id, amount));
            }
        }
    }

    if let Some(idx) = choice_index
        && let Some(choice) = quest.reward_choices.get(idx)
    {
        result.items.push((choice.item_id, choice.count));
    }

    Some(result)
}

// --- Quest giver markers ---

/// Visual marker shown above an NPC's head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestMarker {
    /// Yellow `!` — NPC has a quest available for the player.
    Available,
    /// Silver `!` — NPC has a quest the player can't accept yet (level too low).
    AvailableLowLevel,
    /// Yellow `?` — NPC can accept a completed quest turn-in.
    TurnIn,
    /// Silver `?` — NPC is the turn-in target but objectives aren't complete.
    TurnInIncomplete,
    /// No marker.
    None,
}

/// Determine the quest marker for an NPC given the player's quest state.
///
/// Checks all quests the NPC offers or accepts, returning the highest-priority marker.
/// Priority: TurnIn > TurnInIncomplete > Available > AvailableLowLevel > None.
pub fn npc_quest_marker(
    npc_id: u32,
    quests: &[QuestData],
    log: &QuestLog,
    player_level: u8,
) -> QuestMarker {
    let mut best = QuestMarker::None;

    for quest in quests {
        // Check turn-in: this NPC accepts a quest the player has
        let turn_in_npc = if quest.turn_in_npc == 0 {
            quest.quest_giver
        } else {
            quest.turn_in_npc
        };
        if turn_in_npc == npc_id
            && let Some(progress) = log.get(quest.id)
        {
            let marker = match progress.state {
                QuestState::Complete => QuestMarker::TurnIn,
                QuestState::InProgress | QuestState::Accepted => QuestMarker::TurnInIncomplete,
                _ => QuestMarker::None,
            };
            best = higher_priority(best, marker);
        }

        // Check offer: this NPC offers a quest the player doesn't have
        if quest.quest_giver == npc_id && !log.has_quest(quest.id) && !log.is_completed(quest.id) {
            let marker = if quest.can_accept(player_level, &log.completed, quests) {
                QuestMarker::Available
            } else if !quest.meets_level_req(player_level)
                && quest.prerequisites_met(&log.completed)
            {
                QuestMarker::AvailableLowLevel
            } else {
                QuestMarker::None
            };
            best = higher_priority(best, marker);
        }
    }

    best
}

fn higher_priority(a: QuestMarker, b: QuestMarker) -> QuestMarker {
    if priority(b) > priority(a) { b } else { a }
}

fn priority(m: QuestMarker) -> u8 {
    match m {
        QuestMarker::None => 0,
        QuestMarker::AvailableLowLevel => 1,
        QuestMarker::Available => 2,
        QuestMarker::TurnInIncomplete => 3,
        QuestMarker::TurnIn => 4,
    }
}

// --- Per-player quest log ---

/// Maximum number of active (non-turned-in) quests a player can hold.
const MAX_ACTIVE_QUESTS: usize = 25;

/// Timestamp of when a repeatable quest was last completed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuestCooldown {
    pub quest_id: u32,
    pub repeat: QuestRepeat,
    /// Server timestamp (seconds since epoch) when the quest was completed.
    pub completed_at: u64,
}

/// Seconds in a day (24h).
const SECONDS_PER_DAY: u64 = 86400;
/// Seconds in a week (7d).
const SECONDS_PER_WEEK: u64 = 604800;

impl QuestCooldown {
    /// Whether this quest is available again given the current server time.
    pub fn is_reset(&self, now: u64) -> bool {
        match self.repeat {
            QuestRepeat::None => false,
            QuestRepeat::Daily => now >= self.next_reset(),
            QuestRepeat::Weekly => now >= self.next_reset(),
        }
    }

    /// Timestamp of the next reset.
    pub fn next_reset(&self) -> u64 {
        let period = match self.repeat {
            QuestRepeat::None => return u64::MAX,
            QuestRepeat::Daily => SECONDS_PER_DAY,
            QuestRepeat::Weekly => SECONDS_PER_WEEK,
        };
        // Next period boundary after completion
        let periods_elapsed = self.completed_at / period;
        (periods_elapsed + 1) * period
    }
}

/// Per-player quest log tracking all active and completed quests.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QuestLog {
    /// Active quests (Accepted/InProgress/Complete — not yet turned in).
    pub active: Vec<QuestProgress>,
    /// IDs of quests that have been turned in (for prerequisite checks).
    pub completed: Vec<u32>,
    /// Cooldowns for repeatable quests.
    pub cooldowns: Vec<QuestCooldown>,
}

impl QuestLog {
    /// Accept a quest. Returns `false` if the log is full, already active, or on cooldown.
    pub fn accept(&mut self, quest: &QuestData) -> bool {
        if self.active.len() >= MAX_ACTIVE_QUESTS {
            return false;
        }
        if self.has_quest(quest.id) {
            return false;
        }
        if self.is_on_cooldown(quest.id) {
            return false;
        }
        self.active.push(QuestProgress::new(quest));
        true
    }

    /// Whether a quest is currently in the active log.
    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.active.iter().any(|q| q.quest_id == quest_id)
    }

    /// Whether a quest has been completed (turned in).
    pub fn is_completed(&self, quest_id: u32) -> bool {
        self.completed.contains(&quest_id)
    }

    /// Get mutable progress for an active quest.
    pub fn get_mut(&mut self, quest_id: u32) -> Option<&mut QuestProgress> {
        self.active.iter_mut().find(|q| q.quest_id == quest_id)
    }

    /// Get progress for an active quest.
    pub fn get(&self, quest_id: u32) -> Option<&QuestProgress> {
        self.active.iter().find(|q| q.quest_id == quest_id)
    }

    /// Turn in a quest: move from active to completed. Returns `false` if not ready.
    pub fn turn_in(&mut self, quest_id: u32, quest: &QuestData, now: u64) -> bool {
        let Some(progress) = self.get_mut(quest_id) else {
            return false;
        };
        if !progress.turn_in() {
            return false;
        }
        self.completed.push(quest_id);
        self.active.retain(|q| q.quest_id != quest_id);

        // Record cooldown for repeatable quests
        if quest.repeat != QuestRepeat::None {
            self.cooldowns.retain(|c| c.quest_id != quest_id);
            self.cooldowns.push(QuestCooldown {
                quest_id,
                repeat: quest.repeat,
                completed_at: now,
            });
        }
        true
    }

    /// Whether a repeatable quest is on cooldown (not yet reset).
    pub fn is_on_cooldown(&self, quest_id: u32) -> bool {
        // Cooldowns are only checked at accept time; actual reset is via process_resets
        self.cooldowns.iter().any(|c| c.quest_id == quest_id)
    }

    /// Process daily/weekly resets. Removes expired cooldowns and their
    /// completed entries so repeatable quests become available again.
    pub fn process_resets(&mut self, now: u64) {
        let reset_ids: Vec<u32> = self
            .cooldowns
            .iter()
            .filter(|c| c.is_reset(now))
            .map(|c| c.quest_id)
            .collect();
        for id in &reset_ids {
            self.completed.retain(|q| q != id);
        }
        self.cooldowns.retain(|c| !c.is_reset(now));
    }

    /// Abandon a quest, removing it from the active log.
    pub fn abandon(&mut self, quest_id: u32) -> bool {
        let had = self.has_quest(quest_id);
        self.active.retain(|q| q.quest_id != quest_id);
        had
    }

    /// Number of active quests.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

#[cfg(test)]
#[path = "quest_tests.rs"]
mod tests;
