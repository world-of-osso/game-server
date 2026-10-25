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

#[cfg(test)]
#[path = "talent_loadout_persistence_tests.rs"]
mod talent_loadout_persistence_tests;

/// Versioned storage is deliberately separate from the legacy bitcode layout.
pub(crate) const CHARACTER_TRAIT_LOADOUTS: TableDefinition<u64, &[u8]> =
    TableDefinition::new("character_trait_loadouts_v1");

#[derive(bitcode::Decode)]
struct LegacyTraitConfigs {
    specs: Vec<(u32, Vec<Purchase>)>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, PartialEq, Eq)]
pub struct NamedTraitConfig {
    pub id: u32,
    pub name: String,
    pub purchases: Vec<Purchase>,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecTraitConfigs {
    pub spec_id: u32,
    pub selected_id: u32,
    pub default: Vec<Purchase>,
    pub configs: Vec<NamedTraitConfig>,
}
impl SpecTraitConfigs {
    pub fn active(&self) -> &[Purchase] {
        if self.selected_id == 0 || self.selected_id == shared::protocol::STARTER_BUILD_LOADOUT_ID {
            return &self.default;
        }
        &self
            .configs
            .iter()
            .find(|config| config.id == self.selected_id)
            .expect("Saved selected trait config is missing")
            .purchases
    }
    pub fn replace_active(&mut self, purchases: Vec<Purchase>) {
        if self.selected_id == 0 || self.selected_id == shared::protocol::STARTER_BUILD_LOADOUT_ID {
            self.default = purchases;
        } else {
            self.configs
                .iter_mut()
                .find(|config| config.id == self.selected_id)
                .expect("Saved selected trait config is missing")
                .purchases = purchases;
        }
    }
}

/// Saved named loadouts, unnamed defaults and selected IDs. Granted ranks are not stored.
#[derive(bitcode::Encode, bitcode::Decode, Debug, Clone, Default, PartialEq, Eq)]
pub struct TraitConfigsData {
    pub specs: Vec<SpecTraitConfigs>,
    pub next_id: u32,
}
impl TraitConfigsData {
    pub fn spec(&self, spec_id: u32) -> &[Purchase] {
        self.specs
            .iter()
            .find(|spec| spec.spec_id == spec_id)
            .map_or(&[], SpecTraitConfigs::active)
    }
    pub fn spec_mut(&mut self, spec_id: u32) -> &mut SpecTraitConfigs {
        if !self.specs.iter().any(|spec| spec.spec_id == spec_id) {
            self.specs.push(SpecTraitConfigs {
                spec_id,
                ..Default::default()
            });
            self.specs.sort_by_key(|spec| spec.spec_id);
        }
        self.specs
            .iter_mut()
            .find(|spec| spec.spec_id == spec_id)
            .unwrap()
    }
    pub fn create(
        &mut self,
        spec_id: u32,
        name: String,
        purchases: Vec<Purchase>,
    ) -> Result<u32, String> {
        validate_loadout_name(&name)?;
        let id = self.next_id.max(1);
        self.next_id = id.checked_add(1).ok_or("Trait config IDs exhausted")?;
        let spec = self.spec_mut(spec_id);
        spec.configs.push(NamedTraitConfig {
            id,
            name,
            purchases,
        });
        spec.selected_id = id;
        Ok(id)
    }
    pub fn rename(&mut self, spec_id: u32, id: u32, name: String) -> Result<(), String> {
        validate_loadout_name(&name)?;
        let config = self
            .spec_mut(spec_id)
            .configs
            .iter_mut()
            .find(|config| config.id == id)
            .ok_or("Unknown loadout for this specialization")?;
        config.name = name;
        Ok(())
    }
    pub fn delete(&mut self, spec_id: u32, id: u32) -> Result<(), String> {
        let spec = self.spec_mut(spec_id);
        let index = spec
            .configs
            .iter()
            .position(|config| config.id == id)
            .ok_or("Unknown loadout for this specialization")?;
        let removed = spec.configs.remove(index);
        // Retail deletes the saved copy, not the active talent allocation.
        if spec.selected_id == id {
            spec.default = removed.purchases;
            spec.selected_id = 0;
        }
        Ok(())
    }
    pub fn activate(&mut self, spec_id: u32, id: u32) -> Result<(), String> {
        let spec = self.spec_mut(spec_id);
        if !spec.configs.iter().any(|config| config.id == id) {
            return Err("Unknown loadout for this specialization".into());
        }
        spec.selected_id = id;
        Ok(())
    }
}
fn validate_loadout_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().count() > 30 || name.chars().any(char::is_control) {
        return Err("Loadout name must contain 1–30 printable characters".into());
    }
    Ok(())
}

pub fn load_trait_configs(db: &Database, character_id: u64) -> TraitConfigsData {
    let txn = db.begin_read().expect("Failed to begin read txn");
    if let Ok(table) = txn.open_table(CHARACTER_TRAIT_LOADOUTS) {
        if let Some(guard) = table.get(character_id).expect("Read trait loadouts") {
            return bitcode::decode(guard.value()).expect("Decode trait loadouts");
        }
    }
    let legacy = txn.open_table(CHARACTER_TRAITS).ok().and_then(|table| {
        table
            .get(character_id)
            .expect("Read legacy traits")
            .map(|guard| {
                bitcode::decode::<LegacyTraitConfigs>(guard.value()).expect("Decode legacy traits")
            })
    });
    let mut data = TraitConfigsData {
        next_id: 1,
        ..Default::default()
    };
    if let Some(legacy) = legacy {
        for (spec_id, purchases) in legacy.specs {
            data.spec_mut(spec_id).default = purchases;
        }
    }
    drop(txn);
    save_trait_configs(db, character_id, &data);
    data
}

pub fn save_trait_configs(db: &Database, character_id: u64, data: &TraitConfigsData) {
    let bytes = bitcode::encode(data);
    let txn = db.begin_write().expect("Failed to begin write txn");
    txn.open_table(CHARACTER_TRAIT_LOADOUTS)
        .expect("Open trait loadouts")
        .insert(character_id, bytes.as_slice())
        .expect("Insert trait loadouts");
    txn.commit().expect("Commit trait loadouts");
}

/// Apply edits to the selected saved loadout, or the unnamed default.
pub fn save_trait_config(db: &Database, character_id: u64, spec_id: u32, purchases: Vec<Purchase>) {
    let mut data = load_trait_configs(db, character_id);
    data.spec_mut(spec_id).replace_active(purchases);
    save_trait_configs(db, character_id, &data);
}
