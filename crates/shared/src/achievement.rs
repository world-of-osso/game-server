//! Achievement system: definitions, criteria, tracking.
//!
//! Ref: AzerothCore `AchievementMgr.cpp`, `Achievement.dbc`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// What a player must do to earn an achievement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AchievementCriteria {
    /// Kill a specific creature N times.
    Kill { creature_id: u32, count: u32 },
    /// Complete a specific quest.
    CompleteQuest { quest_id: u32 },
    /// Reach a character level.
    ReachLevel { level: u8 },
    /// Collect N of an item (lifetime, not current inventory).
    CollectItem { item_id: u32, count: u32 },
    /// Visit a specific area/zone.
    VisitArea { area_id: u32 },
}

/// Reward granted when an achievement is earned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AchievementReward {
    /// A title the player can display.
    Title(String),
    /// An item mailed to the player.
    Item { item_id: u32, count: u16 },
    /// A spell/effect learned (mount, pet, etc).
    Spell { spell_id: u32 },
}

/// Static achievement definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AchievementData {
    /// Unique achievement ID.
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Description shown in the UI.
    pub description: String,
    /// Criteria that must all be met to earn the achievement.
    pub criteria: Vec<AchievementCriteria>,
    /// Achievement points awarded.
    pub points: u16,
    /// Optional reward.
    pub reward: Option<AchievementReward>,
    /// Whether this is account-wide (true) or per-character (false).
    pub account_wide: bool,
    /// Feat of Strength: no points, special category.
    pub feat_of_strength: bool,
}

// --- Criteria progress tracking ---

/// Progress on a single achievement criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CriteriaProgress {
    /// Counter-based (kill N, collect N): current / required.
    Counter { current: u32, required: u32 },
    /// Boolean (complete quest, reach level, visit area): done or not.
    Done(bool),
}

impl CriteriaProgress {
    /// Create initial progress from a criteria definition.
    pub fn from_criteria(criteria: &AchievementCriteria) -> Self {
        match criteria {
            AchievementCriteria::Kill { count, .. } => Self::Counter {
                current: 0,
                required: *count,
            },
            AchievementCriteria::CollectItem { count, .. } => Self::Counter {
                current: 0,
                required: *count,
            },
            AchievementCriteria::CompleteQuest { .. }
            | AchievementCriteria::ReachLevel { .. }
            | AchievementCriteria::VisitArea { .. } => Self::Done(false),
        }
    }

    /// Whether this criteria is complete.
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Counter { current, required } => current >= required,
            Self::Done(done) => *done,
        }
    }

    /// Increment a counter by amount. No-op for Done criteria.
    pub fn increment(&mut self, amount: u32) {
        if let Self::Counter { current, required } = self {
            *current = (*current + amount).min(*required);
        }
    }

    /// Mark a boolean criteria as done.
    pub fn mark_done(&mut self) {
        if let Self::Done(done) = self {
            *done = true;
        }
    }
}

/// Per-achievement progress for a player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AchievementProgress {
    pub achievement_id: u32,
    pub criteria: Vec<CriteriaProgress>,
    pub completed: bool,
}

impl AchievementProgress {
    pub fn new(achievement: &AchievementData) -> Self {
        Self {
            achievement_id: achievement.id,
            criteria: achievement
                .criteria
                .iter()
                .map(CriteriaProgress::from_criteria)
                .collect(),
            completed: false,
        }
    }

    /// Update completion state. Returns `true` if just completed this call.
    pub fn update_completion(&mut self) -> bool {
        if self.completed {
            return false;
        }
        if self.criteria.iter().all(|c| c.is_complete()) {
            self.completed = true;
            return true;
        }
        false
    }

    /// Record a creature kill. Updates matching Kill criteria.
    pub fn record_kill(&mut self, creature_id: u32, achievement: &AchievementData) {
        for (progress, criteria) in self.criteria.iter_mut().zip(&achievement.criteria) {
            if let AchievementCriteria::Kill {
                creature_id: cid, ..
            } = criteria
                && *cid == creature_id
            {
                progress.increment(1);
            }
        }
    }

    /// Record a quest completion.
    pub fn record_quest(&mut self, quest_id: u32, achievement: &AchievementData) {
        for (progress, criteria) in self.criteria.iter_mut().zip(&achievement.criteria) {
            if let AchievementCriteria::CompleteQuest { quest_id: qid } = criteria
                && *qid == quest_id
            {
                progress.mark_done();
            }
        }
    }

    /// Record reaching a level.
    pub fn record_level(&mut self, level: u8, achievement: &AchievementData) {
        for (progress, criteria) in self.criteria.iter_mut().zip(&achievement.criteria) {
            if let AchievementCriteria::ReachLevel { level: req } = criteria
                && level >= *req
            {
                progress.mark_done();
            }
        }
    }

