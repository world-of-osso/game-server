//! Authenticated Retail housing requests. Static membership is never an ownership grant.
use crate::auth::{Authenticated, InWorld};
use crate::housing_catalog::HousingCatalog;
use crate::maps::{InMap, InstanceManager, MapStore, TeleportOptions};
use crate::networking::PlayerLink;
use crate::persistence::{GameDatabase, housing_store as store};
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::Position;
use shared::protocol::{
    HousingChannel, HousingCommand, HousingReply, HousingRequest, HousingState,
};

#[path = "housing_dye_requests.rs"]
mod dye_requests;
#[path = "housing_persistence.rs"]
mod persistence_actor;
#[path = "housing_transfers.rs"]
mod transfers;
pub(crate) type HousingTransferReceipt =
    crate::persistence_writer::CommitReceipt<transfers::Transfer>;
pub(crate) use persistence_actor::HousingPersistencePlugin;
#[cfg(test)]
pub(crate) fn persistence_pending(world: &World) -> bool {
    persistence_actor::persistence_pending(world) || transfers::pending(world)
}

#[derive(Resource)]
pub(crate) struct HousingAvailability(pub(crate) Result<HousingCatalog, String>);

#[derive(Clone, Copy)]
struct Actor {
    connection: Entity,
    player: Entity,
    account: u64,
    character: u64,
}

pub(crate) struct HousingPlugin;
impl Plugin for HousingPlugin {
    fn build(&self, app: &mut App) {
        HousingPersistencePlugin::install(app);
        app.add_systems(Startup, load_catalog)
            .add_systems(Update, process_requests);
    }
}

fn load_catalog(mut commands: Commands) {
    let connection = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    );
    let (loaded, collection) = match connection {
        Ok(db) => (
            HousingCatalog::read(&db),
            crate::housing_acquisition::read_collection_rules(&db),
        ),
        Err(error) => {
            let message = format!("Open housing catalog: {error}");
            (Err(message.clone()), Err(message))
        }
    };
    if let Err(message) = &loaded {
        warn!("Housing unavailable: {message}");
    }
    commands.insert_resource(HousingAvailability(loaded));
    commands.insert_resource(crate::housing_acquisition::HousingCollectionRules(
        collection,
    ));
}

pub(crate) fn spawn_neighborhood_objects(
    commands: &mut Commands,
    source: &HousingAvailability,
    map: InMap,
) {
    if let Ok(catalog) = &source.0 {
        let objects = crate::game_objects::GameObjectSpawns(catalog.neighborhood_objects());
        crate::game_objects::spawn_map_game_objects(commands, &objects, map, 0, None);
    }
}

fn catalog(world: &World) -> Result<&HousingCatalog, String> {
    world
        .resource::<HousingAvailability>()
        .0
        .as_ref()
        .map_err(Clone::clone)
}

