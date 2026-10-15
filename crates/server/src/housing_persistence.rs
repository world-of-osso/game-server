//! Receipt-gated collection from normal bag use and authenticated housing requests.
use super::*;
use crate::persistence_writer::CommitReceipt;
use shared::protocol::{InventoryChannel, InventoryError, InventoryErrorReason};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub(super) enum CollectionOrigin {
    ItemUse,
    Request(u64),
}

struct PendingCollection {
    actor: Actor,
    origin: CollectionOrigin,
    receipt: CommitReceipt,
}

#[derive(Resource, Default)]
struct PendingCollections(VecDeque<PendingCollection>);

pub(crate) struct HousingPersistencePlugin;

impl HousingPersistencePlugin {
    pub(crate) fn install(app: &mut App) {
        app.init_resource::<PendingCollections>()
            .add_systems(PostUpdate, publish_collections);
    }
}

#[cfg(test)]
pub(crate) fn persistence_pending(world: &World) -> bool {
    !world.resource::<PendingCollections>().0.is_empty()
}

pub(super) fn enqueue_request(world: &mut World, actor: Actor, request: &HousingRequest) -> bool {
    let HousingCommand::RedeemDecor { item_guid } = &request.command else {
        return false;
    };
    let origin = CollectionOrigin::Request(request.request_id);
    if let Err(error) = enqueue_collection(world, actor, *item_guid, origin) {
        send_snapshot(world, actor, request.request_id, None, None, Some(error));
    }
    true
}

pub(super) fn enqueue_collection(
    world: &mut World,
    actor: Actor,
    guid: u64,
    origin: CollectionOrigin,
) -> Result<(), String> {
    catalog(world)?;
    let context = actor_context(world, actor)?;
    let receipt = crate::housing_acquisition::redeem(world, context, actor.player, guid)?;
    world
        .resource_mut::<PendingCollections>()
        .0
        .push_back(PendingCollection {
            actor,
            origin,
            receipt,
        });
    Ok(())
}

fn drain_completed(world: &mut World) -> Vec<(Actor, CollectionOrigin, Result<(), String>)> {
    let mut pending = world.resource_mut::<PendingCollections>();
    let mut completed = Vec::new();
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending
            .0
            .pop_front()
            .expect("completed housing collection receipt");
        completed.push((job.actor, job.origin, result));
    }
    completed
}

fn owns_collection_session(world: &World, actor: Actor) -> bool {
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

fn publish_collections(world: &mut World) {
    for (actor, origin, result) in drain_completed(world) {
        if owns_collection_session(world, actor) {
            publish_collection_result(world, actor, origin, result);
        }
    }
}

fn publish_collection_result(
    world: &mut World,
    actor: Actor,
    origin: CollectionOrigin,
    result: Result<(), String>,
) {
    if result.is_ok() {
        world
            .resource_mut::<crate::gold::GoldUpdateQueue>()
            .record_items(actor.character);
    }
    match (origin, result) {
        (CollectionOrigin::Request(id), result) => {
            send_snapshot(world, actor, id, None, None, result.err())
        }
        (CollectionOrigin::ItemUse, Ok(())) => send_snapshot(world, actor, 0, None, None, None),
        (CollectionOrigin::ItemUse, Err(detail)) => {
            if let Some(mut sender) =
                world.get_mut::<MessageSender<InventoryError>>(actor.connection)
            {
                sender.send::<InventoryChannel>(InventoryError {
                    reason: InventoryErrorReason::CantDoRightNow,
                    detail: Some(detail),
                });
            }
        }
    }
}
