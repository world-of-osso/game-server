//! Fresh FIFO housing writes; inventory and owner replies follow commit receipts.
use super::*;
use crate::persistence_writer::CommitReceipt;
use shared::protocol::{InventoryChannel, InventoryError, InventoryErrorReason};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub(super) enum ReplyOrigin {
    ItemUse,
    Request(u64),
}

enum InventoryPublication {
    Items,
    Gold,
}

struct PendingHousingWrite {
    actor: Actor,
    origin: ReplyOrigin,
    inventory: InventoryPublication,
    receipt: CommitReceipt,
}

#[derive(Resource, Default)]
pub(super) struct PendingHousingWrites(VecDeque<PendingHousingWrite>);

pub(crate) struct HousingPersistencePlugin;

impl HousingPersistencePlugin {
    pub(crate) fn install(app: &mut App) {
        app.init_resource::<PendingHousingWrites>()
            .add_systems(PostUpdate, publish_writes);
    }
}

#[cfg(test)]
pub(crate) fn persistence_pending(world: &World) -> bool {
    !world.resource::<PendingHousingWrites>().0.is_empty()
}

pub(super) fn enqueue_request(world: &mut World, actor: Actor, request: &HousingRequest) -> bool {
    let origin = ReplyOrigin::Request(request.request_id);
    let result = match &request.command {
        HousingCommand::RedeemDecor { item_guid } => {
            enqueue_collection(world, actor, *item_guid, origin)
        }
        HousingCommand::PurchasePlot { plot_id } => {
            enqueue_purchase(world, actor, *plot_id, request.request_id)
        }
        _ => return false,
    };
    if let Err(error) = result {
        send_snapshot(world, actor, request.request_id, None, None, Some(error));
    }
    true
}

pub(super) fn enqueue_collection(
    world: &mut World,
    actor: Actor,
    guid: u64,
    origin: ReplyOrigin,
) -> Result<(), String> {
    catalog(world)?;
    let context = actor_context(world, actor)?;
    let receipt = crate::housing_acquisition::redeem(world, context, actor.player, guid)?;
    world
        .resource_mut::<PendingHousingWrites>()
        .0
        .push_back(PendingHousingWrite {
            actor,
            origin,
            inventory: InventoryPublication::Items,
            receipt,
        });
    Ok(())
}

pub(super) fn enqueue_purchase(
    world: &mut World,
    actor: Actor,
    plot_id: u32,
    request_id: u64,
) -> Result<(), String> {
    let context = actor_context(world, actor)?;
    let plot = catalog(world)?.plot(plot_id)?.clone();
    if context.neighborhood_id != Some(plot.neighborhood_id) {
        return Err("Enter this neighborhood before purchasing its plot.".into());
    }
    require_cornerstone_reach(world, actor.player, plot_id)?;
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("housing plot purchase", move |db| {
            store::purchase(db, context, &plot).map(|_| ())
        });
    world
        .resource_mut::<PendingHousingWrites>()
        .0
        .push_back(PendingHousingWrite {
            actor,
            origin: ReplyOrigin::Request(request_id),
            inventory: InventoryPublication::Gold,
            receipt,
        });
    Ok(())
}

fn drain_completed(world: &mut World) -> Vec<(PendingHousingWrite, Result<(), String>)> {
    let mut pending = world.resource_mut::<PendingHousingWrites>();
    let mut completed = Vec::new();
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending
            .0
            .pop_front()
            .expect("completed housing write receipt");
        completed.push((job, result));
    }
    completed
}

fn owns_housing_session(world: &World, actor: Actor) -> bool {
    let authenticated = world
        .get::<Authenticated>(actor.connection)
        .is_some_and(|auth| auth.account_id == actor.account);
    let in_world = world
        .get::<InWorld>(actor.connection)
        .is_some_and(|session| session.character_id == actor.character);
    let linked = world
        .get::<PlayerLink>(actor.connection)
        .is_some_and(|link| link.0 == actor.player);
    authenticated && in_world && linked
}

pub(super) fn publish_writes(world: &mut World) {
    for (job, result) in drain_completed(world) {
        if owns_housing_session(world, job.actor) {
            publish_write_result(world, job, result);
        }
    }
}

fn publish_inventory(world: &mut World, actor: Actor, publication: InventoryPublication) {
    match publication {
        InventoryPublication::Items => world
            .resource_mut::<crate::gold::GoldUpdateQueue>()
            .record_items(actor.character),
        InventoryPublication::Gold => {
            let gold = crate::persistence::load_inventory(
                &world.resource::<GameDatabase>().db,
                actor.character,
            )
            .gold;
            world
                .resource_mut::<crate::gold::GoldUpdateQueue>()
                .record(actor.character, gold);
        }
    }
}

fn publish_write_result(world: &mut World, job: PendingHousingWrite, result: Result<(), String>) {
    if result.is_ok() {
        publish_inventory(world, job.actor, job.inventory);
    }
    match (job.origin, result) {
        (ReplyOrigin::Request(id), result) => {
            send_snapshot(world, job.actor, id, None, None, result.err())
        }
        (ReplyOrigin::ItemUse, Ok(())) => send_snapshot(world, job.actor, 0, None, None, None),
        (ReplyOrigin::ItemUse, Err(detail)) => send_item_use_error(world, job.actor, detail),
    }
}

fn send_item_use_error(world: &mut World, actor: Actor, detail: String) {
    if let Some(mut sender) = world.get_mut::<MessageSender<InventoryError>>(actor.connection) {
        sender.send::<InventoryChannel>(InventoryError {
            reason: InventoryErrorReason::CantDoRightNow,
            detail: Some(detail),
        });
    }
}
