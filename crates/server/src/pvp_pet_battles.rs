//! Find Battle matchmaking wraps the same deterministic engine as wild battles.
use super::*;

#[derive(Resource, Default)]
struct Matchmaker {
    queued: Vec<Entity>,
    proposals: BTreeMap<u64, Proposal>,
    battles: BTreeMap<u64, PvpBattle>,
}
#[derive(Clone)]
struct Proposal {
    owners: [Entity; 2],
    accepted: [bool; 2],
}
#[derive(Clone)]
struct PvpBattle {
    encounter: Encounter,
    owners: [Entity; 2],
    decisions: [Option<Action>; 2],
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
    build_team(world.resource::<WildCatalog>(), &identities)?;
    Ok(identities)
}
fn queue_request(
    world: &mut World,
    owner: Entity,
    request: PetBattleQueueRequest,
) -> Result<(), String> {
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
            world.resource_mut::<Matchmaker>().queued.push(owner);
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
    send_queue(world, owner, state(world, owner), None);
}
fn level_profile(world: &World, owner: Entity) -> Result<Vec<u8>, String> {
    let mut levels: Vec<_> = load_team(world, owner)?.iter().map(|p| p.level).collect();
    levels.sort_unstable();
    Ok(levels)
}
fn propose_matches(world: &mut World) {
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
            !matched.contains(*other) && level_profile(world, **other).is_ok_and(|p| p == profile)
        });
        let Some(opponent) = opponent else { continue };
        let owners = [*owner, *opponent];
        let id = next_id(world);
        world.resource_mut::<Matchmaker>().proposals.insert(
            id,
            Proposal {
                owners,
                accepted: [false; 2],
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
    let catalog = world.resource::<WildCatalog>();
    let teams = [
        build_team(catalog, &identities[0])?,
        build_team(catalog, &identities[1])?,
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
        rng: RuntimeRng(rand::rngs::StdRng::from_entropy()),
    };
    for (team, owner) in owners.iter().enumerate() {
        world.entity_mut(*owner).insert(PvpParticipant { id, team });
        send_queue(world, *owner, PetBattleQueueState::InBattle, None);
        let snapshot = snapshot(world, &battle, team);
        let connection = connection(world, *owner)?;
        send_update(world, connection, WildPetBattleUpdate::Start(snapshot));
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
    if team == 1 {
        state.teams.swap(0, 1);
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
    _world: &mut World,
    _owner: Entity,
    _request: WildPetBattleActionRequest,
) -> Result<(), String> {
    Err("PvP actions not implemented".into())
}
pub(super) fn is_in_battle(world: &World, owner: Entity) -> bool {
    world.get::<PvpParticipant>(owner).is_some()
}

#[cfg(test)]
#[path = "pvp_pet_battles_tests.rs"]
mod tests;
