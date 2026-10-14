//! Mount favourites persist fresh account ownership before live journal replies.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

struct FavouriteSession {
    connection: Entity,
    player: Entity,
    account: u64,
    character: u64,
}

struct PendingFavourite {
    session: FavouriteSession,
    receipt: CommitReceipt,
}

#[derive(Resource, Default)]
pub(super) struct PendingMountFavourites(VecDeque<PendingFavourite>);

pub(super) fn enqueue_mount_favourite(
    world: &mut World,
    connection: Entity,
    account: u64,
    player: Entity,
    request: SetMountFavourite,
) {
    let character = world
        .get::<InWorld>(connection)
        .expect("in-world mount requester")
        .character_id;
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("mount favourite", move |db| {
            mount_store::favourite(db, account, request.mount_id, request.favourite)
        });
    world
        .resource_mut::<PendingMountFavourites>()
        .0
        .push_back(PendingFavourite {
            session: FavouriteSession {
                connection,
                player,
                account,
                character,
            },
            receipt,
        });
}

impl PendingMountFavourites {
    fn drain_completed(&mut self) -> Vec<(FavouriteSession, Result<(), String>)> {
        let mut completed = Vec::new();
        while let Some(result) = self.0.front().and_then(|job| job.receipt.result()) {
            let job = self
                .0
                .pop_front()
                .expect("completed mount favourite receipt");
            completed.push((job.session, result));
        }
        completed
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub(super) fn publish_mount_favourites(world: &mut World) {
    let completed = world
        .resource_mut::<PendingMountFavourites>()
        .drain_completed();
    for (session, result) in completed {
        if !has_favourite_session(world, &session) {
            continue;
        }
        match result {
            Ok(()) => broadcast_account(world, session.account),
            Err(error) => send_snapshot(world, session.connection, session.player, Some(error)),
        }
    }
}

fn has_favourite_session(world: &World, session: &FavouriteSession) -> bool {
    let authenticated = world
        .get::<Authenticated>(session.connection)
        .is_some_and(|auth| auth.account_id == session.account);
    let in_world = world
        .get::<InWorld>(session.connection)
        .is_some_and(|current| current.character_id == session.character);
    let linked = world
        .get::<PlayerLink>(session.connection)
        .is_some_and(|link| link.0 == session.player);
    authenticated && in_world && linked
}
