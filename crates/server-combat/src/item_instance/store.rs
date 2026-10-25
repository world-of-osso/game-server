//! Additive GUID-keyed modifiers; existing bitcode inventory rows stay unchanged.
use super::ItemInstanceModifiers;
use redb::{Database, ReadableDatabase, TableDefinition, WriteTransaction};

type StoreResult<T> = Result<T, Box<dyn std::error::Error>>;
pub const ITEM_MODIFIERS: TableDefinition<u64, &[u8]> = TableDefinition::new("item_modifiers");

pub fn initialize_store(txn: &WriteTransaction) -> StoreResult<()> {
    txn.open_table(ITEM_MODIFIERS)?;
    Ok(())
}

pub fn write_modifiers(
    txn: &WriteTransaction,
    guid: u64,
    state: &ItemInstanceModifiers,
) -> StoreResult<()> {
    txn.open_table(ITEM_MODIFIERS)?
        .insert(guid, bitcode::encode(state).as_slice())?;
    Ok(())
}

pub fn load_modifiers(db: &Database, guid: u64) -> StoreResult<ItemInstanceModifiers> {
    let txn = db.begin_read()?;
    let table = match txn.open_table(ITEM_MODIFIERS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Ok(ItemInstanceModifiers::default()),
        Err(error) => return Err(error.into()),
    };
    match table.get(guid)? {
        Some(row) => Ok(bitcode::decode(row.value())?),
        None => Ok(ItemInstanceModifiers::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_instance_modifiers_survive_database_reopen_and_owner_transfer_without_legacy_rewrite() {
        const LEGACY: TableDefinition<u64, &[u8]> = TableDefinition::new("item_instances");
        const OWNERS: TableDefinition<u64, u64> = TableDefinition::new("fixture_item_owner");
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        let txn = db.begin_write().unwrap();
        let legacy = bitcode::encode(&false);
        txn.open_table(LEGACY)
            .unwrap()
            .insert(41, legacy.as_slice())
            .unwrap();
        txn.open_table(OWNERS).unwrap().insert(41, 1).unwrap();
        initialize_store(&txn).unwrap();
        let state = ItemInstanceModifiers {
            bonus_list: vec![12066, 12214, 13622, 9627],
            item_level_override: Some(285),
            enchant: 7987,
            gems: vec![240983],
            crafted_stats: vec![32, 36],
        };
        write_modifiers(&txn, 41, &state).unwrap();
        txn.commit().unwrap();
        drop(db);
        let db = Database::open(file.path()).unwrap();
        assert_eq!(load_modifiers(&db, 41).unwrap(), state);
        let txn = db.begin_write().unwrap();
        txn.open_table(OWNERS).unwrap().insert(41, 2).unwrap();
        txn.commit().unwrap();
        drop(db);
        let db = Database::open(file.path()).unwrap();
        assert_eq!(load_modifiers(&db, 41).unwrap(), state);
        let txn = db.begin_read().unwrap();
        assert_eq!(
            txn.open_table(OWNERS)
                .unwrap()
                .get(41)
                .unwrap()
                .unwrap()
                .value(),
            2
        );
        assert_eq!(
            txn.open_table(LEGACY)
                .unwrap()
                .get(41)
                .unwrap()
                .unwrap()
                .value(),
            legacy.as_slice()
        );
        assert_eq!(
            load_modifiers(&db, 42).unwrap(),
            ItemInstanceModifiers::default()
        );
    }

    #[test]
    fn item_instance_modifier_write_rolls_back_with_the_owner_transaction() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Database::create(file.path()).unwrap();
        let txn = db.begin_write().unwrap();
        initialize_store(&txn).unwrap();
        txn.commit().unwrap();
        let original = ItemInstanceModifiers {
            enchant: 7987,
            ..Default::default()
        };
        let txn = db.begin_write().unwrap();
        write_modifiers(&txn, 41, &original).unwrap();
        txn.commit().unwrap();
        let txn = db.begin_write().unwrap();
        write_modifiers(
            &txn,
            41,
            &ItemInstanceModifiers {
                enchant: 8039,
                ..Default::default()
            },
        )
        .unwrap();
        txn.abort().unwrap();
        assert_eq!(load_modifiers(&db, 41).unwrap(), original);
    }
}
