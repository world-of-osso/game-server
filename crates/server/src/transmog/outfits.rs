//! Account-scoped saved configurations; never invokes paid equipment application.
use super::*;
use redb::{ReadableDatabase, ReadableTable, TableDefinition};

const ACCOUNT_OUTFITS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("transmog_account_outfits");
const OUTFIT_SCHEMA: TableDefinition<u8, u32> =
    TableDefinition::new("transmog_account_outfits_schema");
const OUTFIT_VERSION: u32 = 1;

/// Pre-icon schema. Only the migration decodes this shape.
#[derive(serde::Deserialize)]
struct LegacyOutfit {
    id: u32,
    name: String,
    choices: Vec<TransmogOutfitChoice>,
    illusions: Vec<TransmogOutfitIllusion>,
}

fn migrate_outfits(db: &redb::Database) {
    let read = db.begin_read().expect("begin outfit schema read");
    match read.open_table(OUTFIT_SCHEMA) {
        Ok(table) => {
            let version = table
                .get(0)
                .expect("read outfit schema")
                .expect("missing outfit schema version")
                .value();
            assert_eq!(version, OUTFIT_VERSION, "unsupported account outfit schema");
            return;
        }
        Err(redb::TableError::TableDoesNotExist(_)) => {}
        Err(error) => panic!("open outfit schema: {error}"),
    }
    drop(read);
    let txn = db.begin_write().expect("begin outfit migration");
    {
        let mut schema = txn.open_table(OUTFIT_SCHEMA).expect("open outfit schema");
        // Another caller can finish migration before this write transaction starts.
        if let Some(version) = schema.get(0).expect("read outfit schema") {
            assert_eq!(
                version.value(),
                OUTFIT_VERSION,
                "unsupported account outfit schema"
            );
            return;
        }
        let mut table = txn
            .open_table(ACCOUNT_OUTFITS)
            .expect("open legacy outfits");
        let rows: Vec<_> = table
            .iter()
            .expect("scan legacy outfits")
            .map(|row| {
                let (account, bytes) = row.expect("read legacy outfits");
                let old: Vec<LegacyOutfit> =
                    serde_json::from_slice(bytes.value()).expect("decode legacy outfits");
                let outfits: Vec<_> = old
                    .into_iter()
                    .map(|old| TransmogOutfit {
                        id: old.id,
                        name: old.name,
                        icon_fdid: 0,
                        choices: old.choices,
                        illusions: old.illusions,
                    })
                    .collect();
                (
                    account.value(),
                    serde_json::to_vec(&outfits).expect("encode migrated outfits"),
                )
            })
            .collect();
        for (account, bytes) in rows {
            table
                .insert(account, bytes.as_slice())
                .expect("write migrated outfits");
        }
        schema
            .insert(0, OUTFIT_VERSION)
            .expect("write outfit schema version");
    }
    crate::persistence_commit::commit(txn).expect("commit outfit migration");
}

const AUTO_AWARDED: u8 = 1;
const AWARD_ON_LOGIN: u32 = 1;
const ALLOWS_ILLUSION: u32 = 2;

pub(super) fn load_outfits(db: &redb::Database, account: u64) -> Vec<TransmogOutfit> {
    migrate_outfits(db);
    let txn = db.begin_read().expect("begin account outfit read");
    let table = match txn.open_table(ACCOUNT_OUTFITS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Vec::new(),
        Err(error) => panic!("open account outfits: {error}"),
    };
    table
        .get(account)
        .expect("read account outfits")
        .map_or_else(Vec::new, |row| {
            serde_json::from_slice(row.value()).expect("decode account outfits")
        })
}

fn persist_outfit(db: &redb::Database, account: u64, outfit: TransmogOutfit) {
    let mut outfits = load_outfits(db, account);
    outfits.retain(|entry| entry.id != outfit.id);
    outfits.push(outfit);
    outfits.sort_by_key(|entry| entry.id);
    let bytes = serde_json::to_vec(&outfits).expect("encode account outfits");
    let txn = db.begin_write().expect("begin account outfit save");
    {
        let mut table = txn
            .open_table(ACCOUNT_OUTFITS)
            .expect("open account outfits");
        table
            .insert(account, bytes.as_slice())
            .expect("save account outfits");
    }
    crate::persistence_commit::commit(txn).expect("commit account outfits");
}

fn validate_outfit(
    state: &TransmogState,
    outfit: &TransmogOutfit,
) -> Result<(), TransmogOutfitError> {
    let entry = state
        .outfit_entries
        .iter()
        .find(|entry| entry.id == outfit.id)
        .ok_or(TransmogOutfitError::EntryUnavailable)?;
    let awarded = entry.source == AUTO_AWARDED && entry.flags & AWARD_ON_LOGIN != 0;
    if !awarded || entry.cost != 0 {
        return Err(TransmogOutfitError::EntryUnavailable);
    }
    let name = outfit.name.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err(TransmogOutfitError::InvalidName);
    }
    validate_choices(state, &outfit.choices)?;
    validate_illusions(state, &outfit.illusions)
}

pub(super) fn validate_choices(
    state: &TransmogState,
    choices: &[TransmogOutfitChoice],
) -> Result<(), TransmogOutfitError> {
    let mut seen = HashSet::new();
    for choice in choices {
        let slot = state
            .slots
            .iter()
            .find(|slot| slot.slot == choice.slot)
            .ok_or(TransmogOutfitError::InvalidChoice)?;
        if !seen.insert(choice.slot) || choice_modifier(slot, choice.choice).is_err() {
            return Err(TransmogOutfitError::InvalidChoice);
        }
    }
    Ok(())
}

