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

    /// Whether a player can accept this quest.
    pub fn can_accept(&self, player_level: u8, completed_quests: &[u32]) -> bool {
        self.meets_level_req(player_level) && self.prerequisites_met(completed_quests)
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
            QuestObjective::Kill { count, .. } => Self::Kill { current: 0, required: count },
            QuestObjective::Collect { count, .. } => Self::Collect { current: 0, required: count },
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
        if let (Self::Kill { current, required }, QuestObjective::Kill { creature_id: cid, .. }) = (self, obj)
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
        if let (Self::Collect { current, required }, QuestObjective::Collect { item_id: iid, .. }) = (self, obj)
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
            objectives: quest.objectives.iter().map(ObjectiveProgress::from_objective).collect(),
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
                ObjectiveProgress::Kill { required, .. } => {
                    ObjectiveProgress::Kill { current: 0, required }
                }
                ObjectiveProgress::Collect { required, .. } => {
                    ObjectiveProgress::Collect { current: 0, required }
                }
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

// --- Per-player quest log ---

/// Maximum number of active (non-turned-in) quests a player can hold.
const MAX_ACTIVE_QUESTS: usize = 25;

/// Per-player quest log tracking all active and completed quests.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuestLog {
    /// Active quests (Accepted/InProgress/Complete — not yet turned in).
    pub active: Vec<QuestProgress>,
    /// IDs of quests that have been turned in (for prerequisite checks).
    pub completed: Vec<u32>,
}

