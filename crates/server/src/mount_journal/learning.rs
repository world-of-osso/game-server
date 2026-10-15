//! Mount teaching items consume fresh inventory and grant ownership at one FIFO position.
use super::*;
use crate::persistence_writer::CommitReceipt;
use shared::protocol::{InventoryChannel, InventoryError, InventoryErrorReason};
use std::collections::VecDeque;

#[derive(Clone)]
pub(super) struct LearningRequest {
    pub connection: Entity,
    pub player: Entity,
    pub account: u64,
    pub character: u64,
    pub location: ItemLocation,
    pub item: u32,
    pub guid: u64,
    pub mount: u32,
}

struct PendingLearning {
    request: LearningRequest,
    receipt: CommitReceipt<Result<(), InventoryError>>,
}

#[derive(Resource, Default)]
struct PendingMountLearning(VecDeque<PendingLearning>);

pub(crate) struct MountPersistencePlugin;

impl MountPersistencePlugin {
    pub(crate) fn install(app: &mut App) {
        app.init_resource::<PendingMountLearning>()
            .add_systems(PostUpdate, publish_mount_learning);
    }
}

#[cfg(test)]
pub(crate) fn persistence_pending(world: &World) -> bool {
    !world.resource::<PendingMountLearning>().0.is_empty()
}

pub(super) fn enqueue_mount_learning(world: &mut World, request: LearningRequest) {
    let templates = world
        .resource::<crate::item_templates::ItemTemplates>()
        .clone();
    let owned = request.clone();
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("mount item learning", move |db| {
            Ok(commit_mount_learning(db, &templates, &owned))
        });
    world
        .resource_mut::<PendingMountLearning>()
        .0
        .push_back(PendingLearning { request, receipt });
}

fn commit_mount_learning(
    db: &redb::Database,
    templates: &crate::item_templates::ItemTemplates,
    request: &LearningRequest,
) -> Result<(), InventoryError> {
    let mut model = crate::inventory::load_model(db, templates, request.character)?;
    let current = model.existing(request.location)?;
    if current.item_guid != request.guid || current.item_id != request.item {
        return Err(InventoryError {
            reason: InventoryErrorReason::ItemNotFound,
            detail: Some("Mount item changed before learning.".into()),
        });
    }
    model.destroy(request.location, 1)?;
    mount_store::learn_item(
        db,
        request.account,
        request.character,
        request.mount,
        &model.into_record(),
    )
    .map_err(|detail| InventoryError {
        reason: InventoryErrorReason::CantDoRightNow,
        detail: Some(detail),
    })
}

fn publish_mount_learning(world: &mut World) {
    let completed = drain_completed_learning(world);
    for (request, completion) in completed {
        if owns_learning_session(world, &request) {
            publish_learning_result(world, request, completion);
        }
    }
}

fn drain_completed_learning(
    world: &mut World,
) -> Vec<(LearningRequest, Result<Result<(), InventoryError>, String>)> {
    let mut pending = world.resource_mut::<PendingMountLearning>();
    let mut completed = Vec::new();
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending
            .0
            .pop_front()
            .expect("completed mount learning receipt");
        completed.push((job.request, result));
    }
    completed
}

fn owns_learning_session(world: &World, request: &LearningRequest) -> bool {
    let authenticated = world
        .get::<Authenticated>(request.connection)
        .is_some_and(|auth| auth.account_id == request.account);
    let in_world = world
        .get::<InWorld>(request.connection)
        .is_some_and(|session| session.character_id == request.character);
    let linked = world
        .get::<PlayerLink>(request.connection)
        .is_some_and(|link| link.0 == request.player);
    authenticated && in_world && linked
}

fn publish_learning_result(
    world: &mut World,
    request: LearningRequest,
    completion: Result<Result<(), InventoryError>, String>,
) {
    let result = completion.unwrap_or_else(|detail| {
        Err(InventoryError {
            reason: InventoryErrorReason::InternalBagError,
            detail: Some(detail),
        })
    });
    match result {
        Ok(()) => {
            world
                .resource_mut::<crate::gold::GoldUpdateQueue>()
                .record_items(request.character);
            broadcast_account(world, request.account);
        }
        Err(error) => {
            if let Some(mut sender) =
                world.get_mut::<MessageSender<InventoryError>>(request.connection)
            {
                sender.send::<InventoryChannel>(error);
            }
        }
    }
}
