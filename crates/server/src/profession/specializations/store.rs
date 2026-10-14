use super::{SpecializationCommit, SpecializationState};
use redb::{Database, ReadableDatabase, TableDefinition, WriteTransaction};

const STATES: TableDefinition<u64, &[u8]> = TableDefinition::new("profession_specializations");

pub(crate) fn initialize_store(transaction: &WriteTransaction) {
    transaction
        .open_table(STATES)
        .expect("initialize profession specializations");
}

pub(super) fn load_state(database: &Database, character: u64) -> SpecializationState {
    let transaction = database
        .begin_read()
        .expect("read profession specialization state");
    let table = transaction
        .open_table(STATES)
        .expect("open profession specializations");
    table
        .get(character)
        .expect("read profession specialization row")
        .map(|row| bitcode::decode(row.value()).expect("decode profession specialization state"))
        .unwrap_or_default()
}

pub(super) fn save_commit(database: &Database, character: u64, commit: &SpecializationCommit) {
    let transaction = database
        .begin_write()
        .expect("begin profession specialization commit");
    write_commit_in(&transaction, character, commit);
    crate::persistence_commit::commit(transaction)
        .expect("commit specialization ranks, knowledge and recipe spells");
}

pub(super) fn write_commit_in(
    transaction: &WriteTransaction,
    character: u64,
    commit: &SpecializationCommit,
) {
    transaction
        .open_table(STATES)
        .expect("open profession specializations")
        .insert(character, bitcode::encode(&commit.state).as_slice())
        .expect("write profession specialization ranks");
    transaction
        .open_table(crate::persistence::CURRENCIES)
        .expect("open currencies")
        .insert(
            character,
            serde_json::to_vec(&commit.currencies)
                .expect("encode profession currency wallet")
                .as_slice(),
        )
        .expect("write profession knowledge");
    transaction
        .open_table(crate::persistence::PROFESSIONS)
        .expect("open professions")
        .insert(character, bitcode::encode(&commit.professions).as_slice())
        .expect("write specialization recipe spells");
}

pub(crate) fn write_currency_wallet(
    transaction: &WriteTransaction,
    character: u64,
    wallet: &shared::currency::CurrencyWallet,
) {
    transaction
        .open_table(crate::persistence::CURRENCIES)
        .expect("open currencies")
        .insert(
            character,
            serde_json::to_vec(wallet)
                .expect("encode currency wallet")
                .as_slice(),
        )
        .expect("write quest knowledge currency");
}
