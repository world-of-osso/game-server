//! Achievement system: definitions, criteria, tracking.
//!
//! Ref: AzerothCore `AchievementMgr.cpp`, `Achievement.dbc`.

use serde::{Deserialize, Serialize};

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
            AchievementCriteria::Kill { count, .. } => Self::Counter { current: 0, required: *count },
            AchievementCriteria::CollectItem { count, .. } => Self::Counter { current: 0, required: *count },
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
            criteria: achievement.criteria.iter().map(CriteriaProgress::from_criteria).collect(),
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
            if let AchievementCriteria::Kill { creature_id: cid, .. } = criteria
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_achievement() -> AchievementData {
        AchievementData {
            id: 46,
            name: "Level 80".into(),
            description: "Reach level 80.".into(),
            criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
            points: 10,
            reward: None,
            account_wide: false,
            feat_of_strength: false,
        }
    }

    #[test]
    fn achievement_construction() {
        let a = sample_achievement();
        assert_eq!(a.id, 46);
        assert_eq!(a.points, 10);
        assert_eq!(a.criteria.len(), 1);
        assert!(!a.feat_of_strength);
    }

    #[test]
    fn achievement_with_reward() {
        let a = AchievementData {
            id: 614,
            name: "For the Alliance!".into(),
            description: "Slay the leaders of the Horde.".into(),
            criteria: vec![
                AchievementCriteria::Kill { creature_id: 1, count: 1 },
                AchievementCriteria::Kill { creature_id: 2, count: 1 },
                AchievementCriteria::Kill { creature_id: 3, count: 1 },
                AchievementCriteria::Kill { creature_id: 4, count: 1 },
            ],
            points: 20,
            reward: Some(AchievementReward::Title("of the Alliance".into())),
            account_wide: false,
            feat_of_strength: false,
        };
        assert_eq!(a.criteria.len(), 4);
        assert!(a.reward.is_some());
    }

    #[test]
    fn feat_of_strength() {
        let a = AchievementData {
            id: 2336,
            name: "Realm First! Level 80".into(),
            description: "First player to reach level 80.".into(),
            criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
            points: 0,
            reward: Some(AchievementReward::Title("the Supreme".into())),
            account_wide: false,
            feat_of_strength: true,
        };
        assert!(a.feat_of_strength);
        assert_eq!(a.points, 0);
    }

    #[test]
    fn criteria_all_variants() {
        let criteria: [AchievementCriteria; 5] = [
            AchievementCriteria::Kill { creature_id: 1, count: 10 },
            AchievementCriteria::CompleteQuest { quest_id: 100 },
            AchievementCriteria::ReachLevel { level: 80 },
            AchievementCriteria::CollectItem { item_id: 50, count: 100 },
            AchievementCriteria::VisitArea { area_id: 1 },
        ];
        assert_eq!(criteria.len(), 5);
    }

    #[test]
    fn reward_all_variants() {
        let rewards: [AchievementReward; 3] = [
            AchievementReward::Title("Champion".into()),
            AchievementReward::Item { item_id: 100, count: 1 },
            AchievementReward::Spell { spell_id: 200 },
        ];
        assert_eq!(rewards.len(), 3);
    }

    #[test]
    fn achievement_serialization_round_trip() {
        let a = sample_achievement();
        let json = serde_json::to_string(&a).unwrap();
        let decoded: AchievementData = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, a);
    }

    // --- Criteria progress tests ---

    #[test]
    fn kill_criteria_tracks_count() {
        let a = AchievementData {
            id: 1, name: "Kill 5 Wolves".into(), description: "".into(),
            criteria: vec![AchievementCriteria::Kill { creature_id: 10, count: 5 }],
            points: 10, reward: None, account_wide: false, feat_of_strength: false,
        };
        let mut progress = AchievementProgress::new(&a);
        assert!(!progress.completed);

        progress.record_kill(10, &a);
        progress.record_kill(10, &a);
        progress.record_kill(10, &a);
        assert!(!progress.update_completion());

        progress.record_kill(10, &a);
        progress.record_kill(10, &a);
        assert!(progress.update_completion());
        assert!(progress.completed);
    }

    #[test]
    fn kill_wrong_creature_ignored() {
        let a = AchievementData {
            id: 1, name: "".into(), description: "".into(),
            criteria: vec![AchievementCriteria::Kill { creature_id: 10, count: 1 }],
            points: 5, reward: None, account_wide: false, feat_of_strength: false,
        };
        let mut progress = AchievementProgress::new(&a);
        progress.record_kill(99, &a); // wrong creature
        assert!(!progress.update_completion());
    }

    #[test]
    fn level_criteria() {
        let a = sample_achievement(); // ReachLevel { level: 80 }
        let mut progress = AchievementProgress::new(&a);
        progress.record_level(79, &a);
        assert!(!progress.update_completion());
        progress.record_level(80, &a);
        assert!(progress.update_completion());
    }

    #[test]
    fn quest_criteria() {
        let a = AchievementData {
            id: 1, name: "".into(), description: "".into(),
            criteria: vec![AchievementCriteria::CompleteQuest { quest_id: 100 }],
            points: 5, reward: None, account_wide: false, feat_of_strength: false,
        };
        let mut progress = AchievementProgress::new(&a);
        progress.record_quest(100, &a);
        assert!(progress.update_completion());
    }

    #[test]
    fn visit_criteria() {
        let a = AchievementData {
            id: 1, name: "".into(), description: "".into(),
            criteria: vec![AchievementCriteria::VisitArea { area_id: 42 }],
            points: 5, reward: None, account_wide: false, feat_of_strength: false,
        };
        let mut progress = AchievementProgress::new(&a);
        progress.record_visit(42, &a);
        assert!(progress.update_completion());
    }

    #[test]
    fn collect_criteria() {
        let a = AchievementData {
            id: 1, name: "".into(), description: "".into(),
            criteria: vec![AchievementCriteria::CollectItem { item_id: 50, count: 10 }],
            points: 5, reward: None, account_wide: false, feat_of_strength: false,
        };
        let mut progress = AchievementProgress::new(&a);
        progress.record_collect(50, 7, &a);
        assert!(!progress.update_completion());
        progress.record_collect(50, 5, &a); // clamped to 10
        assert!(progress.update_completion());
    }

    #[test]
    fn multi_criteria_all_required() {
        let a = AchievementData {
            id: 614, name: "For the Alliance!".into(), description: "".into(),
            criteria: vec![
                AchievementCriteria::Kill { creature_id: 1, count: 1 },
                AchievementCriteria::Kill { creature_id: 2, count: 1 },
            ],
            points: 20, reward: None, account_wide: false, feat_of_strength: false,
        };
        let mut progress = AchievementProgress::new(&a);
        progress.record_kill(1, &a);
        assert!(!progress.update_completion()); // only 1 of 2
        progress.record_kill(2, &a);
        assert!(progress.update_completion()); // both done
    }

    #[test]
    fn already_completed_no_double() {
        let a = sample_achievement();
        let mut progress = AchievementProgress::new(&a);
        progress.record_level(80, &a);
        assert!(progress.update_completion());
        assert!(!progress.update_completion()); // already done
    }
}
