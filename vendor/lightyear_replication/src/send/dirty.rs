use crate::hierarchy::ReplicateLike;
use crate::prelude::NetworkVisibility;
use crate::send::components::{Replicate, Replicating, ReplicationGroup};
use crate::send::sender::ReplicationSender;
use bevy_ecs::change_detection::Tick;
use bevy_ecs::entity::{Entity, EntityHashMap, EntityIndexSet};
use bevy_ecs::prelude::{Insert, On, Query, Remove, Res, Resource};
use bevy_platform::sync::Mutex;
use core::mem;

#[derive(Default)]
pub(crate) struct DirtyReplicationEntities {
    all_senders: EntityIndexSet,
    per_sender: EntityHashMap<EntityIndexSet>,
}

impl DirtyReplicationEntities {
    pub(crate) fn apply_to_sender(
        &self,
        sender_entity: Entity,
        sender: &mut ReplicationSender,
        dirty_tick: Tick,
    ) {
        for entity in &self.all_senders {
            sender.dirty_entities.insert(*entity, dirty_tick);
        }
        if let Some(entities) = self.per_sender.get(&sender_entity) {
            for entity in entities {
                sender.dirty_entities.insert(*entity, dirty_tick);
            }
        }
    }
}

/// Replicated entities that must pass through the sender's existing semantic pipeline.
///
/// Component writers call [`mark`](Self::mark) after a real authoritative state change.
/// Visibility systems use [`mark_for_sender`](Self::mark_for_sender) for connection-specific
/// gain/loss transitions. Marks are drained before Lightyear buffers the next replication tick.
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

    pub(crate) fn drain(&self) -> DirtyReplicationEntities {
        mem::take(&mut *self.lock())
    }

    fn lock(&self) -> bevy_platform::sync::MutexGuard<'_, DirtyReplicationEntities> {
        self.0.lock().expect("replication dirty queue poisoned")
    }
}

pub(crate) fn mark_replicated_component_inserted(
    trigger: On<Insert>,
    root_query: Query<&ReplicateLike>,
    dirty_queue: Res<ReplicationDirtyQueue>,
) {
    dirty_queue.mark(replication_root(trigger.entity, &root_query));
}

pub(crate) fn mark_replication_configuration_inserted(
    trigger: On<
        Insert,
        (
            Replicate,
            Replicating,
            ReplicationGroup,
            NetworkVisibility,
            ReplicateLike,
        ),
    >,
    root_query: Query<&ReplicateLike>,
    dirty_queue: Res<ReplicationDirtyQueue>,
) {
    dirty_queue.mark(replication_root(trigger.entity, &root_query));
}

pub(crate) fn mark_network_visibility_removed(
    trigger: On<Remove, NetworkVisibility>,
    root_query: Query<&ReplicateLike>,
    dirty_queue: Res<ReplicationDirtyQueue>,
) {
    dirty_queue.mark(replication_root(trigger.entity, &root_query));
}

fn replication_root(entity: Entity, root_query: &Query<&ReplicateLike>) -> Entity {
    root_query
        .get(entity)
        .map_or(entity, |replicate_like| replicate_like.root)
}
