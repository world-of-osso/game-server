//! Per-character quest log, rewarded quests and watch list.
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::collections::BTreeMap;

/// Quest state: character_id (u64) -> bitcode `QuestLogData`.
pub(crate) const QUEST_LOGS: TableDefinition<u64, &[u8]> = TableDefinition::new("quest_logs_v2");
const LEGACY_QUEST_LOGS: TableDefinition<u64, &[u8]> = TableDefinition::new("quest_logs");

/// A quest in the log. `counts` parallels the template's objectives; item
/// objectives stay 0 because their progress is the bag count.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveQuestData {
    pub quest_id: u32,
    pub counts: Vec<u32>,
    pub expires_at: Option<i64>,
    pub failed: bool,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestLogData {
    /// Log order (accept order).
    pub active: Vec<ActiveQuestData>,
    /// Turned-in quests, ascending; prerequisites and one-time checks read this.
    pub rewarded: Vec<u32>,
    /// Tracked quest ids, in tracking order.
    pub watched: Vec<u32>,
    /// Last reward time, including daily/weekly history after a reset.
    pub completed_at: BTreeMap<u32, i64>,
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

pub(crate) fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs() as i64
}

#[derive(bitcode::Encode, bitcode::Decode)]
struct LegacyActiveQuest {
    quest_id: u32,
    counts: Vec<u32>,
}
#[derive(bitcode::Encode, bitcode::Decode)]
struct LegacyQuestLog {
    active: Vec<LegacyActiveQuest>,
    rewarded: Vec<u32>,
    watched: Vec<u32>,
}

/// One-time schema migration. Unknown historical completion times are conservatively
/// stamped at migration time, becoming available at the next scheduled reset.
pub(crate) fn migrate(txn: &redb::WriteTransaction, now: i64) {
    let rows: Vec<_> = {
        let table = txn
            .open_table(LEGACY_QUEST_LOGS)
            .expect("open legacy quest logs");
        table
            .iter()
            .expect("iterate legacy quests")
            .map(|row| {
                let (key, value) = row.expect("read legacy quest");
                (
                    key.value(),
                    bitcode::decode::<LegacyQuestLog>(value.value())
                        .expect("decode legacy quest log"),
                )
            })
            .collect()
    };
    {
        let mut table = txn.open_table(QUEST_LOGS).expect("open quest logs v2");
        for (id, old) in rows {
            let completed_at = old.rewarded.iter().map(|&quest| (quest, now)).collect();
            let log = QuestLogData {
                active: old
                    .active
                    .into_iter()
                    .map(|q| ActiveQuestData {
                        quest_id: q.quest_id,
                        counts: q.counts,
                        ..Default::default()
                    })
                    .collect(),
                rewarded: old.rewarded,
                watched: old.watched,
                completed_at,
            };
            table
                .insert(id, bitcode::encode(&log).as_slice())
                .expect("migrate quest log");
        }
    }
    txn.delete_table(LEGACY_QUEST_LOGS)
        .expect("retire legacy quest logs");
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quest_reset_migration_preserves_progress_and_stamps_unknown_completion_once() {
        let db = crate::persistence::temp_database();
        let txn = db.begin_write().unwrap();
        let old = LegacyQuestLog {
            active: vec![LegacyActiveQuest {
                quest_id: 62,
                counts: vec![3],
            }],
            rewarded: vec![28766],
            watched: vec![62],
        };
        txn.open_table(LEGACY_QUEST_LOGS)
            .unwrap()
            .insert(9, bitcode::encode(&old).as_slice())
            .unwrap();
        crate::persistence_commit::commit(txn).unwrap();
        let txn = db.begin_write().unwrap();
        migrate(&txn, 1000);
        crate::persistence_commit::commit(txn).unwrap();
        let migrated = load_quest_log(&db, 9);
        assert_eq!(migrated.active[0].counts, vec![3]);
        assert_eq!(migrated.watched, vec![62]);
        assert_eq!(migrated.completed_at[&28766], 1000);
        let txn = db.begin_write().unwrap();
        migrate(&txn, 2000);
        crate::persistence_commit::commit(txn).unwrap();
        assert_eq!(load_quest_log(&db, 9), migrated);
    }
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
    crate::persistence_commit::commit(txn).expect("Failed to commit quest log save");
}
