//! Fresh account grants commit before connection snapshots are published.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

pub(super) struct RefreshRequest {
    pub connection: Entity,
    pub account: u64,
    pub character: u64,
    pub force_send: bool,
    pub appearances: Vec<u32>,
}

struct PendingRefresh {
    request: RefreshRequest,
    receipt: CommitReceipt<AppearanceCollection>,
}

#[derive(Resource, Default)]
pub(super) struct PendingAppearanceRefreshes(VecDeque<PendingRefresh>);

impl PendingAppearanceRefreshes {
    pub(super) fn enqueue(&mut self, db: &GameDatabase, request: RefreshRequest) {
        let account = request.account;
        let appearances = request.appearances.clone();
        let receipt = db.writer.enqueue("appearance refresh", move |db| {
            let mut collection = load_account_appearances(db, account);
            if collection.learn_many(appearances) > 0 {
                save_account_appearances(db, account, &collection);
                collection = load_account_appearances(db, account);
            }
            Ok(collection)
        });
        self.0.push_back(PendingRefresh { request, receipt });
    }

    fn drain_completed(&mut self) -> Vec<(RefreshRequest, Result<AppearanceCollection, String>)> {
        let mut completed = Vec::new();
        while let Some(result) = self.0.front().and_then(|job| job.receipt.result()) {
            let job = self.0.pop_front().expect("completed appearance receipt");
            completed.push((job.request, result));
        }
        completed
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub(super) fn publish_appearance_refreshes(world: &mut World) {
    let completed = world
        .resource_mut::<PendingAppearanceRefreshes>()
        .drain_completed();
    for (request, result) in completed {
        let collection = match result {
            Ok(collection) => collection,
            Err(error) => {
                error!(account = request.account, %error, "Appearance refresh commit failed");
                continue;
            }
        };
        if has_appearance_session(world, &request) {
            publish_appearance_snapshot(world, request, collection);
        }
    }
}

fn has_appearance_session(world: &World, request: &RefreshRequest) -> bool {
    let authenticated = world
        .get::<Authenticated>(request.connection)
        .is_some_and(|auth| auth.account_id == request.account);
    let in_world = world
        .get::<InWorld>(request.connection)
        .is_some_and(|session| session.character_id == request.character);
    authenticated && in_world
}

fn publish_appearance_snapshot(
    world: &mut World,
    request: RefreshRequest,
    collection: AppearanceCollection,
) {
    let changed = world
        .get::<AccountAppearances>(request.connection)
        .and_then(|loaded| loaded.0.as_ref())
        .is_none_or(|loaded| *loaded != collection);
    world
        .entity_mut(request.connection)
        .insert(AccountAppearances(Some(collection.clone())));
    if (changed || request.force_send)
        && let Some(mut sender) =
            world.get_mut::<MessageSender<AppearanceCollectionUpdate>>(request.connection)
    {
        sender.send::<TooltipChannel>(AppearanceCollectionUpdate {
            appearances: collection.all().collect(),
        });
    }
}
