//! Player-created presets are not DB2-authored outfit slots.
use super::*;
use redb::{ReadableDatabase, TableDefinition};

const CUSTOM_SETS: TableDefinition<u64, &[u8]> = TableDefinition::new("transmog_custom_sets");

pub(super) fn load_sets(db: &redb::Database, account: u64) -> Vec<TransmogCustomSet> {
    let txn = db.begin_read().expect("begin custom set read");
    let table = match txn.open_table(CUSTOM_SETS) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => return Vec::new(),
        Err(error) => panic!("open custom sets: {error}"),
    };
    table
        .get(account)
        .expect("read custom sets")
        .map_or_else(Vec::new, |row| {
            serde_json::from_slice(row.value()).expect("decode custom sets")
        })
}

fn write_sets(db: &redb::Database, account: u64, sets: &[TransmogCustomSet]) {
    let bytes = serde_json::to_vec(sets).expect("encode custom sets");
    let txn = db.begin_write().expect("begin custom set save");
    {
        let mut table = txn.open_table(CUSTOM_SETS).expect("open custom sets");
        table
            .insert(account, bytes.as_slice())
            .expect("save custom sets");
    }
    crate::persistence_commit::commit(txn).expect("commit custom sets");
}

fn save_set(
    world: &mut World,
    connection: Entity,
    request: SaveTransmogCustomSet,
) -> Result<TransmogState, TransmogOutfitError> {
    let (_, account, _) = service_character(world, connection, request.npc)
        .map_err(|_| TransmogOutfitError::NotInteracting)?;
    let mut state = handle(world, connection, request.npc, None)
        .map_err(|_| TransmogOutfitError::NotInteracting)?;
    if request.name.trim().is_empty() || request.name.chars().any(char::is_control) {
        return Err(TransmogOutfitError::InvalidName);
    }
    if request.choices.is_empty() {
        return Err(TransmogOutfitError::InvalidChoice);
    }
    outfits::validate_choices(&state, &request.choices)?;
    let db = &world.resource::<GameDatabase>().db;
    let id = match request.id {
        Some(id) if state.custom_sets.iter().any(|set| set.id == id) => id,
        Some(_) => return Err(TransmogOutfitError::EntryUnavailable),
        None => crate::persistence::next_id(db, "transmog_custom_set"),
    };
    state.custom_sets.retain(|set| set.id != id);
    state.custom_sets.push(TransmogCustomSet {
        id,
        name: request.name,
        choices: request.choices,
    });
    write_sets(db, account, &state.custom_sets);
    Ok(state)
}

pub(super) fn process_saves(world: &mut World) {
    for (connection, request) in collect_requests::<SaveTransmogCustomSet>(world) {
        let npc = request.npc;
        let set = request.id;
        let result = save_set(world, connection, request);
        info!(
            "transmog custom set save connection={connection} set={set:?} result={:?}",
            result.as_ref().map(|_| ())
        );
        outfits::send_save_result(world, connection, npc, result);
    }
}
