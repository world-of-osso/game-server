//! Toy favourite mutations share the serial writer; replies wait for durability.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

struct PendingFavourite {
    connection: Entity,
    account: u64,
    item: u32,
    receipt: CommitReceipt,
}

#[derive(Resource, Default)]
pub(super) struct PendingToyFavourites(VecDeque<PendingFavourite>);

impl PendingToyFavourites {
    pub(super) fn enqueue(
        &mut self,
        db: &GameDatabase,
        connection: Entity,
        account: u64,
        request: SetToyFavourite,
    ) {
        let item = request.item_id;
        let receipt = db.writer.enqueue("toy favourite", move |db| {
            toy_store::favourite(db, account, item, request.favourite)
        });
        self.0.push_back(PendingFavourite {
            connection,
            account,
            item,
            receipt,
        });
    }

    fn drain_completed(&mut self) -> Vec<(PendingFavourite, Result<(), String>)> {
        let mut completed = Vec::new();
        while let Some(result) = self.0.front().and_then(|job| job.receipt.result()) {
            let job = self.0.pop_front().expect("completed toy favourite receipt");
            completed.push((job, result));
        }
        completed
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub(super) struct ToyPersistencePlugin;

impl ToyPersistencePlugin {
    pub(super) fn install(app: &mut App) {
        app.init_resource::<PendingToyFavourites>()
            .init_resource::<PendingToyLearning>()
            .add_systems(
                PostUpdate,
                (learning::publish_toy_learning, publish_toy_favourites).chain(),
            );
    }
}

fn publish_toy_favourites(world: &mut World) {
    let completed = world
        .resource_mut::<PendingToyFavourites>()
        .drain_completed();
    for (job, outcome) in completed {
        let authenticated = world
            .get::<Authenticated>(job.connection)
            .is_some_and(|auth| auth.account_id == job.account);
        if !authenticated || world.get::<InWorld>(job.connection).is_none() {
            continue;
        }
        if outcome.is_ok() {
            broadcast(world, job.account);
        }
        result(
            world,
            job.connection,
            job.item,
            ToyOperation::Favourite,
            None,
            outcome,
        );
    }
}
