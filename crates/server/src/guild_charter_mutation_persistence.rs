//! FIFO signature/rename mutations; petition publication follows their commit receipts.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

pub(super) enum Change {
    Sign(u64),
    Rename { guid: u64, name: String },
}
#[derive(Clone)]
enum Publication {
    Signed { guid: u64, owner: u64 },
    Renamed { guid: u64 },
}
struct PendingMutation {
    actor: Actor,
    receipt: CommitReceipt<Result<Publication, String>>,
}
#[derive(Resource, Default)]
struct PendingCharterMutations(VecDeque<PendingMutation>);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<PendingCharterMutations>()
        .add_systems(PostUpdate, publish_mutations);
}
pub(super) fn enqueue(world: &mut World, actor: Actor, change: Change) {
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("guild charter mutation", move |db| {
            Ok(persist_mutation(db, actor, change))
        });
    world
        .resource_mut::<PendingCharterMutations>()
        .0
        .push_back(PendingMutation { actor, receipt });
}
fn persist_mutation(
    db: &redb::Database,
    actor: Actor,
    change: Change,
) -> Result<Publication, String> {
    match change {
        Change::Sign(guid) => {
            let owner = store::owner_of(db, guid)?;
            store::sign(db, guid, actor.character, actor.account)?;
            Ok(Publication::Signed { guid, owner })
        }
        Change::Rename { guid, name } => {
            require_persisted_owner(db, actor.character, guid)?;
            store::rename(db, guid, actor.character, &name)?;
            Ok(Publication::Renamed { guid })
        }
    }
}
fn owns_session(world: &World, actor: Actor) -> bool {
    let authenticated = world
        .get::<Authenticated>(actor.connection)
        .is_some_and(|auth| auth.account_id == actor.account);
    let entered = world
        .get::<InWorld>(actor.connection)
        .is_some_and(|entered| entered.character_id == actor.character);
    let linked = world
        .get::<PlayerLink>(actor.connection)
        .is_some_and(|link| link.0 == actor.player);
    authenticated && entered && linked
}
fn take_completed_mutations(
    world: &mut World,
) -> Vec<(Actor, Result<Result<Publication, String>, String>)> {
    let mut pending = world.resource_mut::<PendingCharterMutations>();
    let mut completed = Vec::new();
    while let Some(result) = pending.0.front().and_then(|job| job.receipt.result()) {
        let job = pending
            .0
            .pop_front()
            .expect("completed charter mutation receipt");
        completed.push((job.actor, result));
    }
    completed
}
fn publish_mutations(world: &mut World) {
    for (actor, result) in take_completed_mutations(world) {
        match result.and_then(|result| result) {
            Ok(Publication::Signed { guid, owner }) => {
                publish_actor_petition(world, actor, guid);
                if let Some(owner) = find_online_actor(world, owner) {
                    publish_actor_petition(world, owner, guid);
                }
            }
            Ok(Publication::Renamed { guid }) => publish_actor_petition(world, actor, guid),
            Err(message) => publish_error(world, actor, message),
        }
    }
}
fn publish_actor_petition(world: &mut World, actor: Actor, guid: u64) {
    if !owns_session(world, actor) {
        return;
    }
    if let Err(message) = send_petition(world, actor, guid) {
        publish_error(world, actor, message);
    }
}
fn publish_error(world: &mut World, actor: Actor, message: String) {
    if owns_session(world, actor) {
        send_reply(
            world,
            actor.connection,
            GuildCharterReply::Failed { message },
        );
    }
}