    /// Record visiting an area.
    pub fn record_visit(&mut self, area_id: u32, achievement: &AchievementData) {
        for (progress, criteria) in self.criteria.iter_mut().zip(&achievement.criteria) {
            if let AchievementCriteria::VisitArea { area_id: aid } = criteria
                && *aid == area_id
            {
                progress.mark_done();
            }
        }
    }

    /// Record collecting items.
    pub fn record_collect(&mut self, item_id: u32, count: u32, achievement: &AchievementData) {
        for (progress, criteria) in self.criteria.iter_mut().zip(&achievement.criteria) {
            if let AchievementCriteria::CollectItem { item_id: iid, .. } = criteria
                && *iid == item_id
            {
                progress.increment(count);
            }
        }
    }
}

// --- Achievement tracker ---

/// Per-character achievement tracking.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CharacterAchievements {
    pub progress: Vec<AchievementProgress>,
    pub total_points: u32,
}

/// Account-wide achievement tracking (shared across all characters).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AccountAchievements {
    pub completed_ids: BTreeSet<u32>,
    pub total_points: u32,
}

impl CharacterAchievements {
    /// Get or create progress for an achievement.
    pub fn get_or_create(&mut self, achievement: &AchievementData) -> &mut AchievementProgress {
        let pos = self
            .progress
            .iter()
            .position(|p| p.achievement_id == achievement.id);
        if pos.is_none() {
            self.progress.push(AchievementProgress::new(achievement));
        }
        self.progress
            .iter_mut()
            .find(|p| p.achievement_id == achievement.id)
            .unwrap()
    }

    /// Whether an achievement is completed.
    pub fn is_completed(&self, achievement_id: u32) -> bool {
        self.progress
            .iter()
            .any(|p| p.achievement_id == achievement_id && p.completed)
    }

    /// Add points for a newly completed achievement.
    /// Called after `update_completion()` returns true.
    pub fn add_points(&mut self, achievement: &AchievementData) {
        if !achievement.feat_of_strength {
            self.total_points += achievement.points as u32;
        }
    }

    /// List completed achievement IDs matching a predicate on the definition.
    fn completed_matching(
        &self,
        all_achievements: &[AchievementData],
        predicate: fn(&AchievementData) -> bool,
    ) -> Vec<u32> {
        self.progress
            .iter()
            .filter(|p| p.completed)
            .filter(|p| {
                all_achievements
                    .iter()
                    .any(|a| a.id == p.achievement_id && predicate(a))
            })
            .map(|p| p.achievement_id)
            .collect()
    }

    /// List completed achievement IDs that are Feats of Strength.
    pub fn completed_feats(&self, all_achievements: &[AchievementData]) -> Vec<u32> {
        self.completed_matching(all_achievements, |a| a.feat_of_strength)
    }

    /// List completed achievement IDs that are NOT Feats of Strength.
    pub fn completed_normal(&self, all_achievements: &[AchievementData]) -> Vec<u32> {
        self.completed_matching(all_achievements, |a| !a.feat_of_strength)
    }
}

impl AccountAchievements {
    /// Record an account-wide achievement as completed.
    pub fn complete(&mut self, achievement: &AchievementData) {
        if !self.completed_ids.insert(achievement.id) {
            return;
        }
        if !achievement.feat_of_strength {
            self.total_points += achievement.points as u32;
        }
    }

    /// Whether an achievement is completed account-wide.
    pub fn is_completed(&self, achievement_id: u32) -> bool {
        self.completed_ids.contains(&achievement_id)
    }
}

/// Determine if an achievement should be checked against character or account scope.
///
/// Returns `true` if the achievement was newly completed and should trigger rewards.
pub fn try_complete(
    achievement: &AchievementData,
    character: &mut CharacterAchievements,
    account: &mut AccountAchievements,
) -> bool {
    if achievement.account_wide {
        if account.is_completed(achievement.id) {
            return false;
        }
    } else if character.is_completed(achievement.id) {
        return false;
    }

    let progress = character.get_or_create(achievement);
    if !progress.update_completion() {
        return false;
    }

    if achievement.account_wide {
        account.complete(achievement);
    } else {
        character.add_points(achievement);
    }
    true
}

/// Combined achievement point total for display (character + account).
///
/// Character points include per-character achievements only.
/// Account points include account-wide achievements.
/// The combined total is what shows on the player's profile.
pub fn combined_points(character: &CharacterAchievements, account: &AccountAchievements) -> u32 {
    character.total_points + account.total_points
}

/// Count of completed achievements across both scopes.
pub fn completed_count(character: &CharacterAchievements, account: &AccountAchievements) -> usize {
    let char_count = character.progress.iter().filter(|p| p.completed).count();
    let acct_count = account.completed_ids.len();
    char_count + acct_count
}

#[cfg(test)]
#[path = "achievement_tests.rs"]
mod tests;
