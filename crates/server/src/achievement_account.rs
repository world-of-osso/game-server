//! Lazy, atomic account migration. Character rows remain authoritative contributions;
//! retained contributions keep earned account progress when a character is deleted.
use super::*;
use redb::{ReadableTable, TableDefinition};

const ACCOUNTS: TableDefinition<u64, &[u8]> = TableDefinition::new("account_achievements_v1");

#[derive(Default, Serialize, Deserialize)]
struct AccountProgress {
    contributions: BTreeMap<u64, PlayerAchievementState>,
    state: PlayerAchievementState,
}

pub(super) fn save_account_progress(
    db: &redb::Database,
    account: u64,
    character: u64,
    character_state: &PlayerAchievementState,
    catalog: &AchievementCatalog,
    completed_map: Option<u32>,
    race: u8,
    now: i64,
) -> (PlayerAchievementState, Vec<AchievementToastSnapshot>) {
    let characters = persistence::load_account_char_ids(db, account);
    let txn = db
        .begin_write()
        .expect("begin account achievement transaction");
    let mut accounts = txn.open_table(ACCOUNTS).expect("open account achievements");
    let mut saved: AccountProgress = accounts
        .get(account)
        .expect("read account achievements")
        .map(|row| serde_json::from_slice(row.value()).expect("decode account achievements"))
        .unwrap_or_default();
    {
        let mut table = txn
            .open_table(persistence::ACHIEVEMENTS)
            .expect("open character achievements");
        for id in characters {
            if let Some(row) = table.get(id).expect("read achievement contribution") {
                saved
                    .contributions
                    .insert(id, decode_saved_achievement_state(row.value()).0);
            }
        }
        let bytes = serde_json::to_vec(character_state).expect("encode character achievements");
        table
            .insert(character, bytes.as_slice())
            .expect("save character achievement contribution");
        // Round trip deliberately strips the presentation-only account view.
        saved
            .contributions
            .insert(character, decode_saved_achievement_state(&bytes).0);
    }
    let mut aggregate = catalog.aggregate(saved.contributions.values().cloned());
    aggregate.earned_ids.extend(saved.state.earned_ids);
    for (id, date) in saved.state.earned_at {
        aggregate
            .earned_at
            .entry(id)
            .and_modify(|existing| *existing = (*existing).min(date))
            .or_insert(date);
    }
    let toasts = catalog.grant_completed(&mut aggregate, completed_map, race, true);
    for toast in &toasts {
        aggregate.earned_at.insert(toast.achievement_id, now);
    }
    saved.state = aggregate.clone();
    let bytes = serde_json::to_vec(&saved).expect("encode account achievements");
    accounts
        .insert(account, bytes.as_slice())
        .expect("save account achievements");
    drop(accounts);
    txn.commit()
        .expect("commit account achievement migration/update");
    (aggregate, toasts)
}
