//! Homebind is separate from CharacterData's frozen persistence ABI.
use crate::maps::WorldLocation;
use bevy::prelude::*;
use redb::{Database, ReadableDatabase, TableDefinition};

pub(crate) const HOME_BINDS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_home_binds");

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct HomeBind(pub WorldLocation);

type SavedBind = (u32, [f32; 3], f32);

pub(crate) fn save_bind(db: &Database, character: u64, bind: WorldLocation) {
    let bytes = bitcode::encode(&(bind.map_id, bind.position.to_array(), bind.orientation));
    let txn = db.begin_write().expect("Begin homebind write");
    txn.open_table(HOME_BINDS)
        .expect("Open homebind table")
        .insert(character, bytes.as_slice())
        .expect("Save homebind");
    txn.commit().expect("Commit homebind");
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

/// New characters bind at their creation location. Pre-homebind characters have no
/// saved inn: initialize once at the existing race start, not their logout position.
pub(crate) fn load_or_initialize_bind(db: &Database, character: u64, race: u8) -> HomeBind {
    if let Some(bind) = load_bind(db, character) {
        return bind;
    }
    let start = crate::persistence_character::start_location(race);
    let location = WorldLocation {
        map_id: start.map_id,
        position: Vec3::new(start.x, start.y, start.z),
        orientation: 0.0,
    };
    save_bind(db, character, location);
    HomeBind(location)
}