fn receive_requests(world: &mut World) -> Vec<(Actor, HousingRequest)> {
    let mut query = world.query::<(
        Entity,
        &InWorld,
        &Authenticated,
        &PlayerLink,
        &mut MessageReceiver<HousingRequest>,
    )>();
    query
        .iter_mut(world)
        .flat_map(|(connection, entered, auth, link, mut receiver)| {
            let actor = Actor {
                connection,
                character: entered.character_id,
                account: auth.account_id,
                player: link.0,
            };
            receiver
                .receive()
                .map(move |request| (actor, request))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn process_requests(world: &mut World) {
    for (actor, request) in receive_requests(world) {
        if transfers::enqueue_request(world, actor, &request)
            || persistence_actor::enqueue_request(world, actor, &request)
        {
            continue;
        }
        let result = execute(world, actor, &request.command);
        let (neighborhood, error) = match result {
            Ok(neighborhood) => (neighborhood, None),
            Err(message) => (None, Some(message)),
        };
        let house = match request.command {
            HousingCommand::ReloadLayout { house_id } => Some(house_id),
            _ => None,
        };
        send_snapshot(world, actor, request.request_id, neighborhood, house, error);
    }
}

fn publish_layout_at(world: &mut World, map: InMap, excluded: Option<Entity>) {
    let mut query = world.query::<(Entity, &InWorld, &Authenticated, &PlayerLink)>();
    let observers: Vec<_> = query
        .iter(world)
        .filter(|(connection, _, _, link)| {
            Some(*connection) != excluded && world.get::<InMap>(link.0) == Some(&map)
        })
        .map(|(connection, entered, auth, link)| Actor {
            connection,
            player: link.0,
            account: auth.account_id,
            character: entered.character_id,
        })
        .collect();
    for observer in observers {
        send_snapshot(world, observer, 0, None, None, None);
    }
}

fn actor_context(world: &World, actor: Actor) -> Result<store::Actor, String> {
    let Some(&map) = world.get::<InMap>(actor.player) else {
        if world
            .get::<crate::maps::MapTransfer>(actor.player)
            .is_none()
        {
            return Err("Finish entering the world before using housing.".into());
        }
        return Ok(store::Actor {
            account_id: actor.account,
            character_id: actor.character,
            neighborhood_id: None,
            inside_house: None,
        });
    };
    let property = world.resource::<InstanceManager>().housing_property(map);
    let (neighborhood_id, inside_house) = match map.map_id {
        2735 | 2736 => (property.and_then(|id| u32::try_from(id).ok()), None),
        2783 => (None, property),
        _ => (None, None),
    };
    Ok(store::Actor {
        account_id: actor.account,
        character_id: actor.character,
        neighborhood_id,
        inside_house,
    })
}

fn execute(
    world: &mut World,
    actor: Actor,
    command: &HousingCommand,
) -> Result<Option<u32>, String> {
    catalog(world)?;
    let context = actor_context(world, actor)?;
    match command {
        HousingCommand::ListPlots { neighborhood_id } => {
            catalog(world)?.neighborhood(*neighborhood_id)?;
            return Ok(Some(*neighborhood_id));
        }
        HousingCommand::EnterNeighborhood { .. } | HousingCommand::VisitPlot { .. } => {
            unreachable!("location requests are queued before synchronous commands")
        }
        _ => apply_house_command(world, actor, context, command)?,
    }
    Ok(actor_context(world, actor)?.neighborhood_id)
}

fn apply_house_command(
    world: &mut World,
    actor: Actor,
    context: store::Actor,
    command: &HousingCommand,
) -> Result<(), String> {
    match command {
        HousingCommand::PurchasePlot { .. }
        | HousingCommand::RedeemDecor { .. }
        | HousingCommand::SaveHouseAccess { .. }
        | HousingCommand::CommitDecorDyes { .. }
        | HousingCommand::PlaceDecor { .. }
        | HousingCommand::TransformDecor { .. }
        | HousingCommand::MoveDecor { .. }
        | HousingCommand::RotateDecor { .. }
        | HousingCommand::RemoveDecor { .. } => {
            unreachable!("durable mutations are queued before synchronous house commands")
        }
        HousingCommand::EnterHouse { .. } | HousingCommand::LeaveHouse => {
            unreachable!("location requests are queued before synchronous commands")
        }
        HousingCommand::PreviewDecorDyes {
            house_id,
            instance_id,
            changes,
        } => dye_requests::preview(world, actor, context, *house_id, *instance_id, changes)?,
        HousingCommand::CancelDecorDyes => dye_requests::cancel(world, actor),
        HousingCommand::ReloadLayout { house_id } => {
            store::house(
                &world.resource::<GameDatabase>().db,
                context,
                *house_id,
                world.get_resource::<crate::group::GroupRegistry>(),
            )?;
        }
        HousingCommand::ListPlots { .. }
        | HousingCommand::EnterNeighborhood { .. }
        | HousingCommand::VisitPlot { .. } => {
            unreachable!("neighborhood commands handled before house operations")
        }
    }
    Ok(())
}

pub(crate) fn collect_authenticated_item(
    world: &mut World,
    character: u64,
    player: Entity,
    guid: u64,
) -> Result<(), String> {
    let mut query = world.query::<(Entity, &Authenticated, &PlayerLink, &InWorld)>();
    let actor = query
        .iter(world)
        .find(|(_, _, link, entered)| link.0 == player && entered.character_id == character)
        .map(|(connection, auth, _, _)| Actor {
            connection,
            account: auth.account_id,
            character,
            player,
        })
        .ok_or("Housing collection requires an authenticated character.")?;
    persistence_actor::enqueue_collection(
        world,
        actor,
        guid,
        persistence_actor::ReplyOrigin::ItemUse,
    )
}

fn transfer_property(
    world: &mut World,
    actor: Actor,
    destination: crate::maps::WorldLocation,
    property_id: u64,
) -> Result<(), String> {
    if world.get::<InMap>(actor.player).is_none() {
        return Err("Finish the current map transfer first.".into());
    }
    world
        .resource_scope(|world, mut manager: Mut<InstanceManager>| {
            manager.authorize_housing_entry(
                world.resource::<MapStore>(),
                destination.map_id,
                property_id,
                actor.character,
            )
        })
        .map_err(|reason| format!("Housing admission refused: {reason:?}"))?;
    if !crate::maps::teleport_to(world, actor.player, destination, TeleportOptions::default()) {
        world
            .resource_mut::<InstanceManager>()
            .revoke_housing_entry(actor.character);
        return Err("Housing transfer was refused.".into());
    }
    Ok(())
}

/// GAMEOBJECT_TYPE_UI_LINK use: a plot cornerstone or the neighborhood bulletin board.
pub(crate) fn use_ui_link(
    world: &mut World,
    connection: Entity,
    player: Entity,
    object: Entity,
) -> Result<(), shared::protocol::InteractionError> {
    use shared::protocol::InteractionError;
    let actor = authenticate_object_actor(world, connection, player, object)?;
    let entry = world
        .get::<shared::protocol::GameObjectInfo>(object)
        .ok_or(InteractionError::InvalidTarget)?
        .entry;
    let source = catalog(world).map_err(|_| InteractionError::InvalidTarget)?;
    let cornerstone = source.cornerstone_plot(entry);
    let board = source.bulletin_board_neighborhood(entry);
    let mut state =
        read_snapshot(world, actor, None, None).map_err(|_| InteractionError::InvalidTarget)?;
    if let Some(plot_id) = cornerstone {
        require_cornerstone_reach(world, player, plot_id)
            .map_err(|_| InteractionError::TooFarAway)?;
        state.opened_cornerstone = Some(plot_id);
    } else if let Some(neighborhood_id) = board {
        let roster = read_roster(world, neighborhood_id).map_err(|error| {
            warn!("Neighborhood {neighborhood_id} roster unavailable: {error}");
            InteractionError::InvalidTarget
        })?;
        state.bulletin_board = Some(roster);
    } else {
        return Err(InteractionError::InvalidTarget);
    }
    let mut sender = world
        .get_mut::<MessageSender<HousingReply>>(connection)
        .ok_or(InteractionError::InvalidTarget)?;
    sender.send::<HousingChannel>(HousingReply {
        request_id: 0,
        state,
        error: None,
    });
    Ok(())
}

/// Every house owner in this neighborhood, by plot, with live presence.
fn read_roster(
    world: &mut World,
    neighborhood_id: u32,
) -> Result<shared::protocol::NeighborhoodRoster, String> {
    let mut plots = catalog(world)?.plots(neighborhood_id)?;
    let db = &world.resource::<GameDatabase>().db;
    store::populate_plots(db, &mut plots)?;
    let owners: Vec<_> = plots
        .iter()
        .filter_map(|plot| Some((plot.plot_index, plot.owner_character_id?)))
        .map(|(plot_index, character)| {
            let name = crate::persistence::load_character(db, character)
                .ok_or(format!("House owner character {character} is missing."))?
                .name;
            Ok((plot_index, character, name))
        })
        .collect::<Result<_, String>>()?;
    let mut query = world.query::<&InWorld>();
    let online: std::collections::HashSet<u64> = query
        .iter(world)
        .map(|entered| entered.character_id)
        .collect();
    // Retail's client keeps the received roster as its alphabetical member list.
    let mut residents: Vec<_> = owners
        .into_iter()
        .map(
            |(plot_index, character, name)| shared::protocol::NeighborhoodResident {
                name,
                online: online.contains(&character),
                plot_index,
            },
        )
        .collect();
    residents.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(shared::protocol::NeighborhoodRoster {
        neighborhood_id,
        residents,
    })
}

fn authenticate_object_actor(
    world: &World,
    connection: Entity,
    player: Entity,
    object: Entity,
) -> Result<Actor, shared::protocol::InteractionError> {
    use shared::protocol::InteractionError;
    let entered = world
        .get::<InWorld>(connection)
        .ok_or(InteractionError::InvalidTarget)?;
    let auth = world
        .get::<Authenticated>(connection)
        .ok_or(InteractionError::InvalidTarget)?;
    let link = world
        .get::<PlayerLink>(connection)
        .ok_or(InteractionError::InvalidTarget)?;
    let map = world
        .get::<InMap>(player)
        .ok_or(InteractionError::InvalidTarget)?;
    if link.0 != player || world.get::<InMap>(object) != Some(map) {
        return Err(InteractionError::InvalidTarget);
    }
    Ok(Actor {
        connection,
        player,
        account: auth.account_id,
        character: entered.character_id,
    })
}

fn require_cornerstone_reach(
    world: &mut World,
    player: Entity,
    plot_id: u32,
) -> Result<(), String> {
    let position = *world
        .get::<Position>(player)
        .ok_or("Player position unavailable.")?;
    let map = *world
        .get::<InMap>(player)
        .ok_or("Enter the neighborhood before using its cornerstone.")?;
    let source = catalog(world)?;
    let entry = source
        .cornerstones()
        .into_iter()
        .find(|spawn| source.cornerstone_plot(spawn.info.entry) == Some(plot_id))
        .map(|spawn| spawn.info.entry)
        .ok_or("This plot has no authenticated cornerstone source.")?;
    let mut objects = world.query::<(
        &InMap,
        &shared::protocol::GameObjectInfo,
        &Position,
        &crate::game_objects::GameObjectBounds,
    )>();
    let stone = objects
        .iter(world)
        .find(|(copy, info, _, _)| **copy == map && info.entry == entry);
    let Some((_, _, origin, bounds)) = stone else {
        return Err("This plot's cornerstone is not present in this neighborhood copy.".into());
    };
    let reaches = bounds.reaches(
        crate::game_objects::wow_position(origin),
        crate::game_objects::wow_position(&position),
        crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
    );
    if !reaches {
        return Err("Move within interaction range of this plot's authored cornerstone.".into());
    }
    Ok(())
}

fn read_snapshot(
    world: &World,
    actor: Actor,
    neighborhood: Option<u32>,
    house: Option<u64>,
) -> Result<HousingState, String> {
    let context = actor_context(world, actor)?;
    let db = &world.resource::<GameDatabase>().db;
    let mut state = store::snapshot(
        db,
        context,
        house.or(context.inside_house),
        world.get_resource::<crate::group::GroupRegistry>(),
    )?;
    if let Some(neighborhood) = neighborhood.or(context.neighborhood_id) {
        state.plots = catalog(world)?.plots(neighborhood)?;
        store::populate_plots(db, &mut state.plots)?;
        load_plot_relationships(db, actor.character, &mut state);
    }
    dye_requests::attach_preview(world, actor, &mut state);
    Ok(state)
}

fn load_plot_relationships(db: &redb::Database, character: u64, state: &mut HousingState) {
    use shared::protocol::HousingPlotOwnerType;
    let friends = crate::persistence::load_friends(db, character);
    for plot in &mut state.plots {
        plot.owner_type = match plot.house_id {
            None => HousingPlotOwnerType::None,
            Some(house_id)
                if state
                    .owned_houses
                    .iter()
                    .any(|house| house.house_id == house_id) =>
            {
                HousingPlotOwnerType::SelfOwned
            }
            Some(_) => {
                let owner = plot
                    .owner_character_id
                    .and_then(|id| crate::persistence::load_character(db, id));
                let friend = owner.is_some_and(|owner| {
                    friends
                        .entries
                        .iter()
                        .any(|entry| entry.name.eq_ignore_ascii_case(&owner.name))
                });
                if friend {
                    HousingPlotOwnerType::Friend
                } else {
                    HousingPlotOwnerType::Stranger
                }
            }
        };
    }
}

fn send_snapshot(
    world: &mut World,
    actor: Actor,
    request_id: u64,
    neighborhood: Option<u32>,
    house: Option<u64>,
    error: Option<String>,
) {
    let reply = match read_snapshot(world, actor, neighborhood, house) {
        Ok(state) => HousingReply {
            request_id,
            state,
            error,
        },
        Err(message) => HousingReply {
            request_id,
            state: HousingState::default(),
            error: Some(message),
        },
    };
    if let Some(mut sender) = world.get_mut::<MessageSender<HousingReply>>(actor.connection) {
        sender.send::<HousingChannel>(reply);
    }
}

#[cfg(test)]
#[path = "housing_tests.rs"]
mod tests;