fn validate_illusions(
    state: &TransmogState,
    illusions: &[TransmogOutfitIllusion],
) -> Result<(), TransmogOutfitError> {
    let mut seen = HashSet::new();
    for choice in illusions {
        let slot_allowed = state
            .outfit_slots
            .iter()
            .any(|slot| slot.slot == choice.slot && slot.flags & ALLOWS_ILLUSION != 0);
        let collected = state
            .illusions
            .iter()
            .any(|entry| entry.id == choice.illusion_id && entry.collected);
        let equipped = state.slots.iter().any(|slot| slot.slot == choice.slot);
        if !slot_allowed || !collected || !equipped || !seen.insert(choice.slot) {
            return Err(TransmogOutfitError::InvalidChoice);
        }
    }
    Ok(())
}

fn save_outfit(
    world: &mut World,
    connection: Entity,
    request: SaveTransmogOutfit,
) -> Result<TransmogState, TransmogOutfitError> {
    let (_, account, _) = service_character(world, connection, request.npc)
        .map_err(|_| TransmogOutfitError::NotInteracting)?;
    let mut state = handle(world, connection, request.npc, None)
        .map_err(|_| TransmogOutfitError::NotInteracting)?;
    validate_outfit(&state, &request.outfit)?;
    let db = &world.resource::<GameDatabase>().db;
    persist_outfit(db, account, request.outfit);
    state.outfits = load_outfits(db, account);
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transmog_outfit_legacy_rows_migrate_atomically_and_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy.redb");
        let db = redb::Database::create(&path).unwrap();
        let legacy = br#"[{"id":2,"name":"Gladius","choices":[{"slot":"MainHand","choice":{"Source":821}}],"illusions":[{"slot":"MainHand","illusion_id":3}]}]"#;
        let txn = db.begin_write().unwrap();
        {
            let mut table = txn.open_table(ACCOUNT_OUTFITS).unwrap();
            table.insert(1, legacy.as_slice()).unwrap();
            table.insert(2, legacy.as_slice()).unwrap();
        }
        crate::persistence_commit::commit(txn).unwrap();
        drop(db);
        let db = redb::Database::open(&path).unwrap();
        let migrated = load_outfits(&db, 1);
        assert_eq!(migrated[0].icon_fdid, 0);
        assert_eq!(migrated[0].choices[0].choice, TransmogChoice::Source(821));
        assert_eq!(migrated[0].illusions[0].illusion_id, 3);
        assert_eq!(load_outfits(&db, 2), migrated);
        assert!(load_outfits(&db, 3).is_empty());
        let mut edited = migrated[0].clone();
        edited.icon_fdid = 135343;
        persist_outfit(&db, 1, edited.clone());
        drop(db);
        let db = redb::Database::open(&path).unwrap();
        assert_eq!(load_outfits(&db, 1), vec![edited]);
        assert_eq!(load_outfits(&db, 2), migrated);
    }

    #[test]
    #[should_panic(expected = "unsupported account outfit schema")]
    fn transmog_outfit_unknown_version_is_not_silently_decoded() {
        let db = redb::Database::create(tempfile::NamedTempFile::new().unwrap().path()).unwrap();
        let txn = db.begin_write().unwrap();
        txn.open_table(OUTFIT_SCHEMA)
            .unwrap()
            .insert(0, 99)
            .unwrap();
        crate::persistence_commit::commit(txn).unwrap();
        load_outfits(&db, 1);
    }

    #[test]
    fn transmog_outfit_storage_survives_database_reopen_and_overwrites_entry() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("outfits.redb");
        let db = redb::Database::create(&path).unwrap();
        let mut outfit = TransmogOutfit {
            id: 2,
            name: "Gladius".into(),
            icon_fdid: 135274,
            choices: vec![TransmogOutfitChoice {
                slot: EquipmentSlot::MainHand,
                choice: TransmogChoice::Source(821),
            }],
            illusions: vec![TransmogOutfitIllusion {
                slot: EquipmentSlot::MainHand,
                illusion_id: 3,
            }],
        };
        persist_outfit(&db, 1, outfit.clone());
        outfit.name = "Sword".into();
        outfit.icon_fdid = 135343;
        persist_outfit(&db, 1, outfit.clone());
        drop(db);
        let reopened = redb::Database::open(&path).unwrap();
        assert_eq!(load_outfits(&reopened, 1), vec![outfit]);
        assert!(load_outfits(&reopened, 2).is_empty());
    }
}

pub(super) fn process_saves(world: &mut World) {
    for (connection, request) in collect_requests::<SaveTransmogOutfit>(world) {
        let npc = request.npc;
        let outfit = request.outfit.id;
        let result = save_outfit(world, connection, request);
        info!(
            "transmog outfit save connection={connection} outfit={outfit} result={:?}",
            result.as_ref().map(|_| ())
        );
        send_save_result(world, connection, npc, result);
    }
}

pub(super) fn send_save_result(
    world: &mut World,
    connection: Entity,
    npc: u64,
    result: Result<TransmogState, TransmogOutfitError>,
) {
    match result {
        Ok(state) => {
            if let Some(mut sender) = world.get_mut::<MessageSender<TransmogState>>(connection) {
                sender.send::<TransmogChannel>(state);
            }
        }
        Err(error) => {
            if let Some(mut sender) =
                world.get_mut::<MessageSender<TransmogOutfitFailed>>(connection)
            {
                sender.send::<TransmogChannel>(TransmogOutfitFailed { npc, error });
            }
        }
    }
}
