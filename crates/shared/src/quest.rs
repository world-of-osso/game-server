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
}
