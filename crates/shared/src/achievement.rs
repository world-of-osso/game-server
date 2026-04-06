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
}
