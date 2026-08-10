use crate::delta::DeltaManager;
use crate::hierarchy::{ReplicateLike, ReplicateLikeChildren};
use crate::registry::registry::ComponentRegistry;
use crate::send::archetypes::ReplicatedArchetypes;
use crate::send::buffer::replicate_entity;
use crate::send::components::{ReplicationGroup, ReplicationGroupId};
use crate::send::sender::ReplicationSender;
use bevy_ecs::change_detection::Tick as BevyTick;
use bevy_ecs::entity::{Entity, EntityIndexSet};
use bevy_ecs::prelude::Query;
use bevy_ecs::relationship::RelationshipTarget;
use bevy_ecs::world::FilteredEntityRef;
use core::mem;
use lightyear_core::tick::Tick;
use lightyear_serde::entity_map::RemoteEntityMap;

pub(crate) struct ReplicationCandidateContext<'a> {
    pub(crate) tick: Tick,
    pub(crate) sender_entity: Entity,
    pub(crate) component_registry: &'a ComponentRegistry,
    pub(crate) replicated_archetypes: &'a ReplicatedArchetypes,
    pub(crate) delta: Option<&'a DeltaManager>,
}

pub(crate) fn queue_initial_sync(
    sender: &mut ReplicationSender,
    roots: &EntityIndexSet,
    dirty_tick: BevyTick,
) {
    if !sender.needs_full_sync {
        return;
    }
    for entity in roots {
        sender.dirty_entities.insert(*entity, dirty_tick);
    }
    sender.needs_full_sync = false;
}

pub(crate) fn process_sender_candidates(
    query: &Query<FilteredEntityRef>,
    entity_mapper: &mut RemoteEntityMap,
    sender: &mut ReplicationSender,
    context: &ReplicationCandidateContext,
) {
    let candidates = mem::take(&mut sender.dirty_entities);
    for (entity, dirty_tick) in candidates {
        #[cfg(feature = "test_diagnostics")]
        crate::test_diagnostics::count_root_candidate();

        if process_dirty_candidate(entity, dirty_tick, query, entity_mapper, sender, context) {
            sender.dirty_entities.insert(entity, dirty_tick);
        }
    }
}

fn process_dirty_candidate(
    entity: Entity,
    dirty_tick: BevyTick,
    query: &Query<FilteredEntityRef>,
    entity_mapper: &mut RemoteEntityMap,
    sender: &mut ReplicationSender,
    context: &ReplicationCandidateContext,
) -> bool {
    let Ok(entity_ref) = query.get(entity) else {
        return false;
    };
    let Some(replicate_like) = entity_ref.get::<ReplicateLike>() else {
        return process_root_candidate(
            entity,
            dirty_tick,
            entity_ref,
            query,
            entity_mapper,
            sender,
            context,
        );
    };
    process_child_candidate(
        entity,
        dirty_tick,
        entity_ref,
        replicate_like.root,
        query,
        entity_mapper,
        sender,
        context,
    )
}

fn process_child_candidate(
    entity: Entity,
    dirty_tick: BevyTick,
    entity_ref: FilteredEntityRef,
    root: Entity,
    query: &Query<FilteredEntityRef>,
    entity_mapper: &mut RemoteEntityMap,
    sender: &mut ReplicationSender,
    context: &ReplicationCandidateContext,
) -> bool {
    let Ok(root_entity_ref) = query.get(root) else {
        return false;
    };
    process_replication_entity(
        entity,
        dirty_tick,
        &root_entity_ref,
        Some((entity_ref, root)),
        entity_mapper,
        sender,
        context,
    )
}

fn process_root_candidate(
    entity: Entity,
    dirty_tick: BevyTick,
    entity_ref: FilteredEntityRef,
    query: &Query<FilteredEntityRef>,
    entity_mapper: &mut RemoteEntityMap,
    sender: &mut ReplicationSender,
    context: &ReplicationCandidateContext,
) -> bool {
    let mut retry = process_replication_entity(
        entity,
        dirty_tick,
        &entity_ref,
        None,
        entity_mapper,
        sender,
        context,
    );
    if let Some(children) = entity_ref.get::<ReplicateLikeChildren>() {
        for child in children.collection() {
            let Ok(child_entity_ref) = query.get(*child) else {
                continue;
            };
            retry |= process_replication_entity(
                *child,
                dirty_tick,
                &entity_ref,
                Some((child_entity_ref, entity)),
                entity_mapper,
                sender,
                context,
            );
        }
    }
    retry
}

fn process_replication_entity(
    entity: Entity,
    dirty_tick: BevyTick,
    root_entity_ref: &FilteredEntityRef,
    child_entity_ref: Option<(FilteredEntityRef, Entity)>,
    entity_mapper: &mut RemoteEntityMap,
    sender: &mut ReplicationSender,
    context: &ReplicationCandidateContext,
) -> bool {
    let child = child_entity_ref.as_ref();
    let (group_id, group_ready) = replication_group_state(entity, root_entity_ref, child);
    replicate_entity(
        entity,
        context.tick,
        root_entity_ref,
        child,
        entity_mapper,
        sender,
        context.sender_entity,
        context.component_registry,
        context.replicated_archetypes,
        context.delta,
    );
    !group_ready || sender.candidate_requires_retry(group_id, dirty_tick)
}

fn replication_group_state(
    entity: Entity,
    root_entity_ref: &FilteredEntityRef,
    child_entity_ref: Option<&(FilteredEntityRef, Entity)>,
) -> (ReplicationGroupId, bool) {
    if let Some((child_entity_ref, root)) = child_entity_ref {
        return child_entity_ref.get::<ReplicationGroup>().map_or_else(
            || {
                let group = root_entity_ref.get::<ReplicationGroup>().unwrap();
                (group.group_id(Some(*root)), group.should_send)
            },
            |group| (group.group_id(Some(entity)), group.should_send),
        );
    }
    let group = root_entity_ref.get::<ReplicationGroup>().unwrap();
    (group.group_id(Some(entity)), group.should_send)
}
