//! Homebind is separate from CharacterData's frozen persistence ABI.
use crate::maps::WorldLocation;
use bevy::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition};

pub(crate) const HOME_BINDS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_home_binds");

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct HomeBind(pub WorldLocation);

type SavedBind = (u32, [f32; 3], f32);

pub(crate) fn save_bind_in(txn: &redb::WriteTransaction, character: u64, bind: WorldLocation) {
    let bytes = bitcode::encode(&(bind.map_id, bind.position.to_array(), bind.orientation));
    txn.open_table(HOME_BINDS)
        .expect("Open homebind table")
        .insert(character, bytes.as_slice())
        .expect("Save homebind");
}

#[cfg(test)]
pub(crate) fn save_bind(db: &Database, character: u64, bind: WorldLocation) {
    let txn = db.begin_write().expect("Begin homebind write");
    save_bind_in(&txn, character, bind);
    crate::persistence_commit::commit(txn).expect("Commit homebind");
}

pub(crate) fn load_bind(db: &Database, character: u64) -> Option<HomeBind> {
    let txn = db.begin_read().expect("Begin homebind read");
    let table = txn.open_table(HOME_BINDS).expect("Open homebind table");
    let row = table.get(character).expect("Read homebind")?;
    let (map_id, position, orientation): SavedBind =
        bitcode::decode(row.value()).expect("Decode homebind");
    Some(HomeBind(WorldLocation {
        map_id,
        position: Vec3::from_array(position),
        orientation,
    }))
}
