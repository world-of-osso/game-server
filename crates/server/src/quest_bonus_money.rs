//! Selected-TC authored bonuses; only ordinary max-player-level settlement consumes them.
use crate::quest_data::{QuestDb, QuestTemplate};
use crate::quest_rules::{QuestPlayer, quest_money};
use std::path::Path;

const NO_MONEY_FOR_XP: u32 = 0x100;
const WORLD_QUEST_MODIFIER: u32 = 0x100;

#[derive(serde::Deserialize)]
struct Bonus {
    quest_id: u32,
    bonus_money: u32,
}

fn catalog_error(path: &Path, error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
        "required quest bonus catalog {}: {error}",
        path.display()
    ))))
}

pub(crate) fn load(path: &Path, quests: &mut QuestDb) -> rusqlite::Result<()> {
    let bytes = std::fs::read(path).map_err(|error| catalog_error(path, error))?;
    let bonuses: Vec<Bonus> =
        serde_json::from_slice(&bytes).map_err(|error| catalog_error(path, error))?;
    for bonus in bonuses {
        if let Some(quest) = quests.quests.get_mut(&bonus.quest_id) {
            quest.reward_bonus_money = bonus.bonus_money;
        }
    }
    Ok(())
}

/// TC Player::RewardQuest adds GetRewMoneyMaxLevel to base money, not the preview.
/// An unrepresentable settlement is rejected before any gold/items/log mutation.
pub(crate) fn settlement(
    quests: &QuestDb,
    quest: &QuestTemplate,
    player: QuestPlayer,
) -> Option<i32> {
    let base = quest_money(quests, quest, player);
    if player.level < crate::experience::MAX_LEVEL
        || quest.flags & NO_MONEY_FOR_XP != 0
        || quest.quest_info_modifiers & WORLD_QUEST_MODIFIER != 0
        || quest.captured_reward_xp.is_some()
    {
        return Some(base);
    }
    base.checked_add(i32::try_from(quest.reward_bonus_money).ok()?)
}
