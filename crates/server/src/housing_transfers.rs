//! Housing location writes finish on FIFO before their source-bound live transfer.
use super::*;
use crate::maps::WorldLocation;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

#[derive(Clone)]
struct Transfer {
    destination: WorldLocation,
    property: u64,
    neighborhood: Option<u32>,
}

struct PendingTransfer {
    actor: Actor,
    source_map: InMap,
    request_id: u64,
    receipt: CommitReceipt<Transfer>,
}

#[derive(Resource, Default)]
pub(super) struct PendingTransfers(VecDeque<PendingTransfer>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<PendingTransfers>()
        .add_systems(PostUpdate, publish_transfers);
}

#[cfg(test)]
pub(super) fn pending(world: &World) -> bool {
    !world.resource::<PendingTransfers>().0.is_empty()
}

pub(super) fn enqueue_request(world: &mut World, actor: Actor, request: &HousingRequest) -> bool {
    if !matches!(
        request.command,
        HousingCommand::EnterHouse { .. }
            | HousingCommand::LeaveHouse
            | HousingCommand::VisitPlot { .. }
            | HousingCommand::EnterNeighborhood { .. }
    ) {
        return false;
    }
    if let Err(error) = enqueue_transfer(world, actor, request) {
        send_snapshot(world, actor, request.request_id, None, None, Some(error));
    }
    true
}

fn enqueue_transfer(
    world: &mut World,
    actor: Actor,
    request: &HousingRequest,
) -> Result<(), String> {
    let source_map = world
        .get::<InMap>(actor.player)
        .copied()
        .ok_or("Finish the current map transfer first.")?;
    let source = catalog(world)?.clone();
    let context = actor_context(world, actor)?;
    let party = world
        .get_resource::<crate::group::GroupRegistry>()
        .map(|groups| groups.group_members(actor.character))
        .unwrap_or_default();
    let command = request.command.clone();
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("housing location", move |db| {
            persist_transfer(db, context, &command, &source, &party)
        });
    world
        .resource_mut::<PendingTransfers>()
        .0
        .push_back(PendingTransfer {
            actor,
            source_map,
            request_id: request.request_id,
            receipt,
        });
    Ok(())
}

fn persist_transfer(
    db: &redb::Database,
    actor: store::Actor,
    command: &HousingCommand,
    source: &HousingCatalog,
    party: &[u64],
) -> Result<Transfer, String> {
    if let HousingCommand::EnterHouse { house_id } = command {
        let destination = source.home_entry()?;
        store::save_house_location(db, actor, *house_id, party)?;
        return Ok(Transfer {
            destination,
            property: *house_id,
            neighborhood: None,
        });
    }
    let transfer = resolve_neighborhood_transfer(db, actor, command, source, party)?;
    store::save_neighborhood_location(
        db,
        actor.character_id,
        &store::NeighborhoodLocation {
            map_id: transfer.destination.map_id,
            neighborhood_id: transfer
                .neighborhood
                .expect("neighborhood transfer identity"),
        },
    )?;
    Ok(transfer)
}

fn resolve_neighborhood_transfer(
    db: &redb::Database,
    actor: store::Actor,
    command: &HousingCommand,
    source: &HousingCatalog,
    party: &[u64],
) -> Result<Transfer, String> {
    match command {
        HousingCommand::EnterNeighborhood { neighborhood_id } => {
            let neighborhood = source.neighborhood(*neighborhood_id)?;
            Ok(Transfer {
                destination: neighborhood.entry,
                property: u64::from(neighborhood.id),
                neighborhood: Some(neighborhood.id),
            })
        }
        HousingCommand::VisitPlot { plot_id } => plot_transfer(source, *plot_id),
        HousingCommand::LeaveHouse => {
            let house_id = actor
                .inside_house
                .ok_or("Enter a house before leaving it.")?;
            let house = store::read_house_with_party(db, actor, house_id, party)?;
            plot_transfer(source, house.plot_id)
        }
        _ => unreachable!("only location commands are queued as transfers"),
    }
}

fn plot_transfer(source: &HousingCatalog, plot_id: u32) -> Result<Transfer, String> {
    let plot = source.plot(plot_id)?;
    let neighborhood = source.neighborhood(plot.neighborhood_id)?;
    Ok(Transfer {
        destination: WorldLocation {
            map_id: neighborhood.entry.map_id,
            position: Vec3::from_array(plot.teleport),
            orientation: neighborhood.entry.orientation,
        },
        property: u64::from(neighborhood.id),
        neighborhood: Some(neighborhood.id),
    })
}

fn drain_completed(world: &mut World) -> Vec<(PendingTransfer, Result<Transfer, String>)> {
    let mut pending = world.resource_mut::<PendingTransfers>();
    let mut completed = Vec::new();
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        completed.push((
            pending.0.pop_front().expect("completed location receipt"),
            result,
        ));
    }
    completed
}

fn publish_transfers(world: &mut World) {
    for (job, result) in drain_completed(world) {
        if !persistence_actor::owns_housing_session(world, job.actor) {
            continue;
        }
        let result = result.and_then(|transfer| {
            if world.get::<InMap>(job.actor.player) != Some(&job.source_map) {
                return Err("The source map changed before housing admission completed.".into());
            }
            transfer_property(world, job.actor, transfer.destination, transfer.property)?;
            Ok(transfer.neighborhood)
        });
        let (neighborhood, error) = match result {
            Ok(neighborhood) => (neighborhood, None),
            Err(error) => (None, Some(error)),
        };
        send_snapshot(world, job.actor, job.request_id, neighborhood, None, error);
    }
}
