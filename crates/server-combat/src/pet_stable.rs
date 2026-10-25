//! Retail Stable Master owner-scoped operations. NPC hello/gossip remains in npc_interaction.
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Npc, Player};
use shared::protocol::{
    NpcRole, PET_ACTIVE_SLOTS, PetStableChannel, PetStableContents, PetStableError,
    PetStableFailed, PetStableOperation, PetStableRequest, StablePetInfo,
};

use crate::auth::InWorld;
use crate::class_progression::KnownSpells;
use crate::hunter_pet::{
    HunterPet, PetCatalog, PetStable, init_pet_stable, persist_current_pet_health, remove_pet,
    save_stable,
};
use crate::networking::PlayerLink;
use crate::npc_interaction::ActiveInteraction;
use crate::persistence::pet_store::{StoredPet, StoredPetStable};
use crate::world_data::WorldData;

/// Installed Retail StableUI and SpellEffect.csv slot values.
const CALL_PET_SPELLS: [u32; PET_ACTIVE_SLOTS] = [883, 83242, 83243, 83244, 83245];
const HUNTER: u8 = 3;
const MIN_PET_NAME_LETTERS: usize = 2;
const MAX_PET_NAME_LETTERS: usize = 12;

pub(crate) fn process_pet_stable_requests(world: &mut World) {
    let requests: Vec<_> = world.query_filtered::<(Entity, &PlayerLink, &mut MessageReceiver<PetStableRequest>), With<InWorld>>()
        .iter_mut(world).flat_map(|(connection, link, mut receiver)| {
            receiver.receive().map(|request| (connection, link.0, request)).collect::<Vec<_>>()
        }).collect();
    for (connection, owner, request) in requests {
        let result = handle_request(world, connection, owner, &request);
        match result {
            Ok(contents) => {
                if let Some(mut sender) =
                    world.get_mut::<MessageSender<PetStableContents>>(connection)
                {
                    sender.send::<PetStableChannel>(contents);
                }
            }
            Err(error) => {
                if let Some(mut sender) =
                    world.get_mut::<MessageSender<PetStableFailed>>(connection)
                {
                    sender.send::<PetStableChannel>(PetStableFailed { error });
                }
            }
        }
    }
}

fn unlocked_slots(world: &World, owner: Entity) -> [bool; PET_ACTIVE_SLOTS] {
    let known = world.get::<KnownSpells>(owner);
    CALL_PET_SPELLS.map(|spell| known.is_some_and(|known| known.contains(spell)))
}

fn handle_request(
    world: &mut World,
    connection: Entity,
    owner: Entity,
    request: &PetStableRequest,
) -> Result<PetStableContents, PetStableError> {
    let interaction = world
        .get::<ActiveInteraction>(connection)
        .ok_or(PetStableError::NotInteracting)?;
    if interaction.npc.to_bits() != request.npc || interaction.role != Some(NpcRole::StableMaster) {
        return Err(PetStableError::NotInteracting);
    }
    if world.get::<Player>(owner).map(|player| player.class) != Some(HUNTER) {
        return Err(PetStableError::HunterOnly);
    }
    init_pet_stable(world, owner).ok_or(PetStableError::NotInteracting)?;
    persist_current_pet_health(world, owner);
    let unlocked = unlocked_slots(world, owner);
    if matches!(request.operation, PetStableOperation::List) {
        return stable_contents(world, owner, request.npc, unlocked);
    }
    let mut stored = world
        .get::<PetStable>(owner)
        .expect("initialized stable")
        .stored
        .clone();
    apply_operation(
        &mut stored,
        &request.operation,
        unlocked,
        world.resource::<PetCatalog>(),
    )?;
    apply_runtime_change(world, owner, stored);
    stable_contents(world, owner, request.npc, unlocked)
}

fn slot(stable: &StoredPetStable, index: u16) -> Result<&Option<StoredPet>, PetStableError> {
    let index = usize::from(index);
    if index < PET_ACTIVE_SLOTS {
        return Ok(&stable.active[index]);
    }
    stable
        .stabled
        .get(index - PET_ACTIVE_SLOTS)
        .ok_or(PetStableError::InvalidSlot)
}

fn slot_mut(
    stable: &mut StoredPetStable,
    index: u16,
) -> Result<&mut Option<StoredPet>, PetStableError> {
    let index = usize::from(index);
    if index < PET_ACTIVE_SLOTS {
        return Ok(&mut stable.active[index]);
    }
    stable
        .stabled
        .get_mut(index - PET_ACTIVE_SLOTS)
        .ok_or(PetStableError::InvalidSlot)
}

fn check_slot(
    stable: &StoredPetStable,
    index: u16,
    unlocked: [bool; PET_ACTIVE_SLOTS],
) -> Result<(), PetStableError> {
    slot(stable, index)?;
    if usize::from(index) < PET_ACTIVE_SLOTS && !unlocked[usize::from(index)] {
        return Err(PetStableError::SlotLocked);
    }
    Ok(())
}

