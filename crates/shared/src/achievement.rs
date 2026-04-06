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
    pub completed_ids: Vec<u32>,
    pub total_points: u32,
}

impl CharacterAchievements {
    /// Get or create progress for an achievement.
    pub fn get_or_create(&mut self, achievement: &AchievementData) -> &mut AchievementProgress {
        let pos = self.progress.iter().position(|p| p.achievement_id == achievement.id);
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

    /// List completed achievement IDs that are Feats of Strength.
    pub fn completed_feats(&self, all_achievements: &[AchievementData]) -> Vec<u32> {
        self.progress
            .iter()
            .filter(|p| p.completed)
            .filter(|p| {
                all_achievements
                    .iter()
                    .any(|a| a.id == p.achievement_id && a.feat_of_strength)
            })
            .map(|p| p.achievement_id)
            .collect()
    }

    /// List completed achievement IDs that are NOT Feats of Strength.
    pub fn completed_normal(&self, all_achievements: &[AchievementData]) -> Vec<u32> {
        self.progress
            .iter()
            .filter(|p| p.completed)
            .filter(|p| {
                all_achievements
                    .iter()
                    .any(|a| a.id == p.achievement_id && !a.feat_of_strength)
            })
            .map(|p| p.achievement_id)
            .collect()
    }
}

impl AccountAchievements {
    /// Record an account-wide achievement as completed.
    pub fn complete(&mut self, achievement: &AchievementData) {
        if self.completed_ids.contains(&achievement.id) {
            return;
        }
        self.completed_ids.push(achievement.id);
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
        assert!(!progress.update_completion());
    }

    // --- Tracker tests ---

    fn account_wide_achievement() -> AchievementData {
        AchievementData {
            id: 2000,
            name: "Account Level 80".into(),
            description: "Any character reaches 80.".into(),
            criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
            points: 10,
            reward: None,
            account_wide: true,
            feat_of_strength: false,
        }
    }

    #[test]
    fn character_achievement_tracks_points() {
        let a = sample_achievement(); // 10 points
        let mut char_ach = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();

        let progress = char_ach.get_or_create(&a);
        progress.record_level(80, &a);
        assert!(try_complete(&a, &mut char_ach, &mut acct));
        assert_eq!(char_ach.total_points, 10);
        assert!(char_ach.is_completed(a.id));
    }

    #[test]
    fn account_wide_shared() {
        let a = account_wide_achievement();
        let mut char1 = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();

        let progress = char1.get_or_create(&a);
        progress.record_level(80, &a);
        assert!(try_complete(&a, &mut char1, &mut acct));
        assert!(acct.is_completed(a.id));
        assert_eq!(acct.total_points, 10);

        // Second character shouldn't re-complete
        let mut char2 = CharacterAchievements::default();
        let progress2 = char2.get_or_create(&a);
        progress2.record_level(80, &a);
        assert!(!try_complete(&a, &mut char2, &mut acct));
    }

    #[test]
    fn per_character_not_shared() {
        let a = sample_achievement(); // account_wide: false
        let mut char1 = CharacterAchievements::default();
        let mut char2 = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();

        let p1 = char1.get_or_create(&a);
        p1.record_level(80, &a);
        try_complete(&a, &mut char1, &mut acct);

        // Second character can still earn it
        let p2 = char2.get_or_create(&a);
        p2.record_level(80, &a);
        assert!(try_complete(&a, &mut char2, &mut acct));
    }

    #[test]
    fn feat_of_strength_no_points() {
        let a = AchievementData {
            id: 99, name: "Feat".into(), description: "".into(),
            criteria: vec![AchievementCriteria::ReachLevel { level: 1 }],
            points: 0, reward: None, account_wide: false, feat_of_strength: true,
        };
        let mut char_ach = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();
        let p = char_ach.get_or_create(&a);
        p.record_level(1, &a);
        try_complete(&a, &mut char_ach, &mut acct);
        assert_eq!(char_ach.total_points, 0);
    }

