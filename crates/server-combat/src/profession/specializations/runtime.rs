use super::{
    CommitSelection, PlayerProfessions, SpecializationActor, SpecializationData,
    SpecializationState,
};
use super::{build_snapshot, calculate_commit, store};
use crate::achievement::PlayerAchievementState;
use crate::auth::{Authenticated, InWorld};
use crate::currency::PlayerCurrencies;
use crate::networking::PlayerLink;
use crate::persistence::{ClientId, GameDatabase};
use crate::trait_config::Purchase;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{ActiveSpec, Level};
use shared::currency::CurrencyWallet;
use shared::protocol::{
    ProfessionChannel, ProfessionSpecializationRequest, ProfessionSpecializationStateUpdate,
};
use std::collections::BTreeSet;

#[derive(Component)]
struct ViewedSpecialization(u32);

pub(crate) struct SpecializationsPlugin;
impl Plugin for SpecializationsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, receive_requests);
    }
}

struct ActorState {
    character: u64,
    professions: PlayerProfessions,
    currencies: CurrencyWallet,
    achievements: BTreeSet<u32>,
    completed_quests: BTreeSet<u32>,
    level: u8,
    spec_id: u32,
}
impl ActorState {
    fn actor(&self) -> SpecializationActor<'_> {
        SpecializationActor {
            professions: &self.professions,
            currencies: &self.currencies,
            achievements: &self.achievements,
            completed_quests: self.completed_quests.clone(),
            level: self.level,
            spec_id: self.spec_id,
        }
    }
}

fn read_actor(world: &World, player: Entity) -> Result<ActorState, String> {
    let character = world
        .get::<ClientId>(player)
        .ok_or("Missing character identity")?
        .0;
    let database = world
        .get_resource::<GameDatabase>()
        .ok_or("Profession persistence is unavailable")?;
    let quests = crate::persistence::quest_store::load_quest_log(&database.db, character);
    Ok(ActorState {
        character,
        professions: world
            .get::<PlayerProfessions>(player)
            .ok_or("Missing profession state")?
            .clone(),
        currencies: world
            .get::<PlayerCurrencies>(player)
            .ok_or("Missing currency wallet")?
            .0
            .clone(),
        achievements: world
            .get::<PlayerAchievementState>(player)
            .ok_or("Missing achievement state")?
            .earned_ids
            .clone(),
        completed_quests: quests.rewarded.into_iter().collect(),
        level: world
            .get::<Level>(player)
            .ok_or("Missing character level")?
            .0,
        spec_id: world.get::<ActiveSpec>(player).map_or(0, |spec| spec.0),
    })
}

fn receive_requests(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &PlayerLink,
        Has<InWorld>,
        &mut MessageReceiver<ProfessionSpecializationRequest>,
    ), With<Authenticated>>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .filter(|(_, _, in_world, _)| *in_world)
        .flat_map(|(connection, link, _, mut receiver)| {
            let player = link.0;
            receiver
                .receive()
                .map(move |request| (connection, player, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, request) in requests {
        let result = if world.contains_resource::<SpecializationData>() {
            world.resource_scope(|world, catalog: Mut<SpecializationData>| {
                apply_request(world, &catalog, player, request)
            })
        } else {
            Err("Profession specialization source data is unavailable".into())
        };
        let update = match result {
            Ok(update) => update,
            Err(error) => ProfessionSpecializationStateUpdate {
                snapshot: None,
                message: None,
                error: Some(error),
            },
        };
        send_update(world, connection, update);
    }
}

fn apply_request(
    world: &mut World,
    catalog: &SpecializationData,
    player: Entity,
    request: ProfessionSpecializationRequest,
) -> Result<ProfessionSpecializationStateUpdate, String> {
    let mut actor = read_actor(world, player)?;
    let prior = read_specializations(world, player, actor.character);
    let (skill_line, message, state) = match request {
        ProfessionSpecializationRequest::Query { skill_line } => (skill_line, None, prior),
        ProfessionSpecializationRequest::Commit {
            skill_line,
            tree_id,
            purchases,
        } => {
            let purchases = purchases
                .into_iter()
                .map(|purchase| Purchase {
                    node_id: purchase.node_id,
                    entry_id: purchase.entry_id,
                    ranks: purchase.ranks,
                })
                .collect();
            let selection = CommitSelection {
                skill_line,
                tree_id,
                purchases,
            };
            let commit = calculate_commit(catalog, &prior, &actor.actor(), &selection)?;
            actor.professions = commit.professions.clone();
            actor.currencies = commit.currencies.clone();
            // Validate the owner-visible result before changing durable or live state.
            build_snapshot(catalog, &commit.state, &actor.actor(), skill_line)?;
            let database = world.resource::<GameDatabase>();
            store::save_commit(&database.db, actor.character, &commit);
            world.entity_mut(player).insert((
                commit.state.clone(),
                commit.professions,
                PlayerCurrencies(commit.currencies),
            ));
            (
                skill_line,
                Some("Applied profession specialization".into()),
                commit.state,
            )
        }
    };
    let snapshot = build_snapshot(catalog, &state, &actor.actor(), skill_line)?;
    world
        .entity_mut(player)
        .insert(ViewedSpecialization(skill_line));
    Ok(ProfessionSpecializationStateUpdate {
        snapshot: Some(snapshot),
        message,
        error: None,
    })
}

fn read_specializations(world: &mut World, player: Entity, character: u64) -> SpecializationState {
    if let Some(state) = world.get::<SpecializationState>(player) {
        return state.clone();
    }
    let database = world.resource::<GameDatabase>();
    let state = store::load_state(&database.db, character);
    world.entity_mut(player).insert(state.clone());
    state
}

fn send_update(world: &mut World, connection: Entity, update: ProfessionSpecializationStateUpdate) {
    let Some(mut sender) =
        world.get_mut::<MessageSender<ProfessionSpecializationStateUpdate>>(connection)
    else {
        warn!("Specialization response connection {connection} has no registered sender");
        return;
    };
    sender.send::<ProfessionChannel>(update);
}

pub(crate) fn reload_knowledge_wallet(world: &mut World, character: u64) {
    let mut players = world.query::<(Entity, &ClientId)>();
    let Some((player, _)) = players.iter(world).find(|(_, id)| id.0 == character) else {
        return;
    };
    let database = world.resource::<GameDatabase>();
    let currencies = crate::persistence::load_currencies(&database.db, character);
    world
        .entity_mut(player)
        .insert(PlayerCurrencies(currencies));
    refresh_viewed_knowledge(world, player);
}

fn refresh_viewed_knowledge(world: &mut World, player: Entity) {
    let Some(skill_line) = world.get::<ViewedSpecialization>(player).map(|view| view.0) else {
        return;
    };
    let request = ProfessionSpecializationRequest::Query { skill_line };
    let result = world.resource_scope(|world, catalog: Mut<SpecializationData>| {
        apply_request(world, &catalog, player, request)
    });
    let update = match result {
        Ok(update) => update,
        Err(error) => ProfessionSpecializationStateUpdate {
            snapshot: None,
            message: None,
            error: Some(error),
        },
    };
    let mut links = world.query_filtered::<(Entity, &PlayerLink), With<Authenticated>>();
    let connections: Vec<_> = links
        .iter(world)
        .filter(|(_, link)| link.0 == player)
        .map(|(entity, _)| entity)
        .collect();
    for connection in connections {
        send_update(world, connection, update.clone());
    }
}
