//! Durable owned combat fields. Journal identity/level/XP stay in account_pet_journals.
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use server::pet_battle::{Catalog, calculate_stats};
use shared::pet_battle::OwnedPet;
use std::collections::BTreeMap;

const ACCOUNT_PET_BATTLE_STATES: TableDefinition<u64, &[u8]> =
    TableDefinition::new("account_pet_battle_states");
pub(crate) type OwnedBattleStates = BTreeMap<u64, OwnedPetBattleState>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OwnedPetBattleState {
    pub(crate) health: i32,
    pub(crate) selected_abilities: [u32; 3],
}
impl OwnedPetBattleState {
    pub(crate) fn initial(catalog: &Catalog, pet: &OwnedPet) -> Result<Self, String> {
        let stats = calculate_stats(
            catalog,
            pet.species_id,
            pet.breed_id.into(),
            pet.quality as u32,
            pet.level,
        )?;
        Ok(Self {
            health: stats.health,
            selected_abilities: default_abilities(catalog, pet.species_id, pet.level),
        })
    }
    pub(crate) fn select_ability(
        &mut self,
        catalog: &Catalog,
        pet: &OwnedPet,
        slot: u8,
        ability: u32,
    ) -> Result<(), String> {
        if !(1..=3).contains(&slot) {
            return Err("Ability slot must be 1..=3".into());
        }
        let option = catalog
            .species_abilities
            .iter()
            .find(|option| {
                option.species == pet.species_id
                    && option.slot == i32::from(slot - 1)
                    && option.ability == ability
            })
            .ok_or("Ability does not belong to this species and slot")?;
        if option.level > pet.level {
            return Err(format!(
                "Ability {ability} requires pet level {}",
                option.level
            ));
        }
        self.selected_abilities[usize::from(slot - 1)] = ability;
        Ok(())
    }
    /// A newly unlocked empty slot gets its first authored option; existing choices never reset.
    pub(crate) fn unlock_new_slots(&mut self, catalog: &Catalog, pet: &OwnedPet) -> bool {
        let available = default_abilities(catalog, pet.species_id, pet.level);
        let mut changed = false;
        for (selected, available) in self.selected_abilities.iter_mut().zip(available) {
            if *selected == 0 && available != 0 {
                *selected = available;
                changed = true;
            }
        }
        changed
    }
}

/// Existing wild-battle default choice, now used only for initialization and wild opponents.
pub(crate) fn default_abilities(catalog: &Catalog, species: u32, level: u8) -> [u32; 3] {
    std::array::from_fn(|slot| {
        catalog
            .species_abilities
            .iter()
            .filter(|option| {
                option.species == species && option.slot == slot as i32 && option.level <= level
            })
            .min_by_key(|option| (option.level, option.ability))
            .map_or(0, |option| option.ability)
    })
}

pub(crate) fn load_states(db: &Database, account: u64) -> Result<OwnedBattleStates, String> {
    let transaction = db
        .begin_write()
        .map_err(|e| format!("Begin pet state load: {e}"))?;
    let states = {
        let table = transaction
            .open_table(ACCOUNT_PET_BATTLE_STATES)
            .map_err(|e| format!("Open pet state table: {e}"))?;
        match table
            .get(account)
            .map_err(|e| format!("Read pet state for account {account}: {e}"))?
        {
            Some(row) => serde_json::from_slice(row.value())
                .map_err(|e| format!("Decode pet state for account {account}: {e}"))?,
            None => BTreeMap::new(),
        }
    };
    transaction
        .commit()
        .map_err(|e| format!("Commit pet state initialization: {e}"))?;
    Ok(states)
}
pub(crate) fn save_states(
    db: &Database,
    account: u64,
    states: &OwnedBattleStates,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(states)
        .map_err(|e| format!("Encode pet state for account {account}: {e}"))?;
    let transaction = db
        .begin_write()
        .map_err(|e| format!("Begin pet state save: {e}"))?;
    transaction
        .open_table(ACCOUNT_PET_BATTLE_STATES)
        .map_err(|e| format!("Open pet state table: {e}"))?
        .insert(account, bytes.as_slice())
        .map_err(|e| format!("Save pet state for account {account}: {e}"))?;
    transaction
        .commit()
        .map_err(|e| format!("Commit pet state for account {account}: {e}"))
}
