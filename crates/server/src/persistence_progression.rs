use redb::{Database, ReadableDatabase, TableDefinition};

use crate::trait_config::Purchase;

/// Class progression table: character_id -> serialized `ProgressionData`.
pub(crate) const CHARACTER_PROGRESSION: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_progression");

/// Experience table: character_id -> XP into the current level.
pub(crate) const CHARACTER_XP: TableDefinition<u64, u32> = TableDefinition::new("character_xp");

/// Flight style table: character_id -> its Flight Style aura spell (Steady 404468 or
/// Skyriding 404464), the aura Retail keeps across logins.
pub(crate) const CHARACTER_FLIGHT_STYLE: TableDefinition<u64, u32> =
    TableDefinition::new("character_flight_style");

/// Rested XP table: character_id -> serialized `RestData`.
pub(crate) const CHARACTER_REST: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_rest");

/// Trait configs table: character_id -> serialized `TraitConfigsData`.
pub(crate) const CHARACTER_TRAITS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_traits");

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionRefData {
    Spell(u32),
    Item(u32),
    Macro(u32),
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Eq)]
pub struct ProgressionData {
    /// Active `ChrSpecialization` ID.
    pub spec_id: u32,
    /// Occupied action bar slots, ascending.
    pub action_bar: Vec<(u8, ActionRefData)>,
}

pub fn load_progression(db: &Database, character_id: u64) -> Option<ProgressionData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(CHARACTER_PROGRESSION).ok()?;
    let guard = table
        .get(character_id)
        .expect("Failed to read character progression")?;
    Some(bitcode::decode(guard.value()).expect("Failed to decode character progression"))
}

pub fn save_progression(db: &Database, character_id: u64, data: &ProgressionData) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_PROGRESSION)
            .expect("Failed to open character_progression table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert character progression");
    }
    txn.commit()
        .expect("Failed to commit character progression save");
}

pub fn load_xp(db: &Database, character_id: u64) -> u32 {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(CHARACTER_XP) else {
        return 0;
    };
    table
        .get(character_id)
        .expect("Failed to read character xp")
        .map_or(0, |guard| guard.value())
}

pub fn save_xp(db: &Database, character_id: u64, xp: u32) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_XP)
            .expect("Failed to open character_xp table");
        table
            .insert(character_id, xp)
            .expect("Failed to insert character xp");
    }
    txn.commit().expect("Failed to commit character xp save");
}

/// The saved Flight Style aura spell; `None` before the character first switched.
pub fn load_flight_style(db: &Database, character_id: u64) -> Option<u32> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(CHARACTER_FLIGHT_STYLE).ok()?;
    table
        .get(character_id)
        .expect("Failed to read character flight style")
        .map(|guard| guard.value())
}

pub fn save_flight_style(db: &Database, character_id: u64, aura_spell: u32) {
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_FLIGHT_STYLE)
            .expect("Failed to open character_flight_style table");
        table
            .insert(character_id, aura_spell)
            .expect("Failed to insert character flight style");
    }
    txn.commit()
        .expect("Failed to commit character flight style save");
}

/// TrinityCore `characters.rest_bonus`, `logout_time` and `is_logout_resting`
/// (`Player::SaveToDB`): the rested XP pool, and when and whether resting the
/// character logged out.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestData {
    pub rested_xp: u32,
    pub logout_unix_secs: u64,
    pub logout_resting: bool,
}

pub fn load_rest(db: &Database, character_id: u64) -> Option<RestData> {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let table = txn.open_table(CHARACTER_REST).ok()?;
    let guard = table
        .get(character_id)
        .expect("Failed to read character rest")?;
    Some(bitcode::decode(guard.value()).expect("Failed to decode character rest"))
}

pub fn save_rest(db: &Database, character_id: u64, data: &RestData) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_REST)
            .expect("Failed to open character_rest table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert character rest");
    }
    txn.commit().expect("Failed to commit character rest save");
}

/// Saved trait purchases per `ChrSpecialization` ID, ascending by spec.
/// Granted ranks are not stored.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Default, PartialEq, Eq)]
pub struct TraitConfigsData {
    pub specs: Vec<(u32, Vec<Purchase>)>,
}

impl TraitConfigsData {
    pub fn spec(&self, spec_id: u32) -> &[Purchase] {
        self.specs
            .iter()
            .find(|(id, _)| *id == spec_id)
            .map_or(&[], |(_, purchases)| purchases.as_slice())
    }
}

pub fn load_trait_configs(db: &Database, character_id: u64) -> TraitConfigsData {
    let txn = db.begin_read().expect("Failed to begin read txn");
    let Ok(table) = txn.open_table(CHARACTER_TRAITS) else {
        return TraitConfigsData::default();
    };
    let guard = table
        .get(character_id)
        .expect("Failed to read character traits");
    guard.map_or_else(TraitConfigsData::default, |guard| {
        bitcode::decode(guard.value()).expect("Failed to decode character traits")
    })
}

/// Replace the saved config of one spec.
pub fn save_trait_config(db: &Database, character_id: u64, spec_id: u32, purchases: Vec<Purchase>) {
    let mut data = load_trait_configs(db, character_id);
    match data.specs.iter_mut().find(|(id, _)| *id == spec_id) {
        Some((_, saved)) => *saved = purchases,
        None => {
            data.specs.push((spec_id, purchases));
            data.specs.sort_by_key(|(id, _)| *id);
        }
    }
    let bytes = bitcode::encode(&data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    {
        let mut table = txn
            .open_table(CHARACTER_TRAITS)
            .expect("Failed to open character_traits table");
        table
            .insert(character_id, bytes.as_slice())
            .expect("Failed to insert character traits");
    }
    txn.commit()
        .expect("Failed to commit character traits save");
}
