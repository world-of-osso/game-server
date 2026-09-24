//! Per-character quest log, rewarded quests and watch list.
use redb::{Database, ReadableDatabase, TableDefinition};

/// Quest state: character_id (u64) -> bitcode `QuestLogData`.
pub(crate) const QUEST_LOGS: TableDefinition<u64, &[u8]> = TableDefinition::new("quest_logs");

/// A quest in the log. `counts` parallels the template's objectives; item
/// objectives stay 0 because their progress is the bag count.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Eq)]
pub struct ActiveQuestData {
    pub quest_id: u32,
    pub counts: Vec<u32>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestLogData {
    /// Log order (accept order).
    pub active: Vec<ActiveQuestData>,
    /// Turned-in quests, ascending; prerequisites and one-time checks read this.
    pub rewarded: Vec<u32>,
    /// Tracked quest ids, in tracking order.
    pub watched: Vec<u32>,
}

impl QuestLogData {
    pub fn active(&self, quest_id: u32) -> Option<&ActiveQuestData> {
        self.active.iter().find(|quest| quest.quest_id == quest_id)
    }

    pub fn is_rewarded(&self, quest_id: u32) -> bool {
        self.rewarded.binary_search(&quest_id).is_ok()
    }

    pub fn mark_rewarded(&mut self, quest_id: u32) {
        if let Err(index) = self.rewarded.binary_search(&quest_id) {
            self.rewarded.insert(index, quest_id);
        }
    }

    /// Drop the quest from the log and the watch list.
    pub fn remove(&mut self, quest_id: u32) -> bool {
        let before = self.active.len();
        self.active.retain(|quest| quest.quest_id != quest_id);
        self.watched.retain(|&id| id != quest_id);
        self.active.len() != before
    }
}

pub fn load_quest_log(db: &Database, character_id: u64) -> QuestLogData {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(QUEST_LOGS) else {
        return QuestLogData::default();
    };
    table
        .get(character_id)
        .expect("Failed to read quest log")
        .map_or_else(QuestLogData::default, |guard| {
            bitcode::decode(guard.value()).expect("Failed to decode quest log")
        })
}

pub fn save_quest_log(db: &Database, character_id: u64, log: &QuestLogData) {
    let bytes = bitcode::encode(log);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(QUEST_LOGS)
            .expect("Failed to open quest_logs table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert quest log");
    }
    txn.commit().expect("Failed to commit quest log save");
}
