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
            crate::housing_acquisition::read_bookcase_rule(&db),
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
    commands.insert_resource(crate::housing_acquisition::HousingCollectionRule(
        collection,
    ));
}

pub(crate) fn spawn_cornerstones(
    commands: &mut Commands,
    source: &HousingAvailability,
    map: InMap,
) {
    if let Ok(catalog) = &source.0 {
        let objects = crate::game_objects::GameObjectSpawns(catalog.cornerstones());
        crate::game_objects::spawn_map_game_objects(commands, &objects, map, 0);
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
        let result = execute(world, actor, &request.command);
        if result.is_ok()
            && matches!(
                request.command,
                HousingCommand::PlaceDecor { .. }
                    | HousingCommand::MoveDecor { .. }
                    | HousingCommand::RotateDecor { .. }
                    | HousingCommand::RemoveDecor { .. }
            )
        {
            publish_layout_to_observers(world, actor);
        }
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

fn publish_layout_to_observers(world: &mut World, editor: Actor) {
    let map = *world
        .get::<InMap>(editor.player)
        .expect("accepted house edit requires an admitted map");
    let mut query = world.query::<(Entity, &InWorld, &Authenticated, &PlayerLink)>();
    let observers: Vec<_> = query
        .iter(world)
        .filter(|(connection, _, _, link)| {
            *connection != editor.connection && world.get::<InMap>(link.0) == Some(&map)
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
        HousingCommand::EnterNeighborhood { neighborhood_id } => {
            enter_neighborhood(world, actor, *neighborhood_id)?;
            return Ok(Some(*neighborhood_id));
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
        HousingCommand::PurchasePlot { plot_id } => purchase_plot(world, actor, context, *plot_id)?,
        HousingCommand::EnterHouse { house_id } => enter_house(world, actor, context, *house_id)?,
        HousingCommand::LeaveHouse => leave_house(world, actor, context)?,
        HousingCommand::SetHouseAccessAnyone { house_id, allowed } => {
            store::set_access(
                &world.resource::<GameDatabase>().db,
                context,
                *house_id,
                *allowed,
            )?;
        }
        HousingCommand::RedeemDecor { item_guid } => collect_decor(world, actor, *item_guid)?,
        HousingCommand::PlaceDecor { .. }
        | HousingCommand::MoveDecor { .. }
        | HousingCommand::RotateDecor { .. }
        | HousingCommand::RemoveDecor { .. } => {
            store::edit(&world.resource::<GameDatabase>().db, context, command)?;
        }
        HousingCommand::ReloadLayout { house_id } => {
            store::house(&world.resource::<GameDatabase>().db, context, *house_id)?;
        }
        HousingCommand::ListPlots { .. } | HousingCommand::EnterNeighborhood { .. } => {
            unreachable!("neighborhood commands handled before house operations")
        }
    }
    Ok(())
}

fn collect_decor(world: &mut World, actor: Actor, guid: u64) -> Result<(), String> {
    catalog(world)?;
    crate::housing_acquisition::require_item_eligibility(world, actor.player)?;
    let context = actor_context(world, actor)?;
    crate::housing_acquisition::redeem(world, context, guid)
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
    collect_decor(world, actor, guid)?;
    send_snapshot(world, actor, 0, None, None, None);
    Ok(())
}

fn enter_house(
    world: &mut World,
    actor: Actor,
    context: store::Actor,
    house_id: u64,
) -> Result<(), String> {
    store::house(&world.resource::<GameDatabase>().db, context, house_id)?;
    let entry = catalog(world)?.home_entry()?;
    transfer_property(world, actor, entry, house_id)?;
    store::save_house_location(&world.resource::<GameDatabase>().db, context, house_id)
}

fn leave_house(world: &mut World, actor: Actor, context: store::Actor) -> Result<(), String> {
    let house_id = context
        .inside_house
        .ok_or("Enter a house before leaving it.")?;
    let house = store::house(&world.resource::<GameDatabase>().db, context, house_id)?;
    let plot = catalog(world)?.plot(house.plot_id)?.clone();
    let neighborhood = catalog(world)?.neighborhood(house.neighborhood_id)?.clone();
    let destination = crate::maps::WorldLocation {
        map_id: neighborhood.entry.map_id,
        position: Vec3::from_array(plot.teleport),
        orientation: neighborhood.entry.orientation,
    };
    transfer_property(world, actor, destination, u64::from(house.neighborhood_id))?;
    store::save_neighborhood_location(
        &world.resource::<GameDatabase>().db,
        actor.character,
        &store::NeighborhoodLocation {
            map_id: destination.map_id,
            neighborhood_id: house.neighborhood_id,
        },
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

fn enter_neighborhood(world: &mut World, actor: Actor, neighborhood_id: u32) -> Result<(), String> {
    if world.get::<InMap>(actor.player).is_none() {
        return Err("Finish the current map transfer first.".into());
    }
    let neighborhood = catalog(world)?.neighborhood(neighborhood_id)?.clone();
    world
        .resource_scope(|world, mut manager: Mut<InstanceManager>| {
            manager.authorize_housing_entry(
                world.resource::<MapStore>(),
                neighborhood.entry.map_id,
                u64::from(neighborhood.id),
                actor.character,
            )
        })
        .map_err(|reason| format!("Neighborhood admission refused: {reason:?}"))?;
    let accepted = crate::maps::teleport_to(
        world,
        actor.player,
        neighborhood.entry,
        TeleportOptions::default(),
    );
    if !accepted {
        world
            .resource_mut::<InstanceManager>()
            .revoke_housing_entry(actor.character);
        return Err("Neighborhood transfer was refused.".into());
    }
    store::save_neighborhood_location(
        &world.resource::<GameDatabase>().db,
        actor.character,
        &store::NeighborhoodLocation {
            map_id: neighborhood.entry.map_id,
            neighborhood_id: neighborhood.id,
        },
    )?;
    Ok(())
}

fn purchase_plot(
    world: &mut World,
    actor: Actor,
    context: store::Actor,
    plot_id: u32,
) -> Result<(), String> {
    let plot = catalog(world)?.plot(plot_id)?.clone();
    if context.neighborhood_id != Some(plot.neighborhood_id) {
        return Err("Enter this neighborhood before purchasing its plot.".into());
    }
    require_cornerstone_reach(world, actor.player, plot_id)?;
    let db = &world.resource::<GameDatabase>().db;
    store::purchase(db, context, &plot)?;
    let gold = crate::persistence::load_inventory(db, actor.character).gold;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record(actor.character, gold);
    Ok(())
}

pub(crate) fn open_cornerstone(
    world: &mut World,
    connection: Entity,
    player: Entity,
    object: Entity,
) -> Result<(), shared::protocol::InteractionError> {
    use shared::protocol::InteractionError;
    let actor = authenticate_cornerstone_actor(world, connection, player, object)?;
    let info = world
        .get::<shared::protocol::GameObjectInfo>(object)
        .ok_or(InteractionError::InvalidTarget)?;
    let plot_id = catalog(world)
        .map_err(|_| InteractionError::InvalidTarget)?
        .cornerstone_plot(info.entry)
        .ok_or(InteractionError::InvalidTarget)?;
    require_cornerstone_reach(world, player, plot_id).map_err(|_| InteractionError::TooFarAway)?;
    let mut state =
        read_snapshot(world, actor, None, None).map_err(|_| InteractionError::InvalidTarget)?;
    state.opened_cornerstone = Some(plot_id);
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

fn authenticate_cornerstone_actor(
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
    let mut state = store::snapshot(db, context, house.or(context.inside_house))?;
    if let Some(neighborhood) = neighborhood.or(context.neighborhood_id) {
        state.plots = catalog(world)?.plots(neighborhood)?;
        store::populate_plots(db, &mut state.plots)?;
    }
    Ok(state)
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
