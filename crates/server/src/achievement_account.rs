//! Live account contributions project queued owned snapshots, not stale durable rows.
//! One FIFO writer persists each character/account pair and its guild news atomically.
use super::*;
use redb::{ReadableDatabase, TableDefinition};
use std::sync::Mutex;

const ACCOUNTS: TableDefinition<u64, &[u8]> = TableDefinition::new("account_achievements_v1");

type OwnedNews = (i64, Vec<(shared::protocol::GuildNewsKind, bool)>);

#[derive(Default, Serialize, Deserialize)]
struct AccountProgress {
    contributions: BTreeMap<u64, PlayerAchievementState>,
    state: PlayerAchievementState,
}

#[derive(Default)]
pub(crate) struct AccountProgressCache(Mutex<BTreeMap<u64, AccountProgress>>);

#[derive(Clone, Copy)]
pub(super) struct AccountUpdate {
    pub account: u64,
    pub character: u64,
    pub completed_map: Option<u32>,
    pub race: u8,
    pub now: i64,
}

pub(super) fn sync_account_progress(
    db: &GameDatabase,
    state: &mut PlayerAchievementState,
    catalog: &AchievementCatalog,
    update: AccountUpdate,
) -> Vec<AchievementToastSnapshot> {
    let mut toasts = Vec::new();
    loop {
        let (aggregate, account_toasts) = save_account_progress(db, state, catalog, update);
        state.account = Some(Box::new(aggregate));
        toasts.extend(account_toasts);
        let character_toasts =
            catalog.grant_completed(state, update.completed_map, update.race, false);
        if character_toasts.is_empty() {
            return toasts;
        }
        for toast in &character_toasts {
            state.earned_at.insert(toast.achievement_id, update.now);
        }
        toasts.extend(character_toasts);
    }
}

pub(super) fn save_account_progress(
    db: &GameDatabase,
    character_state: &PlayerAchievementState,
    catalog: &AchievementCatalog,
    update: AccountUpdate,
) -> (PlayerAchievementState, Vec<AchievementToastSnapshot>) {
    let mut accounts = db
        .accounts
        .0
        .lock()
        .expect("live account contributions lock");
    let saved = accounts
        .entry(update.account)
        .or_insert_with(|| load_progress(&db.db, update.account));
    let previous_earned = saved
        .contributions
        .get(&update.character)
        .map(|state| state.earned_ids.clone())
        .unwrap_or_default();
    let character_bytes = insert_contribution(saved, update.character, character_state);
    let mut aggregate = load_account_aggregate(&db.db, update.account, saved, catalog);
    if let Some(previous) = character_state.account.as_deref() {
        aggregate.evaluation = previous.evaluation.clone();
    }
    let (aggregate, toasts) = evaluate_account_progress(catalog, saved, &update, aggregate);
    saved.state = aggregate.clone();
    let news_ids: BTreeSet<_> = character_state
        .earned_ids
        .difference(&previous_earned)
        .copied()
        .chain(toasts.iter().map(|toast| toast.achievement_id))
        .collect();
    let news = news_ids
        .into_iter()
        .filter_map(|id| catalog.build_guild_news_achievement(id, ""))
        .collect();
    enqueue_progress(
        db,
        (update.account, update.character),
        character_bytes,
        saved,
        Some((update.now, news)),
    );
    (aggregate, toasts)
}

pub(super) fn save_monthly_contribution(
    db: &GameDatabase,
    account: u64,
    character: u64,
    state: &PlayerAchievementState,
) -> crate::persistence_writer::CommitReceipt {
    let mut accounts = db
        .accounts
        .0
        .lock()
        .expect("live account contributions lock");
    let saved = accounts
        .entry(account)
        .or_insert_with(|| load_progress(&db.db, account));
    let character_bytes = insert_contribution(saved, character, state);
    enqueue_progress(db, (account, character), character_bytes, saved, None)
}

pub(super) fn monthly_contributions(
    db: &GameDatabase,
    account: u64,
) -> Vec<PlayerAchievementState> {
    let mut accounts = db
        .accounts
        .0
        .lock()
        .expect("live account contributions lock");
    accounts
        .entry(account)
        .or_insert_with(|| load_progress(&db.db, account))
        .contributions
        .values()
        .cloned()
        .collect()
}