    #[test]
    fn incomplete_does_not_complete() {
        let a = sample_achievement();
        let mut char_ach = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();
        let p = char_ach.get_or_create(&a);
        p.record_level(50, &a);
        assert!(!try_complete(&a, &mut char_ach, &mut acct));
        assert!(!char_ach.is_completed(a.id));
    }

    // --- Cumulative points tests ---

    #[test]
    fn combined_points_sums_both_scopes() {
        let char_only = sample_achievement(); // 10 pts, per-character
        let acct_wide = account_wide_achievement(); // 10 pts, account-wide

        let mut char_ach = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();

        // Complete character achievement
        let p = char_ach.get_or_create(&char_only);
        p.record_level(80, &char_only);
        try_complete(&char_only, &mut char_ach, &mut acct);

        // Complete account-wide achievement
        let p = char_ach.get_or_create(&acct_wide);
        p.record_level(80, &acct_wide);
        try_complete(&acct_wide, &mut char_ach, &mut acct);

        assert_eq!(char_ach.total_points, 10); // per-char only
        assert_eq!(acct.total_points, 10); // account-wide only
        assert_eq!(combined_points(&char_ach, &acct), 20);
    }

    #[test]
    fn completed_count_both_scopes() {
        let char_only = sample_achievement();
        let acct_wide = account_wide_achievement();

        let mut char_ach = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();

        let p = char_ach.get_or_create(&char_only);
        p.record_level(80, &char_only);
        try_complete(&char_only, &mut char_ach, &mut acct);

        let p = char_ach.get_or_create(&acct_wide);
        p.record_level(80, &acct_wide);
        try_complete(&acct_wide, &mut char_ach, &mut acct);

        assert_eq!(completed_count(&char_ach, &acct), 3);
        // 2 in character progress (both completed) + 1 in account completed_ids
    }

    #[test]
    fn zero_points_when_empty() {
        let char_ach = CharacterAchievements::default();
        let acct = AccountAchievements::default();
        assert_eq!(combined_points(&char_ach, &acct), 0);
        assert_eq!(completed_count(&char_ach, &acct), 0);
    }

    // --- Feats of Strength tests ---

    fn feat_achievement() -> AchievementData {
        AchievementData {
            id: 2336,
            name: "Realm First! Level 80".into(),
            description: "First to reach 80.".into(),
            criteria: vec![AchievementCriteria::ReachLevel { level: 80 }],
            points: 0,
            reward: Some(AchievementReward::Title("the Supreme".into())),
            account_wide: false,
            feat_of_strength: true,
        }
    }

    #[test]
    fn feat_completes_with_zero_points() {
        let feat = feat_achievement();
        let normal = sample_achievement(); // 10 pts
        let mut char_ach = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();

        // Complete both
        let p = char_ach.get_or_create(&normal);
        p.record_level(80, &normal);
        try_complete(&normal, &mut char_ach, &mut acct);

        let p = char_ach.get_or_create(&feat);
        p.record_level(80, &feat);
        try_complete(&feat, &mut char_ach, &mut acct);

        assert_eq!(char_ach.total_points, 10); // only normal
        assert!(char_ach.is_completed(feat.id));
    }

    #[test]
    fn feats_listed_separately() {
        let feat = feat_achievement();
        let normal = sample_achievement();
        let all = vec![normal.clone(), feat.clone()];

        let mut char_ach = CharacterAchievements::default();
        let mut acct = AccountAchievements::default();

        let p = char_ach.get_or_create(&normal);
        p.record_level(80, &normal);
        try_complete(&normal, &mut char_ach, &mut acct);

        let p = char_ach.get_or_create(&feat);
        p.record_level(80, &feat);
        try_complete(&feat, &mut char_ach, &mut acct);

        let feats = char_ach.completed_feats(&all);
        let normals = char_ach.completed_normal(&all);
        assert_eq!(feats, vec![2336]);
        assert_eq!(normals, vec![46]);
    }

    #[test]
    fn no_feats_when_none_completed() {
        let char_ach = CharacterAchievements::default();
        assert!(char_ach.completed_feats(&[]).is_empty());
        assert!(char_ach.completed_normal(&[]).is_empty());
    }
}
