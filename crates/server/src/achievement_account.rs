//! Lazy, atomic account migration. Character rows remain authoritative contributions;
//! retained contributions keep earned account progress when a character is deleted.
use super::*;
use redb::{ReadableDatabase, ReadableTable, TableDefinition};

const ACCOUNTS: TableDefinition<u64, &[u8]> = TableDefinition::new("account_achievements_v1");

#[derive(Default, Serialize, Deserialize)]
struct AccountProgress {
    contributions: BTreeMap<u64, PlayerAchievementState>,
    state: PlayerAchievementState,
}

#[derive(Clone, Copy)]
pub(super) struct AccountUpdate {
    pub account: u64,
    pub character: u64,
    pub completed_map: Option<u32>,
    pub race: u8,
    pub now: i64,
}

/// Iterate cross-scope dependencies until neither scope gains another ID.
pub(super) fn sync_account_progress(
    db: &redb::Database,
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
    db: &redb::Database,
    character_state: &PlayerAchievementState,
    catalog: &AchievementCatalog,
    update: AccountUpdate,
) -> (PlayerAchievementState, Vec<AchievementToastSnapshot>) {
    let characters = persistence::load_account_char_ids(db, update.account);
    let txn = db
        .begin_write()
        .expect("begin account achievement transaction");
    let mut accounts = txn.open_table(ACCOUNTS).expect("open account achievements");
    let mut saved: AccountProgress = accounts
        .get(update.account)
        .expect("read account achievements")
        .map(|row| serde_json::from_slice(row.value()).expect("decode account achievements"))
        .unwrap_or_default();
    let previous_earned = read_character_earned_ids_in(&txn, update.character);
    write_character_contributions(
        &txn,
        &mut saved,
        characters,
        update.character,
        character_state,
    );
    let mut aggregate = load_account_aggregate(&txn, update.account, &saved, catalog);
    if let Some(previous) = character_state.account.as_deref() {
        aggregate.evaluation = previous.evaluation.clone();
    }
    let (aggregate, toasts) = evaluate_account_progress(catalog, &saved, &update, aggregate);
    saved.state = aggregate.clone();
    let news_ids: BTreeSet<_> = character_state
        .earned_ids
        .difference(&previous_earned)
        .copied()
        .chain(toasts.iter().map(|toast| toast.achievement_id))
        .collect();
    crate::guild_news::record_achievements_in(
        &txn,
        update.character,
        update.now,
        news_ids.into_iter(),
        catalog,
    );
    let bytes = serde_json::to_vec(&saved).expect("encode account achievements");
    accounts
        .insert(update.account, bytes.as_slice())
        .expect("save account achievements");
    drop(accounts);
    txn.commit()
        .expect("commit account achievement migration/update");
    (aggregate, toasts)
}

fn read_character_earned_ids_in(txn: &redb::WriteTransaction, character: u64) -> BTreeSet<u32> {
    let table = txn
        .open_table(persistence::ACHIEVEMENTS)
        .expect("open prior achievement contribution");
    table
        .get(character)
        .expect("read prior achievement contribution")
        .map(|record| decode_saved_achievement_state(record.value()).0.earned_ids)
        .unwrap_or_default()
}

pub(super) fn save_monthly_contribution(
    db: &redb::Database,
    account: u64,
    character: u64,
    state: &PlayerAchievementState,
) {
    let characters = persistence::load_account_char_ids(db, account);
    let txn = db
        .begin_write()
        .expect("begin monthly contribution transaction");
    let mut accounts = txn
        .open_table(ACCOUNTS)
        .expect("open account achievement contributions");
    let mut saved: AccountProgress = match accounts.get(account).expect("read account contribution")
    {
        Some(row) => serde_json::from_slice(row.value()).expect("decode account contribution"),
        None => AccountProgress::default(),
    };
    write_character_contributions(&txn, &mut saved, characters, character, state);
    let bytes = serde_json::to_vec(&saved).expect("encode monthly contribution");
    accounts
        .insert(account, bytes.as_slice())
        .expect("save monthly contribution");
    drop(accounts);
    txn.commit().expect("commit monthly contribution");
}

pub(super) fn monthly_contributions(
    db: &redb::Database,
    account: u64,
) -> Vec<PlayerAchievementState> {
    let txn = db.begin_read().expect("read monthly account contributions");
    let table = txn
        .open_table(ACCOUNTS)
        .expect("account contributions initialized");
    let row = table
        .get(account)
        .expect("read monthly contributions")
        .expect("monthly contribution saved");
    let saved: AccountProgress =
        serde_json::from_slice(row.value()).expect("decode monthly contributions");
    saved.contributions.into_values().collect()
}

fn write_character_contributions(
    txn: &redb::WriteTransaction,
    saved: &mut AccountProgress,
    characters: Vec<u64>,
    character: u64,
    character_state: &PlayerAchievementState,
) {
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

fn load_account_aggregate(
    txn: &redb::WriteTransaction,
    account: u64,
    saved: &AccountProgress,
    catalog: &AchievementCatalog,
) -> PlayerAchievementState {
    let mut aggregate = catalog.aggregate(saved.contributions.values().cloned());
    let pets = crate::battle_pets::unique_species_in_transaction(txn, account);
    catalog.set_account_unique_pet_count(&mut aggregate, pets);
    let toys = crate::persistence::toy_store::load_in_transaction(txn, account);
    catalog.set_account_toy_ownership(&mut aggregate, &toys);
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