fn apply_operation(
    stable: &mut StoredPetStable,
    operation: &PetStableOperation,
    unlocked: [bool; PET_ACTIVE_SLOTS],
    catalog: &PetCatalog,
) -> Result<(), PetStableError> {
    match operation {
        PetStableOperation::List => {}
        PetStableOperation::Swap { from, to } => swap_pets(stable, *from, *to, unlocked)?,
        PetStableOperation::Rename { slot: index, name } => {
            if !(MIN_PET_NAME_LETTERS..=MAX_PET_NAME_LETTERS).contains(&name.chars().count())
                || !name.chars().all(char::is_alphabetic)
            {
                return Err(PetStableError::InvalidName);
            }
            slot_mut(stable, *index)?
                .as_mut()
                .ok_or(PetStableError::PetNotFound)?
                .name = name.clone();
        }
        PetStableOperation::Abandon { slot: index } => {
            if slot_mut(stable, *index)?.take().is_none() {
                return Err(PetStableError::PetNotFound);
            }
            if stable.current.map(u16::from) == Some(*index) {
                stable.current = None
            }
        }
        PetStableOperation::Specialization {
            slot: index,
            specialization,
        } => {
            if !catalog
                .specializations
                .iter()
                .any(|spec| spec.id == *specialization)
            {
                return Err(PetStableError::InvalidSpecialization);
            }
            slot_mut(stable, *index)?
                .as_mut()
                .ok_or(PetStableError::PetNotFound)?
                .specialization = Some(*specialization);
        }
    }
    Ok(())
}

fn swap_pets(
    stable: &mut StoredPetStable,
    from: u16,
    to: u16,
    unlocked: [bool; PET_ACTIVE_SLOTS],
) -> Result<(), PetStableError> {
    check_slot(stable, from, unlocked)?;
    check_slot(stable, to, unlocked)?;
    if slot(stable, from)?.is_none() {
        return Err(PetStableError::PetNotFound);
    }
    if from == to {
        return Ok(());
    }
    let first = slot_mut(stable, from)?.take();
    let second = std::mem::replace(slot_mut(stable, to)?, first);
    *slot_mut(stable, from)? = second;
    stable.current = moved_current(stable.current, from, to);
    Ok(())
}

fn moved_current(current: Option<u8>, from: u16, to: u16) -> Option<u8> {
    let current = u16::from(current?);
    let next = if current == from {
        to
    } else if current == to {
        from
    } else {
        current
    };
    (usize::from(next) < PET_ACTIVE_SLOTS).then_some(next as u8)
}

fn apply_runtime_change(world: &mut World, owner: Entity, stored: StoredPetStable) {
    let pet = world
        .get::<PetStable>(owner)
        .expect("initialized stable")
        .summoned;
    let current = stored.current;
    let mut stable = world
        .get_mut::<PetStable>(owner)
        .expect("initialized stable");
    stable.stored = stored;
    if current.is_none() {
        stable.summoned = None
    }
    if let Some(pet) = pet {
        match current {
            None => remove_pet(world, pet),
            Some(index) => {
                world
                    .get_mut::<HunterPet>(pet)
                    .expect("owned stable pet")
                    .slot = index;
                let name = world.get::<PetStable>(owner).unwrap().stored.active[usize::from(index)]
                    .as_ref()
                    .unwrap()
                    .name
                    .clone();
                world.get_mut::<Npc>(pet).expect("pet identity").name = name;
            }
        }
    }
    save_stable(world, owner);
}

fn stable_contents(
    world: &World,
    owner: Entity,
    npc: u64,
    unlocked_active: [bool; PET_ACTIVE_SLOTS],
) -> Result<PetStableContents, PetStableError> {
    let stable = &world
        .get::<PetStable>(owner)
        .expect("initialized stable")
        .stored;
    let data = world.resource::<WorldData>();
    let catalog = world.resource::<PetCatalog>();
    let records = stable
        .active
        .iter()
        .chain(&stable.stabled)
        .enumerate()
        .filter_map(|(index, record)| record.as_ref().map(|record| (index, record)));
    let pets = records
        .map(|(index, record)| pet_info(data, catalog, index, record))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PetStableContents {
        npc,
        current: stable.current.map(u16::from),
        unlocked_active,
        pets,
        specializations: catalog.specializations.clone(),
    })
}

fn pet_info(
    data: &WorldData,
    catalog: &PetCatalog,
    index: usize,
    record: &StoredPet,
) -> Result<StablePetInfo, PetStableError> {
    let template = data
        .template_for(record.creature_entry, 0)
        .ok_or(PetStableError::PetNotFound)?;
    let max_health = crate::hunter_pet::pet_max_health(data, template, record.level);
    let family = catalog
        .families
        .get(&record.family)
        .ok_or(PetStableError::PetNotFound)?;
    Ok(StablePetInfo {
        slot: index as u16,
        creature_entry: record.creature_entry,
        display_id: record.display_id,
        name: record.name.clone(),
        level: record.level,
        family: family.0.clone(),
        family_icon: family.1,
        specialization: record.specialization,
        health: record.health.unwrap_or(max_health),
        max_health,
    })
}
