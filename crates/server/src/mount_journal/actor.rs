//! Spell-derived mount grants, favourites and dependent reads share durable FIFO order.
use super::*;
use crate::persistence_writer::CommitReceipt;
use std::collections::VecDeque;

struct MountSession {
    connection: Entity,
    player: Entity,
    account: u64,
    character: u64,
}

struct PendingFavourite {
    session: MountSession,
    receipt: CommitReceipt,
}

struct PendingRandom {
    session: MountSession,
    receipt: CommitReceipt<mount_store::AccountMounts>,
}

struct PendingSpellLearning {
    account: u64,
    mounts: Vec<u32>,
    receipt: CommitReceipt,
}

#[derive(Resource, Default)]
pub(super) struct PendingMountRequests {
    favourites: VecDeque<PendingFavourite>,
    randoms: VecDeque<PendingRandom>,
    learning: VecDeque<PendingSpellLearning>,
    // Failed source grants remain submitted until restart instead of retrying every frame.
    submitted_mounts: BTreeMap<u64, BTreeSet<u32>>,
}

pub(super) fn enqueue_mount_favourite(
    world: &mut World,
    connection: Entity,
    account: u64,
    player: Entity,
    request: SetMountFavourite,
) {
    let session = capture_mount_session(world, connection, account, player);
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("mount favourite", move |db| {
            mount_store::favourite(db, account, request.mount_id, request.favourite)
        });
    world
        .resource_mut::<PendingMountRequests>()
        .favourites
        .push_back(PendingFavourite { session, receipt });
}

pub(super) fn enqueue_random_mount(
    world: &mut World,
    connection: Entity,
    account: u64,
    player: Entity,
) {
    let session = capture_mount_session(world, connection, account, player);
    let receipt = world
        .resource::<GameDatabase>()
        .writer
        .enqueue("random mount ownership", move |db| {
            Ok(mount_store::load(db, account))
        });
    world
        .resource_mut::<PendingMountRequests>()
        .randoms
        .push_back(PendingRandom { session, receipt });
}

pub(super) fn enqueue_mount_spell_learning(world: &mut World, account: u64, mounts: Vec<u32>) {
    let owned = mounts.clone();
    let receipt =
        world
            .resource::<GameDatabase>()
            .writer
            .enqueue("mount spell learning", move |db| {
                mount_store::learn_spells(db, account, &owned);
                Ok(())
            });
    let mut pending = world.resource_mut::<PendingMountRequests>();
    pending
        .submitted_mounts
        .entry(account)
        .or_default()
        .extend(mounts.iter().copied());
    pending.learning.push_back(PendingSpellLearning {
        account,
        mounts,
        receipt,
    });
}

fn capture_mount_session(
    world: &World,
    connection: Entity,
    account: u64,
    player: Entity,
) -> MountSession {
    let character = world
        .get::<InWorld>(connection)
        .expect("in-world mount requester")
        .character_id;
    MountSession {
        connection,
        player,
        account,
        character,
    }
}

impl PendingMountRequests {
    pub(super) fn mount_submitted(&self, account: u64, mount: u32) -> bool {
        self.submitted_mounts
            .get(&account)
            .is_some_and(|mounts| mounts.contains(&mount))
    }

    fn drain_learning(&mut self) -> Vec<(PendingSpellLearning, Result<(), String>)> {
        let mut completed = Vec::new();
        while let Some(result) = self.learning.front().and_then(|job| job.receipt.result()) {
            let job = self
                .learning
                .pop_front()
                .expect("completed mount learning receipt");
            completed.push((job, result));
        }
        completed
    }

    fn retire_learned_mounts(&mut self, account: u64, mounts: &[u32]) {
        let submitted = self
            .submitted_mounts
            .get_mut(&account)
            .expect("submitted mount ids");
        for mount in mounts {
            submitted.remove(mount);
        }
        if submitted.is_empty() {
            self.submitted_mounts.remove(&account);
        }
    }

    fn drain_favourites(&mut self) -> Vec<(MountSession, Result<(), String>)> {
        let mut completed = Vec::new();
        while let Some(result) = self.favourites.front().and_then(|job| job.receipt.result()) {
            let job = self
                .favourites
                .pop_front()
                .expect("completed mount favourite receipt");
            completed.push((job.session, result));
        }
        completed
    }

    fn drain_randoms(&mut self) -> Vec<(MountSession, Result<mount_store::AccountMounts, String>)> {
        let mut completed = Vec::new();
        while let Some(result) = self.randoms.front().and_then(|job| job.receipt.result()) {
            let job = self
                .randoms
                .pop_front()
                .expect("completed random ownership receipt");
            completed.push((job.session, result));
        }
        completed
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.favourites.is_empty() && self.randoms.is_empty() && self.learning.is_empty()
    }
}

pub(super) fn publish_mount_spell_learning(world: &mut World) {
    let completed = world
        .resource_mut::<PendingMountRequests>()
        .drain_learning();
    for (job, result) in completed {
        match result {
            Ok(()) => {
                world
                    .resource_mut::<PendingMountRequests>()
                    .retire_learned_mounts(job.account, &job.mounts);
                broadcast_account(world, job.account);
            }
            Err(error) => {
                error!(account = job.account, %error, "Mount spell ownership commit failed")
            }
        }
    }
}

pub(super) fn publish_mount_favourites(world: &mut World) {
    let completed = world
        .resource_mut::<PendingMountRequests>()
        .drain_favourites();
    for (session, result) in completed {
        if !has_mount_session(world, &session) {
            continue;
        }
        match result {
            Ok(()) => broadcast_account(world, session.account),
            Err(error) => send_snapshot(world, session.connection, session.player, Some(error)),
        }
    }
}

pub(super) fn publish_random_mounts(world: &mut World) {
    let completed = world.resource_mut::<PendingMountRequests>().drain_randoms();
    for (session, result) in completed {
        if !has_mount_session(world, &session) {
            continue;
        }
        let outcome = result.and_then(|owned| {
            broadcast_account(world, session.account);
            summon_random_from_ownership(world, session.player, &owned)
        });
        if let Err(error) = outcome {
            send_snapshot(world, session.connection, session.player, Some(error));
        }
    }
}

fn has_mount_session(world: &World, session: &MountSession) -> bool {
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
