//! Owner-only journal details and acknowledged ability changes; no client-authored stats.
use super::{AccountPetJournal, PetCatalog};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;
use crate::pet_journal_state::{OwnedBattleStates, OwnedPetBattleState, load_states, save_states};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::pet_battle::{OwnedPet, PetJournal};
use shared::protocol::{
    CollectionChannel, OwnedPetBattleSnapshot, PetJournalAbility, PetJournalDetailsUpdate,
    SetBattlePetAbility,
};
use std::collections::{BTreeSet, btree_map::Entry};

pub(crate) fn load_or_initialize_pet_states(
    world: &World,
    owner: Entity,
) -> Result<OwnedBattleStates, String> {
    let account = world
        .get::<AccountPetJournal>(owner)
        .ok_or("Account pet journal unavailable")?;
    let db = &world.resource::<GameDatabase>().db;
    let mut states = load_states(db, account.account_id)?;
    if initialize_pet_states(
        world.resource::<PetCatalog>(),
        &account.journal,
        &mut states,
    )? {
        save_states(db, account.account_id, &states)?;
    }
    Ok(states)
}

/// Persist a completed battle transition before its state is published to the owner.
pub(crate) fn persist_battle_progress(
    world: &mut World,
    owner: Entity,
    journal: &PetJournal,
    health: &[(u64, i32)],
) -> Result<(), String> {
    let account = world
        .get::<AccountPetJournal>(owner)
        .ok_or("Account pet journal unavailable")?
        .account_id;
    let db = &world.resource::<GameDatabase>().db;
    let mut states = load_states(db, account)?;
    initialize_pet_states(world.resource::<PetCatalog>(), journal, &mut states)?;
    for (id, health) in health {
        states
            .get_mut(id)
            .ok_or_else(|| format!("Battle pet {id} has no owned combat state"))?
            .health = *health;
    }
    crate::pet_journal_state::save_journal_and_states(db, account, journal, &states)?;
    world
        .get_mut::<AccountPetJournal>(owner)
        .expect("journal checked")
        .journal = journal.clone();
    Ok(())
}

pub(super) fn initialize_pet_states(
    catalog: &PetCatalog,
    journal: &PetJournal,
    states: &mut OwnedBattleStates,
) -> Result<bool, String> {
    let owned: BTreeSet<_> = journal.pets.iter().map(|pet| pet.id).collect();
    let count = states.len();
    states.retain(|id, _| owned.contains(id));
    let mut changed = states.len() != count;
    for pet in &journal.pets {
        let species = catalog
            .species
            .get(&pet.species_id)
            .ok_or_else(|| format!("Owned pet species {} missing", pet.species_id))?;
        if !species.can_battle {
            continue;
        }
        let state = match states.entry(pet.id) {
            Entry::Vacant(entry) => {
                changed = true;
                entry.insert(OwnedPetBattleState::initial(&catalog.battle, pet)?)
            }
            Entry::Occupied(entry) => entry.into_mut(),
        };
        changed |= state.unlock_new_slots(&catalog.battle, pet);
    }
    Ok(changed)
}