fn insert_contribution(
    saved: &mut AccountProgress,
    character: u64,
    state: &PlayerAchievementState,
) -> Vec<u8> {
    let bytes = serde_json::to_vec(state).expect("encode character contribution");
    saved
        .contributions
        .insert(character, decode_saved_achievement_state(&bytes).0);
    bytes
}

fn enqueue_progress(
    db: &GameDatabase,
    owner: (u64, u64),
    character_bytes: Vec<u8>,
    saved: &AccountProgress,
    news: Option<OwnedNews>,
) -> crate::persistence_writer::CommitReceipt {
    let (account, character) = owner;
    let account_bytes = serde_json::to_vec(saved).expect("encode account contributions");
    db.writer
        .enqueue("account/monthly contributions", move |db| {
            let txn = db.begin_write().map_err(|error| error.to_string())?;
            {
                let mut characters = txn
                    .open_table(persistence::ACHIEVEMENTS)
                    .map_err(|error| error.to_string())?;
                characters
                    .insert(character, character_bytes.as_slice())
                    .map_err(|error| error.to_string())?;
                let mut accounts = txn
                    .open_table(ACCOUNTS)
                    .map_err(|error| error.to_string())?;
                accounts
                    .insert(account, account_bytes.as_slice())
                    .map_err(|error| error.to_string())?;
            }
            if let Some((timestamp, entries)) = news {
                crate::guild_news::record_achievements_in(
                    &txn,
                    character,
                    timestamp,
                    entries.into_iter(),
                );
            }
            txn.commit().map_err(|error| error.to_string())
        })
}

fn load_progress(db: &redb::Database, account: u64) -> AccountProgress {
    let txn = db.begin_read().expect("read account contributions");
    let mut saved = match txn.open_table(ACCOUNTS) {
        Ok(table) => table
            .get(account)
            .expect("read account contributions")
            .map(|row| serde_json::from_slice(row.value()).expect("decode account contributions"))
            .unwrap_or_default(),
        Err(redb::TableError::TableDoesNotExist(_)) => AccountProgress::default(),
        Err(error) => panic!("open account contributions: {error}"),
    };
    let characters = txn
        .open_table(persistence::ACHIEVEMENTS)
        .expect("open character contributions");
    for id in persistence::load_account_char_ids(db, account) {
        if let Some(row) = characters.get(id).expect("read character contribution") {
            saved
                .contributions
                .insert(id, decode_saved_achievement_state(row.value()).0);
        }
    }
    saved
}

fn load_account_aggregate(
    db: &redb::Database,
    account: u64,
    saved: &AccountProgress,
    catalog: &AchievementCatalog,
) -> PlayerAchievementState {
    let mut aggregate = catalog.aggregate(saved.contributions.values().cloned());
    let pets = crate::battle_pets::load_journal(db, account);
    catalog.set_account_unique_pet_count(
        &mut aggregate,
        u32::try_from(pets.unique_species()).expect("journal count fits u32"),
    );
    catalog.set_account_toy_ownership(
        &mut aggregate,
        &crate::persistence::toy_store::load(db, account),
    );
    aggregate
}

fn evaluate_account_progress(
    catalog: &AchievementCatalog,
    saved: &AccountProgress,
    update: &AccountUpdate,
    mut aggregate: PlayerAchievementState,
) -> (PlayerAchievementState, Vec<AchievementToastSnapshot>) {
    aggregate.earned_ids.extend(&saved.state.earned_ids);
    for (&id, &date) in &saved.state.earned_at {
        aggregate
            .earned_at
            .entry(id)
            .and_modify(|existing| *existing = (*existing).min(date))
            .or_insert(date);
    }
    let toasts = catalog.grant_completed(&mut aggregate, update.completed_map, update.race, true);
    for toast in &toasts {
        aggregate.earned_at.insert(toast.achievement_id, update.now);
    }
    (aggregate, toasts)
}