impl QuestLog {
    /// Accept a quest. Returns `false` if the log is full or quest already active.
    pub fn accept(&mut self, quest: &QuestData) -> bool {
        if self.active.len() >= MAX_ACTIVE_QUESTS {
            return false;
        }
        if self.has_quest(quest.id) {
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
    pub fn turn_in(&mut self, quest_id: u32) -> bool {
        let Some(progress) = self.get_mut(quest_id) else {
            return false;
        };
        if !progress.turn_in() {
            return false;
        }
        self.completed.push(quest_id);
        self.active.retain(|q| q.quest_id != quest_id);
        true
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
mod tests {
    use super::*;

    fn sample_quest() -> QuestData {
        QuestData {
            id: 100,
            title: "The Defias Brotherhood".into(),
            description: "Investigate the Defias threat in Westfall.".into(),
            objective_text: "Kill 10 Defias Bandits".into(),
            objectives: vec![QuestObjective::Kill {
                creature_id: 94,
                count: 10,
            }],
            rewards: vec![
                QuestReward::Xp(1500),
                QuestReward::Gold(350),
            ],
            reward_choices: vec![
                QuestRewardChoice { item_id: 2041, count: 1 },
                QuestRewardChoice { item_id: 2042, count: 1 },
            ],
            required_level: 10,
            suggested_level: 14,
            prerequisites: vec![50],
            quest_giver: 234,
            turn_in_npc: 234,
        }
    }

    #[test]
    fn quest_data_construction() {
        let q = sample_quest();
        assert_eq!(q.id, 100);
        assert_eq!(q.objectives.len(), 1);
        assert_eq!(q.rewards.len(), 2);
        assert_eq!(q.reward_choices.len(), 2);
        assert_eq!(q.prerequisites, vec![50]);
    }

    #[test]
    fn meets_level_req() {
        let q = sample_quest();
        assert!(!q.meets_level_req(9));
        assert!(q.meets_level_req(10));
        assert!(q.meets_level_req(80));
    }

    #[test]
    fn prerequisites_met_empty() {
        let mut q = sample_quest();
        q.prerequisites.clear();
        assert!(q.prerequisites_met(&[]));
    }

    #[test]
    fn prerequisites_met_with_completed() {
        let q = sample_quest();
        assert!(!q.prerequisites_met(&[]));
        assert!(!q.prerequisites_met(&[49]));
        assert!(q.prerequisites_met(&[50]));
        assert!(q.prerequisites_met(&[50, 51, 52]));
    }

    #[test]
    fn can_accept_both_checks() {
        let q = sample_quest();
        assert!(!q.can_accept(9, &[50]));   // too low level
        assert!(!q.can_accept(10, &[]));     // prereq missing
        assert!(q.can_accept(10, &[50]));    // both met
    }

    #[test]
    fn objective_variants() {
        let objs: [QuestObjective; 5] = [
            QuestObjective::Kill { creature_id: 1, count: 5 },
            QuestObjective::Collect { item_id: 2, count: 3 },
            QuestObjective::Interact { target_id: 3 },
            QuestObjective::Escort { npc_id: 4 },
            QuestObjective::ReachLocation { x: 1.0, y: 2.0, z: 3.0, radius: 10.0 },
        ];
        assert_eq!(objs.len(), 5);
    }

    #[test]
    fn reward_variants() {
        let rewards: [QuestReward; 4] = [
            QuestReward::Xp(1000),
            QuestReward::Gold(500),
            QuestReward::Item { item_id: 100, count: 1 },
            QuestReward::Reputation { faction_id: 72, amount: 250 },
        ];
        assert_eq!(rewards.len(), 4);
    }

    #[test]
    fn quest_serialization_round_trip() {
        let q = sample_quest();
        let json = serde_json::to_string(&q).unwrap();
        let decoded: QuestData = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, q);
    }

    // --- Objective progress tests ---

    #[test]
    fn kill_progress_tracks_count() {
        let obj = QuestObjective::Kill { creature_id: 94, count: 3 };
        let mut progress = ObjectiveProgress::from_objective(&obj);
        assert!(!progress.is_complete());

        assert!(progress.record_kill(94, &obj));
        assert!(progress.record_kill(94, &obj));
        assert!(!progress.is_complete());

        assert!(progress.record_kill(94, &obj));
        assert!(progress.is_complete());
    }

    #[test]
    fn kill_wrong_creature_ignored() {
        let obj = QuestObjective::Kill { creature_id: 94, count: 3 };
        let mut progress = ObjectiveProgress::from_objective(&obj);
        assert!(!progress.record_kill(999, &obj));
    }

    #[test]
    fn kill_over_count_capped() {
        let obj = QuestObjective::Kill { creature_id: 94, count: 2 };
        let mut progress = ObjectiveProgress::from_objective(&obj);
        progress.record_kill(94, &obj);
        progress.record_kill(94, &obj);
        assert!(!progress.record_kill(94, &obj)); // already at max
    }

    #[test]
    fn collect_progress_tracks_count() {
        let obj = QuestObjective::Collect { item_id: 50, count: 5 };
        let mut progress = ObjectiveProgress::from_objective(&obj);
        assert!(progress.record_collect(50, 3, &obj));
        assert!(!progress.is_complete());
        assert!(progress.record_collect(50, 3, &obj)); // clamped to 5
        assert!(progress.is_complete());
    }

    #[test]
    fn interact_mark_done() {
        let obj = QuestObjective::Interact { target_id: 10 };
        let mut progress = ObjectiveProgress::from_objective(&obj);
        assert!(!progress.is_complete());
        progress.mark_done();
        assert!(progress.is_complete());
    }

    #[test]
    fn escort_mark_done() {
        let obj = QuestObjective::Escort { npc_id: 5 };
        let mut progress = ObjectiveProgress::from_objective(&obj);
        progress.mark_done();
        assert!(progress.is_complete());
    }

    #[test]
    fn reach_location_mark_done() {
        let obj = QuestObjective::ReachLocation { x: 0.0, y: 0.0, z: 0.0, radius: 10.0 };
        let mut progress = ObjectiveProgress::from_objective(&obj);
        progress.mark_done();
        assert!(progress.is_complete());
    }

    #[test]
    fn quest_progress_complete_check() {
        let q = sample_quest();
        let mut progress = QuestProgress::new(&q);
        assert!(!progress.objectives_complete());

        let obj = &q.objectives[0];
        for _ in 0..10 {
            progress.objectives[0].record_kill(94, obj);
        }
        assert!(progress.objectives_complete());
    }

    // --- Quest state tests ---

    #[test]
    fn quest_state_accepted_to_in_progress() {
        let q = sample_quest();
        let mut progress = QuestProgress::new(&q);
        assert_eq!(progress.state, QuestState::Accepted);

        progress.update_state();
        assert_eq!(progress.state, QuestState::InProgress);
    }

    #[test]
    fn quest_state_in_progress_to_complete() {
        let q = sample_quest();
        let mut progress = QuestProgress::new(&q);
        progress.update_state(); // → InProgress

        let obj = &q.objectives[0];
        for _ in 0..10 {
            progress.objectives[0].record_kill(94, obj);
        }
        progress.update_state();
        assert_eq!(progress.state, QuestState::Complete);
    }

    #[test]
    fn quest_state_turn_in() {
        let q = sample_quest();
        let mut progress = QuestProgress::new(&q);
        progress.state = QuestState::Complete;
        assert!(progress.turn_in());
        assert_eq!(progress.state, QuestState::TurnedIn);
    }

    #[test]
    fn quest_state_cant_turn_in_incomplete() {
        let q = sample_quest();
        let mut progress = QuestProgress::new(&q);
        progress.update_state(); // → InProgress
        assert!(!progress.turn_in());
        assert_eq!(progress.state, QuestState::InProgress);
    }

    #[test]
    fn quest_state_abandon_resets() {
        let q = sample_quest();
        let mut progress = QuestProgress::new(&q);
        progress.update_state();

        let obj = &q.objectives[0];
        for _ in 0..5 {
            progress.objectives[0].record_kill(94, obj);
        }

        progress.abandon();
        assert_eq!(progress.state, QuestState::Available);
        assert!(!progress.objectives[0].is_complete());
        if let ObjectiveProgress::Kill { current, .. } = progress.objectives[0] {
            assert_eq!(current, 0);
        }
    }

    #[test]
    fn quest_instant_complete_no_objectives() {
        let mut q = sample_quest();
        q.objectives.clear();
        let mut progress = QuestProgress::new(&q);
        progress.update_state();
        assert_eq!(progress.state, QuestState::Complete);
    }

    // --- Reward claiming tests ---

    #[test]
    fn claim_rewards_basic() {
        let q = sample_quest(); // rewards: Xp(1500), Gold(350), choices: 2 items
        let rewards = claim_rewards(&q, Some(0)).unwrap();
        assert_eq!(rewards.xp, 1500);
        assert_eq!(rewards.gold, 350);
        assert_eq!(rewards.items.len(), 1); // chosen item
        assert_eq!(rewards.items[0].0, 2041);
    }

    #[test]
    fn claim_rewards_second_choice() {
        let q = sample_quest();
        let rewards = claim_rewards(&q, Some(1)).unwrap();
        assert_eq!(rewards.items[0].0, 2042);
    }

    #[test]
    fn claim_rewards_invalid_choice() {
        let q = sample_quest();
        assert!(claim_rewards(&q, Some(5)).is_none()); // out of range
    }

    #[test]
    fn claim_rewards_no_choice_required() {
        let q = second_quest(); // no reward_choices
        let rewards = claim_rewards(&q, None).unwrap();
        assert_eq!(rewards.xp, 800);
        assert!(rewards.items.is_empty());
    }

    #[test]
    fn claim_rewards_must_choose_when_choices_exist() {
        let q = sample_quest(); // has reward_choices
        assert!(claim_rewards(&q, None).is_none());
    }

    #[test]
    fn claim_rewards_with_reputation() {
        let mut q = second_quest();
        q.rewards.push(QuestReward::Reputation {
            faction_id: 72,
            amount: 250,
        });
        let rewards = claim_rewards(&q, None).unwrap();
        assert_eq!(rewards.reputation.len(), 1);
        assert_eq!(rewards.reputation[0], (72, 250));
    }

    #[test]
    fn claim_rewards_with_guaranteed_item() {
        let mut q = second_quest();
        q.rewards.push(QuestReward::Item {
            item_id: 5000,
            count: 3,
        });
        let rewards = claim_rewards(&q, None).unwrap();
        assert_eq!(rewards.items, vec![(5000, 3)]);
    }

    // --- Quest log tests ---

    fn second_quest() -> QuestData {
        QuestData {
            id: 200,
            title: "Red Ridge".into(),
            description: "Help the town.".into(),
            objective_text: "Talk to the marshal.".into(),
            objectives: vec![QuestObjective::Interact { target_id: 500 }],
            rewards: vec![QuestReward::Xp(800)],
            reward_choices: vec![],
            required_level: 15,
            suggested_level: 18,
            prerequisites: vec![100],
            quest_giver: 300,
            turn_in_npc: 300,
        }
    }

    #[test]
    fn quest_log_accept() {
        let mut log = QuestLog::default();
        assert!(log.accept(&sample_quest()));
        assert_eq!(log.active_count(), 1);
        assert!(log.has_quest(100));
    }

    #[test]
    fn quest_log_reject_duplicate() {
        let mut log = QuestLog::default();
        log.accept(&sample_quest());
        assert!(!log.accept(&sample_quest()));
        assert_eq!(log.active_count(), 1);
    }

    #[test]
    fn quest_log_full_rejects() {
        let mut log = QuestLog::default();
        for i in 0..25 {
            let mut q = sample_quest();
            q.id = 1000 + i;
            assert!(log.accept(&q));
        }
        assert!(!log.accept(&second_quest()));
    }

    #[test]
    fn quest_log_turn_in_flow() {
        let q = sample_quest();
        let mut log = QuestLog::default();
        log.accept(&q);

        // Complete objectives
        let obj = &q.objectives[0];
        let progress = log.get_mut(100).unwrap();
        progress.update_state();
        for _ in 0..10 {
            progress.objectives[0].record_kill(94, obj);
        }
        progress.update_state();

        assert!(log.turn_in(100));
        assert!(!log.has_quest(100));
        assert!(log.is_completed(100));
        assert_eq!(log.active_count(), 0);
    }

    #[test]
    fn quest_log_cant_turn_in_incomplete() {
        let mut log = QuestLog::default();
        log.accept(&sample_quest());
        log.get_mut(100).unwrap().update_state();
        assert!(!log.turn_in(100));
    }

    #[test]
    fn quest_log_abandon() {
        let mut log = QuestLog::default();
        log.accept(&sample_quest());
        assert!(log.abandon(100));
        assert!(!log.has_quest(100));
        assert!(!log.is_completed(100));
    }

    #[test]
    fn quest_log_completed_enables_prereq() {
        let _q1 = sample_quest(); // id=100
        let q2 = second_quest(); // prereq=[100]
        let mut log = QuestLog::default();

        // q2 needs q1 completed
        assert!(!q2.prerequisites_met(&log.completed));

        // Complete q1
        log.completed.push(100);
        assert!(q2.prerequisites_met(&log.completed));
    }
}
