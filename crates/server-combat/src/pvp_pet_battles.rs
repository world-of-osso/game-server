//! Find Battle matchmaking wraps the same deterministic engine as wild battles.
use super::*;
#[path = "pvp_pet_battle_policy.rs"]
mod policy;
use policy::*;

#[derive(Resource, Default)]
struct Matchmaker {
    queued: Vec<Entity>,
    queued_at: BTreeMap<Entity, (std::time::Duration, u64)>,
    next_queue_ticket: u64,
    proposals: BTreeMap<u64, Proposal>,
    battles: BTreeMap<u64, PvpBattle>,
}
#[derive(Clone)]
struct Proposal {
    owners: [Entity; 2],
    accepted: [bool; 2],
    expires_at: std::time::Duration,
}
#[derive(Clone)]
struct PvpBattle {
    encounter: Encounter,
    owners: [Entity; 2],
    decisions: [Option<Action>; 2],
    initial_pets: Option<[Option<usize>; 2]>,
    expires_at: std::time::Duration,
    consecutive_timeouts: [u8; 2],
    participated: [BTreeSet<usize>; 2],
    rng: RuntimeRng,
}
#[derive(Component, Clone, Copy)]
struct PvpParticipant {
    id: u64,
    team: usize,
}

pub(super) fn busy(world: &World, owner: Entity) -> bool {
    let Some(matches) = world.get_resource::<Matchmaker>() else {
        return false;
    };
    matches.queued.contains(&owner)
        || matches
            .proposals
            .values()
            .any(|p| p.owners.contains(&owner))
        || world.get::<PvpParticipant>(owner).is_some()
}
fn connection(world: &mut World, owner: Entity) -> Result<Entity, String> {
    let mut query =
        world.query_filtered::<(Entity, &PlayerLink), (With<Authenticated>, With<InWorld>)>();
    query
        .iter(world)
        .find(|(_, link)| link.0 == owner)
        .map(|(entity, _)| entity)
        .ok_or("player connection unavailable".into())
}
fn send_queue(world: &mut World, owner: Entity, state: PetBattleQueueState, error: Option<String>) {
    let Ok(connection) = connection(world, owner) else {
        return;
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<PetBattleQueueUpdate>>(connection) {
        sender.send::<CollectionChannel>(PetBattleQueueUpdate { state, error });
    }
}
fn state(world: &World, owner: Entity) -> PetBattleQueueState {
    if world.get::<PvpParticipant>(owner).is_some() {
        return PetBattleQueueState::InBattle;
    }
    let matches = world.resource::<Matchmaker>();
    for (id, proposal) in &matches.proposals {
        if let Some(team) = proposal.owners.iter().position(|o| *o == owner) {
            return PetBattleQueueState::Proposal {
                proposal_id: *id,
                accepted: proposal.accepted[team],
            };
        }
    }
    if matches.queued.contains(&owner) {
        PetBattleQueueState::Queued
    } else {
        PetBattleQueueState::Idle
    }
}
fn load_team(world: &World, owner: Entity) -> Result<Vec<Identity>, String> {
    if world.get::<ActiveEncounter>(owner).is_some() {
        return Err("already in a wild pet battle".into());
    }
    if world.get::<Health>(owner).is_none_or(|h| h.current <= 0.0) {
        return Err("player must be alive".into());
    }
    let journal = &world
        .get::<AccountPetJournal>(owner)
        .ok_or("account journal unavailable")?
        .journal;
    let identities = journal
        .battle_slots
        .iter()
        .map(|slot| {
            let id = slot.ok_or("equip all three battle slots")?;
            journal
                .get(id)
                .map(Identity::from)
                .ok_or("loadout pet no longer owned")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let states = load_or_initialize_pet_states(world, owner)?;
    build_team(world.resource::<WildCatalog>(), &identities, &states)?;
    Ok(identities)
}
fn queue_request(
    world: &mut World,
    owner: Entity,
    request: PetBattleQueueRequest,
) -> Result<(), String> {
    expire_proposals(world);
    connection(world, owner)?;
    match request {
        PetBattleQueueRequest::Join => {
            if busy(world, owner) {
                return Err("already queued or in a pet battle".into());
            }
            load_team(world, owner)?;
            let account = world
                .get::<AccountPetJournal>(owner)
                .ok_or("journal unavailable")?
                .account_id;
            let mut query = world.query::<(Entity, &AccountPetJournal)>();
            if query.iter(world).any(|(other, journal)| {
                other != owner && journal.account_id == account && busy(world, other)
            }) {
                return Err("account already queued or in a pet battle".into());
            }
            let joined_at = now(world);
            {
                let mut matches = world.resource_mut::<Matchmaker>();
                let ticket = matches.next_queue_ticket;
                matches.next_queue_ticket += 1;
                matches.queued_at.insert(owner, (joined_at, ticket));
                matches.queued.push(owner);
            }
            send_queue(world, owner, PetBattleQueueState::Queued, None);
        }
        PetBattleQueueRequest::Leave => cancel_queue(world, owner),
        PetBattleQueueRequest::Decline { proposal_id } => {
            validate_proposal(world, owner, proposal_id)?;
            cancel_queue(world, owner);
        }
        PetBattleQueueRequest::Accept { proposal_id } => {
            let team = validate_proposal(world, owner, proposal_id)?;
            load_team(world, owner)?;
            let mut matches = world.resource_mut::<Matchmaker>();
            let proposal = matches
                .proposals
                .get_mut(&proposal_id)
                .expect("validated proposal");
            if proposal.accepted[team] {
                return Err("proposal already accepted".into());
            }
            proposal.accepted[team] = true;
            send_queue(world, owner, state(world, owner), None);
        }
    }
    Ok(())
}
fn validate_proposal(world: &World, owner: Entity, id: u64) -> Result<usize, String> {
    world
        .resource::<Matchmaker>()
        .proposals
        .get(&id)
        .and_then(|p| p.owners.iter().position(|o| *o == owner))
        .ok_or("stale queue proposal".into())
}
fn cancel_queue(world: &mut World, owner: Entity) {
    world
        .resource_mut::<Matchmaker>()
        .queued
        .retain(|o| *o != owner);
    let ids: Vec<_> = world
        .resource::<Matchmaker>()
        .proposals
        .iter()
        .filter(|(_, p)| p.owners.contains(&owner))
        .map(|(id, _)| *id)
        .collect();
    for id in ids {
        let proposal = world
            .resource_mut::<Matchmaker>()
            .proposals
            .remove(&id)
            .expect("listed proposal");
        for other in proposal.owners {
            if other != owner {
                world.resource_mut::<Matchmaker>().queued.push(other);
                send_queue(world, other, PetBattleQueueState::Queued, None);
            }
        }
    }
    world.resource_mut::<Matchmaker>().queued_at.remove(&owner);
    send_queue(world, owner, state(world, owner), None);
}
fn level_profile(world: &World, owner: Entity) -> Result<u16, String> {
    Ok(load_team(world, owner)?
        .iter()
        .map(|p| u16::from(p.level))
        .sum())
}
fn propose_matches(world: &mut World) {
    sort_queue_by_wait(world);
    let queue = world.resource::<Matchmaker>().queued.clone();
    let mut matched = BTreeSet::new();
    for (index, owner) in queue.iter().enumerate() {
        if matched.contains(owner) {
            continue;
        }
        let Ok(profile) = level_profile(world, *owner) else {
            continue;
        };
        let opponent = queue[index + 1..].iter().find(|other| {
            !matched.contains(*other)
                && level_profile(world, **other)
                    .is_ok_and(|p| compatible_levels(world, *owner, profile, p))
        });
        let Some(opponent) = opponent else { continue };
        let owners = [*owner, *opponent];
        let id = next_id(world);
        let expires_at = now(world) + PET_BATTLE_PVP_TIMERS;
        world.resource_mut::<Matchmaker>().proposals.insert(
            id,
            Proposal {
                owners,
                accepted: [false; 2],
                expires_at,
            },
        );
        for owner in owners {
            matched.insert(owner);
            send_queue(
                world,
                owner,
                PetBattleQueueState::Proposal {
                    proposal_id: id,
                    accepted: false,
                },
                None,
            );
        }
    }
    world
        .resource_mut::<Matchmaker>()
        .queued
        .retain(|o| !matched.contains(o));
}
fn next_id(world: &mut World) -> u64 {
    let mut ids = world.resource_mut::<BattleIds>();
    ids.0 += 1;
    ids.0
}
fn start_match(world: &mut World, id: u64, owners: [Entity; 2]) -> Result<(), String> {
    let identities = [load_team(world, owners[0])?, load_team(world, owners[1])?];
    let states = [
        load_or_initialize_pet_states(world, owners[0])?,
        load_or_initialize_pet_states(world, owners[1])?,
    ];
    let catalog = world.resource::<WildCatalog>();
    let teams = [
        build_team(catalog, &identities[0], &states[0])?,
        build_team(catalog, &identities[1], &states[1])?,
    ];
    let encounter = Encounter {
        id,
        creature: 0,
        battle: Battle::new(teams)?,
        identities,
        participated: BTreeSet::new(),
        trap_failures: 0,
        captured: None,
        forfeited: false,
        feedback: vec![],
    };
    let battle = PvpBattle {
        encounter,
        owners,
        decisions: [None; 2],
        initial_pets: Some([None; 2]),
        expires_at: now(world) + PET_BATTLE_PVP_TIMERS,
        consecutive_timeouts: [0; 2],
        participated: Default::default(),
        rng: RuntimeRng(rand::rngs::StdRng::from_entropy()),
    };
    for (team, owner) in owners.iter().enumerate() {
        world.resource_mut::<Matchmaker>().queued_at.remove(owner);
        world.entity_mut(*owner).insert(PvpParticipant { id, team });
        send_queue(world, *owner, PetBattleQueueState::InBattle, None);
        let snapshot = snapshot(world, &battle, team);
        let connection = connection(world, *owner)?;
        send_update(world, connection, WildPetBattleUpdate::Start(snapshot));
        send_pet_details(world, connection, *owner, None);
    }
    info!("PvP pet battle {id} started between {:?}", owners);
    world
        .resource_mut::<Matchmaker>()
        .battles
        .insert(id, battle);
    Ok(())
}
fn snapshot(world: &World, battle: &PvpBattle, team: usize) -> WildPetBattleSnapshot {
    let journal = &world
        .get::<AccountPetJournal>(battle.owners[team])
        .expect("validated journal")
        .journal;
    let mut state = battle
        .encounter
        .snapshot(world.resource::<WildCatalog>(), journal);
    state.can_trap = false;
    state.turn_time_ms = battle.expires_at.saturating_sub(now(world)).as_millis() as u32;
    state.initial_selection_required = battle.initial_pets.is_some();
    if team == 1 {
        state.teams.swap(0, 1);
        state.team_auras.swap(0, 1);
        state.active.swap(0, 1);
        for feedback in &mut state.feedback {
            feedback.team = 1 - feedback.team;
        }
    }
    state.replacement_required = state.teams[0][usize::from(state.active[0])].health <= 0;
    state
}

pub(super) fn receive_queue(world: &mut World) {
    if !world.contains_resource::<Matchmaker>() {
        world.init_resource::<Matchmaker>();
    }
    expire_proposals(world);
    expire_battles(world);
    let mut query = world.query_filtered::<(&PlayerLink, &mut MessageReceiver<PetBattleQueueRequest>), (With<Authenticated>, With<InWorld>)>();
    let requests: Vec<_> = query
        .iter_mut(world)
        .flat_map(|(link, mut receiver)| {
            receiver.receive().map(|r| (link.0, r)).collect::<Vec<_>>()
        })
        .collect();
    for (owner, request) in requests {
        if let Err(error) = queue_request(world, owner, request) {
            send_queue(world, owner, state(world, owner), Some(error));
        }
    }
    propose_matches(world);
    let ready: Vec<_> = world
        .resource::<Matchmaker>()
        .proposals
        .iter()
        .filter(|(_, p)| p.accepted == [true; 2])
        .map(|(id, p)| (*id, p.owners))
        .collect();
    for (id, owners) in ready {
        world.resource_mut::<Matchmaker>().proposals.remove(&id);
        if let Err(error) = start_match(world, id, owners) {
            for owner in owners {
                send_queue(world, owner, PetBattleQueueState::Idle, Some(error.clone()));
            }
        }
    }
}

pub(super) fn advance(
    world: &mut World,
    owner: Entity,
    request: WildPetBattleActionRequest,
) -> Result<(), String> {
    expire_battles(world);
    let participant = *world
        .get::<PvpParticipant>(owner)
        .ok_or("no PvP pet battle")?;
    let mut battle = world.resource::<Matchmaker>().battles[&participant.id].clone();
    if request.battle_id != participant.id || request.round != battle.encounter.battle.round {
        return Err("stale battle or round".into());
    }
    let team = participant.team;
    if battle.decisions[team].is_some() && request.action != WildPetBattleAction::Forfeit {
        return Err("decision already submitted".into());
    }
    let action = player_action(&battle.encounter, team, request.action)?;
    if action == Action::Forfeit {
        let mut actions = [Action::Pass; 2];
        actions[team] = action;
        return resolve_round(world, participant.id, battle, actions);
    }
    if battle.initial_pets.is_some() {
        return select_initial_pet(world, participant.id, battle, team, action);
    }
    validate_decision(world.resource::<WildCatalog>(), &battle, team, action)?;
    battle.decisions[team] = Some(action);
    if !needs_replacement(&battle) {
        battle.consecutive_timeouts[team] = 0;
    }
    for side in 0..2 {
        let other = 1 - side;
        let dead = battle.encounter.battle.teams[side].pets
            [battle.encounter.battle.teams[side].active]
            .health
            <= 0;
        if dead && battle.decisions[other].is_none() {
            let other_team = &battle.encounter.battle.teams[other];
            if other_team.pets[other_team.active].health > 0 {
                battle.decisions[other] = Some(Action::Pass);
            }
        }
    }
    if let [Some(a), Some(b)] = battle.decisions {
        resolve_round(world, participant.id, battle, [a, b])
    } else {
        world
            .resource_mut::<Matchmaker>()
            .battles
            .insert(participant.id, battle);
        Ok(())
    }
}
fn select_initial_pet(
    world: &mut World,
    id: u64,
    mut battle: PvpBattle,
    team: usize,
    action: Action,
) -> Result<(), String> {
    let Action::Swap(slot) = action else {
        return Err("choose an initial pet before taking a turn".into());
    };
    let pets = &battle.encounter.battle.teams[team].pets;
    if pets.get(slot).is_none_or(|pet| pet.health <= 0) {
        return Err("initial pet must be alive".into());
    }
    let choices = battle
        .initial_pets
        .as_mut()
        .expect("selection phase checked");
    if choices[team].is_some() {
        return Err("initial pet already selected".into());
    }
    choices[team] = Some(slot);
    if let [Some(a), Some(b)] = *choices {
        for (team, slot) in [a, b].into_iter().enumerate() {
            battle.encounter.battle.teams[team].active = slot;
        }
        battle.initial_pets = None;
        battle.expires_at = now(world) + PET_BATTLE_PVP_TIMERS;
        for side in 0..2 {
            let state = snapshot(world, &battle, side);
            let connection = connection(world, battle.owners[side])?;
            send_update(world, connection, WildPetBattleUpdate::State(state));
        }
    }
    world
        .resource_mut::<Matchmaker>()
        .battles
        .insert(id, battle);
    Ok(())
}

fn player_action(
    encounter: &Encounter,
    team: usize,
    decision: WildPetBattleAction,
) -> Result<Action, String> {
    let pet = &encounter.battle.teams[team].pets[encounter.battle.teams[team].active];
    match decision {
        WildPetBattleAction::Ability(button) if (1..=3).contains(&button) => {
            let id = pet.abilities[usize::from(button - 1)];
            if id == 0 {
                return Err("ability slot locked".into());
            }
            Ok(Action::Ability(id))
        }
        WildPetBattleAction::Swap(slot) if (1..=3).contains(&slot) => {
            Ok(Action::Swap(usize::from(slot - 1)))
        }
        WildPetBattleAction::Pass => Ok(Action::Pass),
        WildPetBattleAction::Forfeit => Ok(Action::Forfeit),
        WildPetBattleAction::Trap => Err("cannot trap another player's pet".into()),
        _ => Err("invalid battle action slot".into()),
    }
}
fn validate_decision(
    catalog: &WildCatalog,
    battle: &PvpBattle,
    team: usize,
    action: Action,
) -> Result<(), String> {
    // Trial uses a cloned RNG/state: rejected or waiting decisions never advance the real engine.
    let other = 1 - team;
    let opponent = &battle.encounter.battle.teams[other];
    let other_action = if opponent.pets[opponent.active].health <= 0 {
        let slot = opponent
            .pets
            .iter()
            .position(|p| p.health > 0)
            .ok_or("opponent has no living pet")?;
        Action::Swap(slot)
    } else {
        Action::Pass
    };
    let mut actions = [Action::Pass; 2];
    actions[team] = action;
    actions[other] = other_action;
    battle
        .encounter
        .battle
        .step(&catalog.turns, actions, &mut battle.rng.clone())
        .map(|_| ())
}
fn resolve_round(
    world: &mut World,
    id: u64,
    mut battle: PvpBattle,
    actions: [Action; 2],
) -> Result<(), String> {
    let combat_round = battle.initial_pets.is_none()
        && !needs_replacement(&battle)
        && !actions.contains(&Action::Forfeit);
    let before_active = battle.encounter.battle.teams.each_ref().map(|t| t.active);
    let (next, events) = battle.encounter.battle.step(
        &world.resource::<WildCatalog>().turns,
        actions,
        &mut battle.rng,
    )?;
    battle.encounter.battle = next;
    if combat_round {
        for (team, slot) in before_active.into_iter().enumerate() {
            battle.participated[team].insert(slot);
            battle.participated[team].insert(battle.encounter.battle.teams[team].active);
        }
    }
    battle.expires_at = now(world) + PET_BATTLE_PVP_TIMERS;
    battle.encounter.feedback = events.iter().filter_map(combat_feedback).collect();
    let text: Vec<_> = events
        .iter()
        .filter_map(|event| format_combat_event(event, world.resource::<WildCatalog>()))
        .collect();
    battle.decisions = [None; 2];
    for team in 0..2 {
        let owner = battle.owners[team];
        let journal = world
            .get::<AccountPetJournal>(owner)
            .ok_or("Account pet journal unavailable")?
            .journal
            .clone();
        persist_battle_progress(world, owner, &journal, &battle.encounter.owned_health(team))?;
    }
    if let Some(outcome) = battle.encounter.battle.outcome {
        finish_match(world, id, &battle, outcome, actions, text)?;
    } else {
        for team in 0..2 {
            let owner = battle.owners[team];
            let state = snapshot(world, &battle, team);
            if let Ok(connection) = connection(world, owner) {
                send_update(
                    world,
                    connection,
                    WildPetBattleUpdate::Round {
                        state,
                        combat_text: text.clone(),
                    },
                );
            }
        }
        world
            .resource_mut::<Matchmaker>()
            .battles
            .insert(id, battle);
    }
    Ok(())
}
fn finish_match(
    world: &mut World,
    id: u64,
    battle: &PvpBattle,
    outcome: Outcome,
    actions: [Action; 2],
    text: Vec<String>,
) -> Result<(), String> {
    let rewards = persist_winner_xp(world, battle, outcome)?;
    for team in 0..2 {
        let owner = battle.owners[team];
        if outcome == Outcome::Winner(team) && actions[team] != Action::Forfeit {
            crate::achievement::record_pet_battle_win(world, owner);
        }
        if let Ok(connection) = connection(world, owner) {
            let state = snapshot(world, battle, team);
            send_update(world, connection, WildPetBattleUpdate::State(state));
            let result = if actions[team] == Action::Forfeit {
                WildPetBattleOutcome::Forfeited
            } else {
                match outcome {
                    Outcome::Winner(winner) if winner == team => WildPetBattleOutcome::Won,
                    Outcome::Winner(_) => WildPetBattleOutcome::Lost,
                    Outcome::Draw => WildPetBattleOutcome::Draw,
                }
            };
            send_update(
                world,
                connection,
                WildPetBattleUpdate::End {
                    battle_id: id,
                    outcome: result,
                    rewards: rewards[team].clone(),
                    captured_pet_id: None,
                    combat_text: text.clone(),
                },
            );
        }
        if let Ok(mut entity) = world.get_entity_mut(owner) {
            entity.remove::<PvpParticipant>();
        }
        if let Ok(connection) = connection(world, owner) {
            send_pet_update(world, connection, owner, None);
        }
        send_queue(world, owner, PetBattleQueueState::Idle, None);
    }
    world.resource_mut::<Matchmaker>().battles.remove(&id);
    info!("PvP pet battle {id} ended: {outcome:?}");
    Ok(())
}
pub(super) fn remove_abandoned(world: &mut World) {
    let Some(matches) = world.get_resource::<Matchmaker>() else {
        return;
    };
    let owners: BTreeSet<_> = matches
        .queued
        .iter()
        .copied()
        .chain(matches.proposals.values().flat_map(|p| p.owners))
        .collect();
    let battles: Vec<_> = matches
        .battles
        .iter()
        .map(|(id, battle)| (*id, battle.clone()))
        .collect();
    for owner in owners {
        if connection(world, owner).is_err() || load_team(world, owner).is_err() {
            cancel_queue(world, owner);
        }
    }
    for (id, battle) in battles {
        let missing = battle.owners.map(|owner| connection(world, owner).is_err());
        if !missing.iter().any(|v| *v) {
            continue;
        }
        let actions = missing.map(|v| if v { Action::Forfeit } else { Action::Pass });
        if let Err(error) = resolve_round(world, id, battle, actions) {
            error!("PvP disconnect: {error}");
        }
    }
}

pub(super) fn is_in_battle(world: &World, owner: Entity) -> bool {
    world.get::<PvpParticipant>(owner).is_some()
}

#[cfg(test)]
#[path = "pvp_pet_battles_tests.rs"]
mod tests;