impl PetCatalog {
    fn journal_details(
        &self,
        journal: &PetJournal,
        states: &OwnedBattleStates,
    ) -> Result<Vec<OwnedPetBattleSnapshot>, String> {
        let mut pets = Vec::new();
        for pet in &journal.pets {
            let species = self
                .species
                .get(&pet.species_id)
                .ok_or("Owned pet species missing")?;
            if !species.can_battle {
                continue;
            }
            let state = states
                .get(&pet.id)
                .ok_or_else(|| format!("Owned combat state missing for pet {}", pet.id))?;
            pets.push(self.owned_detail(pet, state)?);
        }
        Ok(pets)
    }
    fn owned_detail(
        &self,
        pet: &OwnedPet,
        state: &OwnedPetBattleState,
    ) -> Result<OwnedPetBattleSnapshot, String> {
        let stats = server_pet_battle::pet_battle::calculate_stats(
            &self.battle,
            pet.species_id,
            pet.breed_id.into(),
            pet.quality as u32,
            pet.level,
        )?;
        let next_level_xp = *self
            .next_level_xp
            .get(&pet.level)
            .ok_or_else(|| format!("Missing pet XP threshold at level {}", pet.level))?;
        Ok(OwnedPetBattleSnapshot {
            pet_id: pet.id,
            health: state.health,
            max_health: stats.health,
            power: stats.power,
            speed: stats.speed,
            next_level_xp,
            selected_abilities: state.selected_abilities,
            choices: self.ability_choices(pet.species_id)?,
        })
    }
    fn ability_choices(&self, species: u32) -> Result<Vec<PetJournalAbility>, String> {
        let mut choices = self
            .battle
            .species_abilities
            .iter()
            .filter(|option| option.species == species)
            .map(|option| {
                if !(0..3).contains(&option.slot) {
                    return Err(format!("Invalid species ability slot {}", option.slot));
                }
                let ability = self
                    .battle
                    .abilities
                    .get(&option.ability)
                    .ok_or("Species ability record missing")?;
                let (name, icon) = self
                    .ability_art
                    .get(&option.ability)
                    .ok_or("Species ability art record missing")?;
                Ok(PetJournalAbility {
                    id: option.ability,
                    name: name.clone(),
                    icon: *icon,
                    family: ability
                        .family
                        .try_into()
                        .map_err(|_| "Invalid ability family")?,
                    slot: (option.slot + 1) as u8,
                    required_level: option.level,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        choices.sort_by_key(|option| (option.slot, option.required_level, option.id));
        Ok(choices)
    }
}

pub(crate) fn send_pet_details(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    error: Option<String>,
) {
    if world
        .get::<MessageSender<PetJournalDetailsUpdate>>(connection)
        .is_none()
    {
        return;
    }
    let locked = crate::wild_pet_battles::journal_locked(world, owner);
    let result = load_or_initialize_pet_states(world, owner).and_then(|states| {
        let journal = &world
            .get::<AccountPetJournal>(owner)
            .ok_or("Account pet journal unavailable")?
            .journal;
        world
            .resource::<PetCatalog>()
            .journal_details(journal, &states)
    });
    let update = match result {
        Ok(pets) => PetJournalDetailsUpdate {
            pets,
            locked,
            error,
        },
        Err(error) => {
            warn!("Pet journal details for {owner:?}: {error}");
            PetJournalDetailsUpdate {
                pets: Vec::new(),
                locked,
                error: Some(error),
            }
        }
    };
    world
        .get_mut::<MessageSender<PetJournalDetailsUpdate>>(connection)
        .expect("sender checked")
        .send::<CollectionChannel>(update);
}

pub(super) fn receive_ability_requests(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &mut MessageReceiver<SetBattlePetAbility>,
    ), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, owner, request) in requests {
        let error = choose_ability(world, owner, request).err();
        send_pet_details(world, connection, owner, error);
    }
}
pub(crate) fn choose_ability(
    world: &mut World,
    owner: Entity,
    request: SetBattlePetAbility,
) -> Result<(), String> {
    if crate::wild_pet_battles::journal_locked(world, owner) {
        return Err("Cannot change abilities while queued or during a pet battle".into());
    }
    let mut states = load_or_initialize_pet_states(world, owner)?;
    let account = world
        .get::<AccountPetJournal>(owner)
        .ok_or("Account pet journal unavailable")?;
    let pet = account
        .journal
        .get(request.pet_id)
        .ok_or("Pet is not owned by this account")?;
    let state = states.get_mut(&request.pet_id).ok_or("Pet cannot battle")?;
    state.select_ability(
        &world.resource::<PetCatalog>().battle,
        pet,
        request.slot,
        request.ability_id,
    )?;
    save_states(
        &world.resource::<GameDatabase>().db,
        account.account_id,
        &states,
    )
}
