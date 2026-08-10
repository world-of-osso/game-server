use core::mem;

use bevy::{
    ecs::entity::{EntityHashMap, EntityIndexSet},
    platform::sync::{Mutex, MutexGuard},
    prelude::{Entity, Resource},
};

#[derive(Default)]
pub(crate) struct DirtyReplicationEntities {
    pub(crate) all_senders: EntityIndexSet,
    pub(crate) per_sender: EntityHashMap<EntityIndexSet>,
    pub(crate) full_senders: EntityIndexSet,
}

/// Replicated entities that must be reconsidered by the server sender.
///
/// Authoritative component writers call [`mark`](Self::mark) after a real state
/// change. Visibility systems call [`mark_for_sender`](Self::mark_for_sender)
/// for connection-specific gain or loss transitions.
#[derive(Resource, Default)]
pub struct ReplicationDirtyQueue(Mutex<DirtyReplicationEntities>);

impl ReplicationDirtyQueue {
    /// Reconsider this replicated entity for every connected sender.
    pub fn mark(&self, entity: Entity) {
        self.lock().all_senders.insert(entity);
    }

    /// Reconsider this replicated entity only for `sender`.
    pub fn mark_for_sender(&self, entity: Entity, sender: Entity) {
        self.lock()
            .per_sender
            .entry(sender)
            .or_default()
            .insert(entity);
    }

    pub(crate) fn mark_full_sender(&self, sender: Entity) {
        self.lock().full_senders.insert(sender);
    }

    pub(crate) fn drain(&self) -> DirtyReplicationEntities {
        mem::take(&mut *self.lock())
    }

    fn lock(&self) -> MutexGuard<'_, DirtyReplicationEntities> {
        self.0.lock().expect("replication dirty queue poisoned")
    }
}
