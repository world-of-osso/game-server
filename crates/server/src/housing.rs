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
    let loaded = rusqlite::Connection::open_with_flags(
        crate::spell_info::WORLD_DB_PATH,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| format!("Open housing catalog: {error}"))
    .and_then(|db| HousingCatalog::read(&db));
    if let Err(message) = &loaded {
        warn!("Housing unavailable: {message}");
    }
    commands.insert_resource(HousingAvailability(loaded));
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
        HousingCommand::EnterHouse { house_id } => refuse_unavailable_house_entry(world, context, *house_id)?,
        HousingCommand::LeaveHouse => return Err("House entry is not yet supported.".into()),
        HousingCommand::SetHouseAccessAnyone { house_id, allowed } => {
            store::set_access(&world.resource::<GameDatabase>().db, context, *house_id, *allowed)?;
        }
        HousingCommand::RedeemDecor { .. } => return Err("Use the authenticated vendor item collection path; redemption integration is not yet supported.".into()),
        HousingCommand::PlaceDecor { .. } | HousingCommand::MoveDecor { .. } | HousingCommand::RotateDecor { .. } | HousingCommand::RemoveDecor { .. } => {
            store::edit(&world.resource::<GameDatabase>().db, context, command)?;
        }
        HousingCommand::ReloadLayout { house_id } => {
            store::house(&world.resource::<GameDatabase>().db, context, *house_id)?;
        }
        HousingCommand::ListPlots { .. } | HousingCommand::EnterNeighborhood { .. } => unreachable!("neighborhood commands handled before house operations"),
    }
    Ok(())
}

fn refuse_unavailable_house_entry(
    world: &World,
    context: store::Actor,
    house_id: u64,
) -> Result<(), String> {
    let house = store::house(&world.resource::<GameDatabase>().db, context, house_id)?;
    if !house.interior_grant_available {
        return Err("No verified first-house interior grant source is available; entry is not yet supported.".into());
    }
    Err("House component placement is not yet supported.".into())
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
    let position = world
        .get::<Position>(actor.player)
        .ok_or("Player position unavailable.")?;
    let (x, y, z) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
    let distance = Vec3::new(x, y, z).distance(Vec3::from_array(plot.cornerstone));
    if !distance.is_finite() || distance > crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE {
        return Err("Move within interaction range of this plot's authored cornerstone.".into());
    }
    let db = &world.resource::<GameDatabase>().db;
    store::purchase(db, context, &plot)?;
    let gold = crate::persistence::load_inventory(db, actor.character).gold;
    world
        .resource_mut::<crate::gold::GoldUpdateQueue>()
        .record(actor.character, gold);
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
