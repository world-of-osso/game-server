//! Fresh FIFO inventory consumption and toy ownership commit atomically.
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
}

#[derive(Clone)]
struct LearningCommit {
    outcome: Result<(), String>,
}

struct PendingLearning {
    request: LearningRequest,
    receipt: CommitReceipt<Result<LearningCommit, InventoryError>>,
}

#[derive(Resource, Default)]
pub(super) struct PendingToyLearning(VecDeque<PendingLearning>);

impl PendingToyLearning {
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub(super) fn enqueue_toy_learning(world: &mut World, request: LearningRequest) {
    let templates = world
        .resource::<crate::item_templates::ItemTemplates>()
        .clone();
    let owned = request.clone();
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("toy learning", move |db| {
            Ok(commit_toy_learning(db, &templates, &owned))
        });
    world
        .resource_mut::<PendingToyLearning>()
        .0
        .push_back(PendingLearning { request, receipt });
}

fn commit_toy_learning(
    db: &redb::Database,
    templates: &crate::item_templates::ItemTemplates,
    request: &LearningRequest,
) -> Result<LearningCommit, InventoryError> {
    let mut model = crate::inventory::load_model(db, templates, request.character)?;
    let current = model.existing(request.location)?;
    if current.item_guid != request.guid || current.item_id != request.item {
        return Err(InventoryError {
            reason: InventoryErrorReason::ItemNotFound,
            detail: Some("toy item changed before learning".into()),
        });
    }
    model.destroy(request.location, 1)?;
    let outcome = toy_store::learn(
        db,
        request.account,
        request.character,
        request.item,
        &model.into_record(),
    );
    Ok(LearningCommit { outcome })
}

pub(super) fn publish_toy_learning(world: &mut World) {
    let completed = {
        let mut pending = world.resource_mut::<PendingToyLearning>();
        let mut completed = Vec::new();
        while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
            let job = pending
                .0
                .pop_front()
                .expect("completed toy learning receipt");
            completed.push((job.request, result));
        }
        completed
    };
    for (request, result) in completed {
        if owns_learning_session(world, &request) {
            publish_learning_result(world, request, result);
        }
    }
}

fn owns_learning_session(world: &World, request: &LearningRequest) -> bool {
    world
        .get::<Authenticated>(request.connection)
        .is_some_and(|auth| auth.account_id == request.account)
        && world
            .get::<InWorld>(request.connection)
            .is_some_and(|session| session.character_id == request.character)
        && world
            .get::<PlayerLink>(request.connection)
            .is_some_and(|link| link.0 == request.player)
}

fn publish_learning_result(
    world: &mut World,
    request: LearningRequest,
    completion: Result<Result<LearningCommit, InventoryError>, String>,
) {
    let outcome = send_learning_commit_result(world, &request, completion);
    match outcome {
        Ok(()) => {
            world
                .resource_mut::<crate::gold::GoldUpdateQueue>()
                .record_items(request.character);
            crate::achievement::record_toy_ownership(world, request.player, request.account);
            broadcast(world, request.account);
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

fn send_learning_commit_result(
    world: &mut World,
    request: &LearningRequest,
    completion: Result<Result<LearningCommit, InventoryError>, String>,
) -> Result<(), InventoryError> {
    match completion {
        Ok(Ok(committed)) => {
            result(
                world,
                request.connection,
                request.item,
                ToyOperation::Learn,
                None,
                committed.outcome.clone(),
            );
            committed.outcome.map_err(|error| InventoryError {
                reason: InventoryErrorReason::CantDoRightNow,
                detail: Some(error),
            })
        }
        Ok(Err(error)) => Err(error),
        Err(error) => Err(InventoryError {
            reason: InventoryErrorReason::InternalBagError,
            detail: Some(error),
        }),
    }
}
