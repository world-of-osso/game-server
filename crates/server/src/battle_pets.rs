use redb::{Database, ReadableTable, TableDefinition};
use shared::pet_battle::PetJournal;

const ACCOUNT_PET_JOURNALS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("account_pet_journals");

/// Load by authenticated account, never character id. An absent journal is empty.
pub(crate) fn load_journal(db: &Database, account_id: u64) -> PetJournal {
    let transaction = db.begin_write().expect("begin pet journal load");
    let journal = {
        let table = transaction
            .open_table(ACCOUNT_PET_JOURNALS)
            .expect("open account pet journals");
        table
            .get(account_id)
            .expect("read account pet journal")
            .map(|value| serde_json::from_slice(value.value()).expect("decode account pet journal"))
            .unwrap_or_default()
    };
    transaction
        .commit()
        .expect("commit pet journal table initialization");
    journal
}

pub(crate) fn save_journal(db: &Database, account_id: u64, journal: &PetJournal) {
    let bytes = serde_json::to_vec(journal).expect("encode account pet journal");
    let transaction = db.begin_write().expect("begin account pet journal save");
    {
        let mut table = transaction
            .open_table(ACCOUNT_PET_JOURNALS)
            .expect("open account pet journals");
        table
            .insert(account_id, bytes.as_slice())
            .expect("save account pet journal");
    }
    transaction
        .commit()
        .expect("commit account pet journal save");
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::pet_battle::{PetJournal, PetJournalError, PetQuality};

    #[test]
    fn account_journal_survives_database_close_and_character_change() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("journal.redb");
        let first_character_account = 42;
        let other_character_account = 42;
        let first_id;
        {
            let db = redb::Database::create(&path).unwrap();
            let mut journal = load_journal(&db, first_character_account);
            first_id = journal
                .add_with_breed(39, 1, PetQuality::Common, 3)
                .unwrap();
            save_journal(&db, first_character_account, &journal);
        }
        let reopened = redb::Database::open(&path).unwrap();
        let mut journal = load_journal(&reopened, other_character_account);
        assert_eq!(journal.get(first_id).unwrap().species_id, 39);
        assert_eq!(journal.get(first_id).unwrap().breed_id, 3);
        journal
            .add_with_breed(39, 1, PetQuality::Common, 3)
            .unwrap();
        journal
            .add_with_breed(39, 1, PetQuality::Common, 3)
            .unwrap();
        assert_eq!(
            journal.add_with_breed(39, 1, PetQuality::Common, 3),
            Err(PetJournalError::SpeciesLimitReached)
        );
        assert_eq!(load_journal(&reopened, 43), PetJournal::default());
    }
}
