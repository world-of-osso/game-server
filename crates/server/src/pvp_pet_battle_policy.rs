//! Explicit project policies; numerical Retail authority is unavailable in the UI cache.
//! See docs/wiki/systems/petbattle-pvp.md (lead decision 2026-10-21).
use super::*;
use std::time::Duration;

pub(super) const PET_BATTLE_PVP_TIMERS: Duration = Duration::from_secs(30);
const PET_BATTLE_PVP_TIMEOUT_FORFEIT: u8 = 3;
const PET_BATTLE_PVP_MATCHMAKING: (u64, u64) = (3, 10);
const PET_BATTLE_PVP_XP: fn(u8, u8) -> u32 = earned_xp;

pub(super) fn now(world: &World) -> Duration {
    world.resource::<Time>().elapsed()
}

pub(super) fn sort_queue_by_wait(world: &mut World) {
    let mut matches = world.resource_mut::<Matchmaker>();
    let Matchmaker {
        queued, queued_at, ..
    } = &mut *matches;
    queued.sort_by_key(|owner| queued_at[owner]);
}

pub(super) fn compatible_levels(world: &World, owner: Entity, a: u16, b: u16) -> bool {
    let joined = world.resource::<Matchmaker>().queued_at[&owner];
    let waited = now(world).saturating_sub(joined).as_secs();
    let (base, widening_seconds) = PET_BATTLE_PVP_MATCHMAKING;
    let tolerance = base + waited / widening_seconds;
    // Both teams have three slots: compare sums, never rounded average levels.
    u64::from(a.abs_diff(b)) <= tolerance * 3
}

pub(super) fn expire_proposals(world: &mut World) {
    let time = now(world);
    let expired: Vec<_> = world
        .resource::<Matchmaker>()
        .proposals
        .iter()
        .filter(|(_, proposal)| time >= proposal.expires_at)
        .map(|(id, _)| *id)
        .collect();
    for id in expired {
        let proposal = world
            .resource_mut::<Matchmaker>()
            .proposals
            .remove(&id)
            .expect("listed proposal");
        for (team, owner) in proposal.owners.into_iter().enumerate() {
            let state = if proposal.accepted[team] {
                world.resource_mut::<Matchmaker>().queued.push(owner);
                PetBattleQueueState::Queued
            } else {
                world.resource_mut::<Matchmaker>().queued_at.remove(&owner);
                PetBattleQueueState::Idle
            };
            send_queue(world, owner, state, None);
        }
    }
}

pub(super) fn needs_replacement(battle: &PvpBattle) -> bool {
    battle
        .encounter
        .battle
        .teams
        .iter()
        .any(|team| team.pets[team.active].health <= 0)
}

pub(super) fn expire_battles(world: &mut World) {
    let time = now(world);
    let expired: Vec<_> = world
        .resource::<Matchmaker>()
        .battles
        .iter()
        .filter(|(_, battle)| time >= battle.expires_at)
        .map(|(id, battle)| (*id, battle.clone()))
        .collect();
    for (id, battle) in expired {
        let result = if battle.initial_pets.is_some() {
            select_expired_initial_pets(world, id)
        } else {
            resolve_expired_turn(world, id, battle)
        };
        if let Err(error) = result {
            error!("PvP pet battle {id} deadline: {error}");
        }
    }
}

fn select_expired_initial_pets(world: &mut World, id: u64) -> Result<(), String> {
    for team in 0..2 {
        let battle = world.resource::<Matchmaker>().battles[&id].clone();
        let required = battle
            .initial_pets
            .as_ref()
            .is_some_and(|picks| picks[team].is_none());
        if required {
            let slot = battle.encounter.battle.teams[team].active;
            select_initial_pet(world, id, battle, team, Action::Swap(slot))?;
        }
    }
    Ok(())
}

fn resolve_expired_turn(world: &mut World, id: u64, mut battle: PvpBattle) -> Result<(), String> {
    let replacing = needs_replacement(&battle);
    for team in 0..2 {
        if battle.decisions[team].is_some() {
            continue;
        }
        let pets = &battle.encounter.battle.teams[team];
        let action = if pets.pets[pets.active].health <= 0 {
            let slot = pets
                .pets
                .iter()
                .position(|pet| pet.health > 0)
                .ok_or("replacement deadline has no living pet")?;
            Action::Swap(slot)
        } else if replacing {
            Action::Pass
        } else {
            battle.consecutive_timeouts[team] += 1;
            if battle.consecutive_timeouts[team] >= PET_BATTLE_PVP_TIMEOUT_FORFEIT {
                Action::Forfeit
            } else {
                Action::Pass
            }
        };
        battle.decisions[team] = Some(action);
    }
    let actions = battle
        .decisions
        .map(|action| action.expect("deadline supplies missing decisions"));
    resolve_round(world, id, battle, actions)
}

pub(super) fn persist_winner_xp(
    world: &mut World,
    battle: &PvpBattle,
    outcome: Outcome,
) -> Result<[Vec<BattlePetXpReward>; 2], String> {
    let mut rewards = [vec![], vec![]];
    let Outcome::Winner(team) = outcome else {
        return Ok(rewards);
    };
    let owner = battle.owners[team];
    let before = world
        .get::<AccountPetJournal>(owner)
        .ok_or("winner journal unavailable")?
        .journal
        .clone();
    let mut journal = before.clone();
    rewards[team] =
        calculate_winner_xp(world.resource::<WildCatalog>(), battle, team, &mut journal)?;
    persist_battle_progress(world, owner, &journal, &battle.encounter.owned_health(team))?;
    crate::achievement::record_pet_level_changes(world, owner, &before, &journal);
    refresh_catalog_snapshot(world, owner);
    Ok(rewards)
}

fn calculate_winner_xp(
    catalog: &WildCatalog,
    battle: &PvpBattle,
    team: usize,
    journal: &mut PetJournal,
) -> Result<Vec<BattlePetXpReward>, String> {
    let eligible: Vec<_> = battle.participated[team]
        .iter()
        .copied()
        .filter(|slot| {
            battle.encounter.battle.teams[team].pets[*slot].health > 0
                && battle.encounter.identities[team][*slot].level < MAX_LEVEL
        })
        .collect();
    let mut rewards = Vec::new();
    for slot in &eligible {
        let guid = battle.encounter.identities[team][*slot]
            .guid
            .expect("owned PvP pet");
        let pet = journal
            .pets
            .iter_mut()
            .find(|pet| pet.id == guid)
            .ok_or("XP pet no longer owned")?;
        let total: u32 = battle.encounter.identities[1 - team]
            .iter()
            .map(|opponent| PET_BATTLE_PVP_XP(pet.level, opponent.level))
            .sum();
        let gained = total / eligible.len() as u32;
        if gained != 0 {
            rewards.push(grant_xp(catalog, pet, gained)?);
        }
    }
    Ok(rewards)
}

fn grant_xp(
    catalog: &WildCatalog,
    pet: &mut OwnedPet,
    gained: u32,
) -> Result<BattlePetXpReward, String> {
    pet.xp += gained;
    while pet.level < MAX_LEVEL {
        let threshold = catalog
            .next_level_xp
            .get(&pet.level)
            .ok_or("missing pet XP game-table level")?;
        if pet.xp < *threshold {
            break;
        }
        pet.xp -= threshold;
        pet.level += 1;
    }
    if pet.level == MAX_LEVEL {
        pet.xp = 0;
    }
    Ok(BattlePetXpReward {
        instance_id: pet.id,
        xp_gained: gained,
        level: pet.level,
        xp: pet.xp,
        next_level_xp: catalog.next_level_xp[&pet.level],
    })
}
