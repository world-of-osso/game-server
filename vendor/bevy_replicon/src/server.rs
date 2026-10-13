pub mod dirty;
pub mod message;
pub mod related_entities;
pub(super) mod removal_buffer;
pub mod replicated_archetypes;
pub(super) mod replication_messages;
mod replication_query;
pub mod server_tick;
pub mod visibility;

use core::{ops::Range, time::Duration};

use bevy::{
    ecs::{
        archetype::{Archetype, ArchetypeEntity, Archetypes},
        change_detection::{CheckChangeTicks, ComponentTicks, Tick},
        component::{ComponentId, StorageType},
        entity::{Entities, EntityHash, EntityHashMap, EntityIndexSet},
        intern::Interned,
        schedule::ScheduleLabel,
        system::{SystemChangeTick, SystemParam},
    },
    platform::collections::{HashSet, hash_map::Entry},
    prelude::*,
    time::common_conditions::on_timer,
};
use bytes::Buf;
use log::{Level, debug, log_enabled, trace, warn};

use crate::{
    postcard_utils,
    prelude::*,
    server::{
        replicated_archetypes::{ReplicatedArchetype, ReplicatedArchetypes},
        replication_messages::{mutations::MutationsSplit, serialized_data::ErasedComponent},
        visibility::registry::FilterRegistry,
    },
    shared::{
        backend::channels::ClientChannel,
        message::server_message::message_buffer::MessageBuffer,
        replication::{
            client_ticks::{ClientTicks, EntityTicks},
            registry::{
                ComponentIndex, ReplicationRegistry, component_mask::ComponentMask,
                ctx::SerializeCtx,
            },
            rules::{ReplicationRules, component::ComponentRule},
            storage::ReplicationStorage,
            track_mutate_messages::TrackMutateMessages,
            visibility::VisibilityScope,
        },
    },
};
pub use dirty::ReplicationDirtyQueue;
use related_entities::RelatedEntities;
use removal_buffer::RemovalBuffer;
use replication_messages::{
    mutations::Mutations, serialized_data::SerializedData, updates::Updates,
};
use replication_query::ReplicationQuery;
use server_tick::ServerTick;
use visibility::client_visibility::ClientVisibility;

#[derive(Resource, Default)]
struct ReplicationCandidates(EntityHashMap<EntityIndexSet>);

impl ReplicationCandidates {
    fn clear(&mut self) {
        self.0.clear();
    }

    fn insert(&mut self, entity: Entity, sender: Entity) {
        self.0.entry(entity).or_default().insert(sender);
    }

    fn iter(&self) -> impl Iterator<Item = (&Entity, &EntityIndexSet)> {
        self.0.iter()
    }

    fn take(&mut self) -> EntityHashMap<EntityIndexSet> {
        core::mem::take(&mut self.0)
    }
}

type ReplicationClients<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Updates,
        &'static mut Mutations,
        &'static mut ClientTicks,
        &'static mut PriorityMap,
        &'static mut ClientVisibility,
    ),
>;

#[derive(SystemParam)]
struct CollectChangesParams<'w, 's> {
    server_tick: Res<'w, ServerTick>,
    change_tick: Res<'w, ServerChangeTick>,
    registry: Res<'w, ReplicationRegistry>,
    filter_registry: Res<'w, FilterRegistry>,
    type_registry: Res<'w, AppTypeRegistry>,
    related_entities: Res<'w, RelatedEntities>,
    rules: Res<'w, ReplicationRules>,
    candidates: ResMut<'w, ReplicationCandidates>,
    dirty_queue: Res<'w, ReplicationDirtyQueue>,
    replication_storage: ResMut<'w, ReplicationStorage>,
    replicated_archetypes: ResMut<'w, ReplicatedArchetypes>,
    serialized: ResMut<'w, SerializedData>,
    removal_buffer: ResMut<'w, RemovalBuffer>,
    clients: ReplicationClients<'w, 's>,
}

pub struct ServerPlugin {
    /// Schedule in which [`ServerTick`] is incremented.
    ///
    /// By default it's set to [`FixedPostUpdate`].
    /// Use [`Self::new`] to avoid calling [`ScheduleLabel::intern`].
    ///
    /// You can also set it to `None` to trigger replication by manually
    /// incrementing [`ServerTick`] or scheduling [`increment_tick`].
    ///
    /// # Examples
    ///
    /// Run every frame.
    ///
    /// ```
    /// use bevy::{ecs::schedule::ScheduleLabel, prelude::*, state::app::StatesPlugin};
    /// use bevy_replicon::prelude::*;
    ///
    /// # let mut app = App::new();
    /// app.add_plugins((
    ///     MinimalPlugins,
    ///     StatesPlugin,
    ///     RepliconPlugins.build().set(ServerPlugin {
    ///         // `ScheduleLabel` needs to be imported to call `intern`.
    ///         tick_schedule: Some(PostUpdate.intern()),
    ///         ..Default::default()
    ///     }),
    /// ));
    /// ```
    pub tick_schedule: Option<Interned<dyn ScheduleLabel>>,

    /// The time after which mutations will be considered lost if an acknowledgment is not received for them.
    ///
    /// In practice mutations will live at least `mutations_timeout`, and at most `2*mutations_timeout`.
    pub mutations_timeout: Duration,
}

impl ServerPlugin {
    /// Creates a plugin with the given [`Self::tick_schedule`].
    pub fn new(tick_schedule: impl ScheduleLabel) -> Self {
        Self {
            tick_schedule: Some(tick_schedule.intern()),
            mutations_timeout: Duration::from_secs(10),
        }
    }
}

impl Default for ServerPlugin {
    fn default() -> Self {
        Self::new(FixedPostUpdate)
    }
}

/// Server functionality and replication sending.
///
/// Can be disabled for client-only apps.
impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DespawnBuffer>()
            .init_resource::<RemovalBuffer>()
            .init_resource::<SerializedData>()
            .init_resource::<ServerMessages>()
            .init_resource::<ServerTick>()
            .init_resource::<ServerChangeTick>()
            .init_resource::<ReplicatedArchetypes>()
            .init_resource::<MessageBuffer>()
            .init_resource::<RelatedEntities>()
            .init_resource::<FilterRegistry>()
            .init_resource::<ReplicationDirtyQueue>()
            .init_resource::<ReplicationCandidates>()
            .configure_sets(
                PreUpdate,
                (ServerSystems::ReceivePackets, ServerSystems::Receive).chain(),
            )
            .configure_sets(
                PostUpdate,
                (
                    ServerSystems::IncrementTick,
                    ServerSystems::Send,
                    ServerSystems::SendPackets,
                )
                    .chain(),
            )
            .add_observer(handle_connect)
            .add_observer(handle_disconnect)
            .add_observer(check_mutation_ticks)
            .add_observer(buffer_despawn)
            .add_observer(mark_replicated_entity_inserted)
            .add_observer(cleanup_storage)
            .add_systems(
                PreUpdate,
                (
                    receive_acks,
                    cleanup_acks(self.mutations_timeout).run_if(on_timer(self.mutations_timeout)),
                )
                    .chain()
                    .in_set(ServerSystems::Receive)
                    .run_if(in_state(ServerState::Running)),
            )
            .add_systems(OnExit(ServerState::Running), reset)
            .add_systems(
                PostUpdate,
                (
                    prepare_messages,
                    prepare_candidates,
                    collect_mappings,
                    collect_despawns,
                    collect_removals,
                    collect_changes,
                    send_messages,
                )
                    .chain()
                    .run_if(resource_changed::<ServerTick>)
                    .in_set(ServerSystems::Send)
                    .run_if(in_state(ServerState::Running)),
            );

        if let Some(tick_schedule) = self.tick_schedule {
            debug!("using tick schedule `{tick_schedule:?}`");
            app.add_systems(
                tick_schedule,
                increment_tick
                    .in_set(ServerSystems::IncrementTick)
                    .run_if(in_state(ServerState::Running)),
            );
        }

        let auth_method = app.world().resource::<AuthMethod>();
        debug!("using authorization method `{auth_method:?}`");
        match auth_method {
            AuthMethod::ProtocolCheck => {
                app.add_observer(check_protocol);
            }
            AuthMethod::None => {
                app.register_required_components::<ConnectedClient, AuthorizedClient>();
            }
            AuthMethod::Custom => (),
        }

        if log_enabled!(Level::Debug) {
            app.add_systems(OnEnter(ServerState::Running), || debug!("running"))
                .add_systems(OnEnter(ServerState::Stopped), || debug!("stopped"));
        }
    }

    fn finish(&self, app: &mut App) {
        // Multiple rules can include components with the same ID,
        // we collect them here to deduplicate.
        let rules = app.world().resource::<ReplicationRules>();
        let replicated_ids: HashSet<_> = rules
            .iter()
            .flat_map(|rule| &rule.components)
            .map(|component| component.id)
            .collect();

        // Observers without any components trigger on every insertion or removal.
        if !replicated_ids.is_empty() {
            let mut insert_observer = Observer::new(mark_replicated_component_inserted);
            let mut remove_observer = Observer::new(buffer_removals);
            for id in replicated_ids {
                insert_observer = insert_observer.with_component(id);
                remove_observer = remove_observer.with_component(id);
            }
            app.world_mut().spawn(insert_observer);
            app.world_mut().spawn(remove_observer);
        }

        app.world_mut()
            .resource_scope(|world, mut messages: Mut<ServerMessages>| {
                let channels = world.resource::<RepliconChannels>();
                messages.setup_client_channels(channels.client_channels().len());
            });
    }
}

fn handle_connect(
    add: On<Add, ConnectedClient>,
    mut message_buffer: ResMut<MessageBuffer>,
    dirty_queue: Res<ReplicationDirtyQueue>,
) {
    debug!("client `{}` connected", add.entity);
    message_buffer.exclude_client(add.entity);
    dirty_queue.mark_full_sender(add.entity);
}

fn mark_replicated_entity_inserted(
    insert: On<Insert, Replicated>,
    dirty_queue: Res<ReplicationDirtyQueue>,
) {
    dirty_queue.mark(insert.entity);
}

fn mark_replicated_component_inserted(insert: On<Insert>, dirty_queue: Res<ReplicationDirtyQueue>) {
    dirty_queue.mark(insert.entity);
}

fn handle_disconnect(remove: On<Remove, ConnectedClient>, mut messages: ResMut<ServerMessages>) {
    debug!("client `{}` disconnected", remove.entity);
    messages.remove_client(remove.entity);
}

fn check_protocol(
    client_protocol: On<FromClient<ProtocolHash>>,
    mut commands: Commands,
    mut disconnects: MessageWriter<DisconnectRequest>,
    protocol: Res<ProtocolHash>,
) {
    let client = client_protocol
        .client_id
        .entity()
        .expect("protocol hash sent only from clients");

    if **client_protocol == *protocol {
        debug!("marking client `{client}` as authorized");
        commands.entity(client).insert(AuthorizedClient);
    } else {
        debug!(
            "disconnecting client `{client}` due to protocol mismatch (client: `{:?}`, server: `{:?}`)",
            **client_protocol, *protocol
        );
        commands.server_trigger(ToClients {
            targets: SendTargets::Single(client_protocol.client_id),
            message: ProtocolMismatch,
        });
        disconnects.write(DisconnectRequest { client });
    }
}

fn check_mutation_ticks(check: On<CheckChangeTicks>, mut clients: Query<&mut ClientTicks>) {
    debug!(
        "checking mutation ticks for overflow for {:?}",
        check.present_tick()
    );
    for mut ticks in &mut clients {
        for entity_ticks in ticks.entities.values_mut() {
            entity_ticks.system_tick.check_tick(*check);
        }
    }
}

/// Increments current server tick which causes the server to replicate this frame.
pub fn increment_tick(mut server_tick: ResMut<ServerTick>) {
    trace!("incrementing `{:?}`", *server_tick);
    server_tick.increment();
}

fn buffer_removals(
    remove: On<Remove>,
    entities: &Entities,
    archetypes: &Archetypes,
    state: Res<State<ServerState>>,
    mut replicated_archetypes: ResMut<ReplicatedArchetypes>,
    rules: Res<ReplicationRules>,
    registry: Option<Res<ReplicationRegistry>>,
    mut removals: ResMut<RemovalBuffer>,
    dirty_queue: Res<ReplicationDirtyQueue>,
) {
    if *state != ServerState::Running {
        return;
    }

    let components = remove.trigger().components;
    if components.contains(&replicated_archetypes.marker_id()) {
        trace!("ignoring removals for despawned `{}`", remove.entity);
        return;
    }

    // Observers can't use run conditions. We return early on the client, but system parameters
    // are validated before the observer runs. Because of this, the registry may not be present
    // in the world during replication receive, so it needs to be optional.
    let registry = registry.expect("registry should always exist on the server");

    replicated_archetypes.update(archetypes, &rules);
    let location = entities.get_spawned(remove.entity).unwrap();
    let Some(archetype) = replicated_archetypes.get(location.archetype_id) else {
        // `Replicated` component is missing.
        trace!(
            "ignoring `{components:?}` removal for non-replicated `{}`",
            remove.entity
        );
        return;
    };

    removals.insert(remove.entity, components, archetype, &registry);
    dirty_queue.mark(remove.entity);
}

fn buffer_despawn(
    remove: On<Remove, Replicated>,
    mut despawn_buffer: ResMut<DespawnBuffer>,
    state: Res<State<ServerState>>,
) {
    if *state == ServerState::Running {
        trace!("buffering despawn of `{}`", remove.entity);
        despawn_buffer.push(remove.entity);
    }
}

fn cleanup_acks(
    mutations_timeout: Duration,
) -> impl FnMut(Query<&mut ClientTicks>, Res<Time<Real>>) {
    move |mut clients: Query<&mut ClientTicks>, time: Res<Time<Real>>| {
        let min_timestamp = time.elapsed().saturating_sub(mutations_timeout);
        for mut ticks in &mut clients {
            ticks.cleanup_older_mutations(min_timestamp);
        }
    }
}

fn receive_acks(mut messages: ResMut<ServerMessages>, mut clients: Query<&mut ClientTicks>) {
    for (client, mut message) in messages.receive(ClientChannel::MutationAcks) {
        let Ok(mut ticks) = clients.get_mut(client) else {
            debug!("ignoring acks for disconnected client `{client}`");
            continue;
        };
        while message.has_remaining() {
            match postcard_utils::from_buf(&mut message) {
                Ok(mutate_index) => {
                    ticks.ack_mutate_message(client, mutate_index);
                }
                Err(e) => {
                    debug!("unable to deserialize mutate index from client `{client}`: {e}")
                }
            }
        }
    }
}

fn prepare_messages(
    change_tick: SystemChangeTick,
    mut related_entities: ResMut<RelatedEntities>,
    mut server_change_tick: ResMut<ServerChangeTick>,
    clients: Query<(&mut Updates, &mut Mutations)>,
) {
    **server_change_tick = change_tick.this_run();
    related_entities.rebuild_graphs();

    for (mut updates, mut mutations) in clients {
        updates.clear();
        mutations.reset(related_entities.graphs_count());
    }
}

fn prepare_candidates(
    dirty_queue: Res<ReplicationDirtyQueue>,
    replicated: Query<Entity, With<Replicated>>,
    senders: Query<(Entity, &ClientVisibility), With<ConnectedClient>>,
    filter_registry: Res<FilterRegistry>,
    mut candidates: ResMut<ReplicationCandidates>,
) {
    candidates.clear();

    let dirty = dirty_queue.drain();
    let connected_senders: EntityIndexSet = senders.iter().map(|(entity, _)| entity).collect();

    for entity in dirty.all_senders {
        for sender in &connected_senders {
            candidates.insert(entity, *sender);
        }
    }

    for (sender, entities) in dirty.per_sender {
        if !connected_senders.contains(&sender) {
            continue;
        }
        for entity in entities {
            candidates.insert(entity, sender);
        }
    }

    for sender in dirty.full_senders {
        if !connected_senders.contains(&sender) {
            continue;
        }
        let (_, visibility) = senders.get(sender).expect("connected replication sender");
        if let Some(visible) = visibility.explicitly_visible() {
            for entity in visible {
                if replicated.contains(entity)
                    && !visibility.get(entity).is_hidden(&filter_registry)
                {
                    candidates.insert(entity, sender);
                }
            }
        } else {
            for entity in &replicated {
                if !visibility.get(entity).is_hidden(&filter_registry) {
                    candidates.insert(entity, sender);
                }
            }
        }
    }
}

/// Collects and writes any new entity mappings that happened in this tick.
fn collect_mappings(
    despawn_buffer: Res<DespawnBuffer>,
    registry: Res<FilterRegistry>,
    candidates: Res<ReplicationCandidates>,
    mut serialized: ResMut<SerializedData>,
    signatures: Query<&Signature, With<Replicated>>,
    mut clients: Query<(
        Entity,
        &mut Updates,
        &mut ClientTicks,
        &mut ClientVisibility,
    )>,
) -> Result<()> {
    for (&entity, target_senders) in candidates.iter() {
        let Ok(signature) = signatures.get(entity) else {
            continue;
        };
        let hash = signature.hash();
        let mut mapping_range = None;

        if let Some(sender) = signature.client() {
            if !target_senders.contains(&sender) {
                continue;
            }
            write_mapping_for_sender(
                entity,
                sender,
                hash,
                &despawn_buffer,
                &registry,
                &mut serialized,
                &mut mapping_range,
                &mut clients,
            )?;
            continue;
        }

        for sender in target_senders {
            write_mapping_for_sender(
                entity,
                *sender,
                hash,
                &despawn_buffer,
                &registry,
                &mut serialized,
                &mut mapping_range,
                &mut clients,
            )?;
        }
    }

    Ok(())
}

fn write_mapping_for_sender(
    entity: Entity,
    sender: Entity,
    hash: u64,
    despawn_buffer: &DespawnBuffer,
    registry: &FilterRegistry,
    serialized: &mut SerializedData,
    mapping_range: &mut Option<core::ops::Range<usize>>,
    clients: &mut Query<(
        Entity,
        &mut Updates,
        &mut ClientTicks,
        &mut ClientVisibility,
    )>,
) -> Result<()> {
    let Ok((_, mut message, ticks, visibility)) = clients.get_mut(sender) else {
        return Ok(());
    };
    if !should_send_mapping(entity, despawn_buffer, registry, &visibility, &ticks) {
        return Ok(());
    }

    trace!("writing mapping `{entity}` to 0x{hash:016x} for client `{sender}`");
    let range = serialized.write_cached_mapping(mapping_range, entity, hash)?;
    message.add_mapping(range);
    Ok(())
}

fn should_send_mapping(
    entity: Entity,
    despawn_buffer: &DespawnBuffer,
    registry: &FilterRegistry,
    visibility: &ClientVisibility,
    ticks: &ClientTicks,
) -> bool {
    // Since despawns processed later, we need to explicitly check for them here
    // because we can't distinguish between a despawn and removal of a visibility filter.
    if visibility.get(entity).is_hidden(registry) || despawn_buffer.contains(&entity) {
        return false;
    }

    // Check if the client already received the entity.
    !ticks.entities.contains_key(&entity)
}

/// Collect entity despawns from this tick into update messages.
fn collect_despawns(
    registry: Res<FilterRegistry>,
    mut serialized: ResMut<SerializedData>,
    mut despawn_buffer: ResMut<DespawnBuffer>,
    mut clients: Query<(
        Entity,
        &mut Updates,
        &mut ClientTicks,
        &mut PriorityMap,
        &mut ClientVisibility,
    )>,
) -> Result<()> {
    for entity in despawn_buffer.drain(..) {
        let entity_range = serialized.write_entity(entity)?;
        for (client, mut message, mut ticks, mut priority, mut visibility) in &mut clients {
            if ticks.entities.remove(&entity).is_some() {
                // Write despawn only if the entity was previously sent because
                // spawn and despawn could happen during the same tick.
                trace!("writing despawn for `{entity}` for client `{client}`");
                message.add_despawn(entity_range.clone());
            }
            visibility.remove_despawned(entity);
            priority.remove(&entity);
        }
    }

    for (client, mut message, mut ticks, mut priority, visibility) in clients {
        for (entity, filter_mask) in visibility.iter_lost() {
            // Skip visibility changes that hide only components.
            if !filter_mask.is_hidden(&registry) {
                continue;
            }

            if ticks.entities.remove(&entity).is_some() {
                trace!("writing visibility lost for `{entity}` for client `{client}`");
                let entity_range = serialized.write_entity(entity)?;
                message.add_despawn(entity_range);
            }
            priority.remove(&entity);
        }
    }

    Ok(())
}

/// Collects component removals from this tick into update messages.
///
/// The removal buffer will be cleaned later in [`collect_changes`].
fn collect_removals(
    archetypes: &Archetypes,
    entities: &Entities,
    removal_buffer: Res<RemovalBuffer>,
    rules: Res<ReplicationRules>,
    registry: Res<ReplicationRegistry>,
    filter_registry: Res<FilterRegistry>,
    mut replicated_archetypes: ResMut<ReplicatedArchetypes>,
    mut serialized: ResMut<SerializedData>,
    mut lost_buffer: Local<Vec<ComponentIndex>>,
    mut clients: Query<(
        Entity,
        &mut Updates,
        &mut ClientTicks,
        &mut ClientVisibility,
    )>,
) -> Result<()> {
    replicated_archetypes.update(archetypes, &rules);

    for (&entity, remove_ids) in removal_buffer.iter() {
        let mut entity_range = None;
        for (_, mut message, _, _) in &mut clients {
            message.start_entity_removals();
        }

        for &(component_index, fns_id) in remove_ids {
            let mut fns_id_range = None;
            for (client, mut message, mut ticks, _) in &mut clients {
                // Only send removals for components that were previously sent.
                // If the entity was despawned or lost visibility, it was removed
                // from ticks earlier during despawn collection.
                let Some(entity_ticks) = ticks.entities.get_mut(&entity) else {
                    continue;
                };
                if !entity_ticks.components.contains(component_index) {
                    continue;
                }

                trace!("writing `{fns_id:?}` removal for `{entity}` for client `{client}`");
                if !message.removals_entity_added() {
                    let entity_range = serialized.write_cached_entity(&mut entity_range, entity)?;
                    message.add_removals_entity(entity_range);
                }
                let fns_id_range = serialized.write_cached_fns_id(&mut fns_id_range, fns_id)?;
                message.add_removal(fns_id_range);
                entity_ticks.remove_component(component_index);
            }
        }
    }

    for (client, mut message, mut ticks, mut visibility) in &mut clients {
        for (entity, filter_mask) in visibility.drain_lost() {
            if filter_mask.is_hidden(&filter_registry) {
                // Was processed earlier during collecting despawns.
                continue;
            }
            let Some(entity_ticks) = ticks.entities.get_mut(&entity) else {
                // The client didn't see this entity.
                continue;
            };
            let Ok(location) = entities.get_spawned(entity) else {
                warn!(
                    "`{entity}` was despawned after despawn processing but before sending, \
                     so the despawn will be sent on the next tick; \
                     consider ordering your despawn before `{:?}`",
                    ServerSystems::Send
                );
                continue;
            };
            let archetype = replicated_archetypes
                .get(location.archetype_id)
                .unwrap_or_else(|| {
                    panic!("`{entity}` should be replicated because the client knows about it")
                });

            let mut entity_range = None;
            message.start_entity_removals();

            // Writes a removal for a lost component and drops it from the client's ticks.
            let mut write_lost = |component_index, entity_ticks: &mut EntityTicks| -> Result<()> {
                let &(id, _) = registry.get_by_index(component_index).unwrap_or_else(|| {
                    panic!("`{component_index:?}` should've been registered to be marked as lost")
                });
                let rule = archetype.find_rule(id).unwrap_or_else(|| {
                    panic!("`{id:?}` should match a rule since the client knows about it")
                });

                trace!(
                    "writing `{:?}` lost for `{entity}` for client `{client}`",
                    rule.fns_id
                );
                if !message.removals_entity_added() {
                    let entity_range = serialized.write_cached_entity(&mut entity_range, entity)?;
                    message.add_removals_entity(entity_range);
                }
                let fns_id_range = serialized.write_fns_id(rule.fns_id)?;
                message.add_removal(fns_id_range);
                entity_ticks.remove_component(component_index);

                Ok(())
            };

            for scope in filter_mask.scopes(&filter_registry) {
                match scope {
                    VisibilityScope::Entity => {
                        unreachable!("entity filters are processed during despawn collection")
                    }
                    VisibilityScope::Components(mask) => {
                        for component_index in mask.iter() {
                            if entity_ticks.components.contains(component_index) {
                                write_lost(component_index, entity_ticks)?;
                            }
                        }
                    }
                    VisibilityScope::AllExcept(mask) => {
                        // Buffered to release the borrow before removing.
                        lost_buffer.extend(
                            entity_ticks
                                .components
                                .iter()
                                .filter(|&component_index| !mask.contains(component_index)),
                        );
                        for component_index in lost_buffer.drain(..) {
                            write_lost(component_index, entity_ticks)?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

struct CandidateChangeContext<'a, 'query_world, 'query_state, 'client_world, 'client_state> {
    query: &'a ReplicationQuery<'query_world, 'query_state>,
    server_tick: RepliconTick,
    change_tick: Tick,
    registry: &'a ReplicationRegistry,
    filter_registry: &'a FilterRegistry,
    type_registry: &'a AppTypeRegistry,
    related_entities: &'a RelatedEntities,
    dirty_queue: &'a ReplicationDirtyQueue,
    replication_storage: &'a mut ReplicationStorage,
    serialized: &'a mut SerializedData,
    removal_buffer: &'a RemovalBuffer,
    clients: &'a mut ReplicationClients<'client_world, 'client_state>,
}

impl CandidateChangeContext<'_, '_, '_, '_, '_> {
    fn collect_candidate(
        &mut self,
        archetype: &Archetype,
        replicated_archetype: &ReplicatedArchetype,
        entity: &ArchetypeEntity,
        target_senders: &EntityIndexSet,
    ) -> Result<()> {
        self.start_candidate(target_senders);
        let mut entity_range = None;
        for &(rule, storage) in &replicated_archetype.components {
            self.collect_component(
                archetype,
                entity,
                target_senders,
                rule,
                storage,
                &mut entity_range,
            )?;
        }
        self.finish_candidate(entity.id(), target_senders, &mut entity_range)
    }

    fn start_candidate(&mut self, target_senders: &EntityIndexSet) {
        for sender in target_senders {
            let Ok((_, mut updates, mut mutations, ..)) = self.clients.get_mut(*sender) else {
                continue;
            };
            #[cfg(feature = "test_diagnostics")]
            {
                crate::test_diagnostics::count_root_candidate();
                crate::test_diagnostics::count_replicate_entity_visit();
            }
            updates.start_entity_changes();
            mutations.start_entity();
        }
    }

    fn collect_component(
        &mut self,
        archetype: &Archetype,
        entity: &ArchetypeEntity,
        target_senders: &EntityIndexSet,
        rule: ComponentRule,
        storage: StorageType,
        entity_range: &mut Option<Range<usize>>,
    ) -> Result<()> {
        let (component_index, component_id, fns) = self.registry.get(rule.fns_id);
        // SAFETY: component and storage were obtained from this archetype.
        let (ptr, ticks) = unsafe {
            self.query
                .get_component_unchecked(entity, archetype.table_id(), storage, component_id)
        };
        // SAFETY: `fns` and `ptr` were created for the same component type.
        let mut component = unsafe { ErasedComponent::new(fns, ptr, rule.fns_id) };
        let mut component_range = None;

        for sender in target_senders {
            self.collect_sender_component(
                entity.id(),
                *sender,
                rule,
                component_id,
                component_index,
                ticks,
                &mut component,
                &mut component_range,
                entity_range,
            )?;
        }
        Ok(())
    }

    fn collect_sender_component(
        &mut self,
        entity: Entity,
        sender: Entity,
        rule: ComponentRule,
        component_id: ComponentId,
        component_index: ComponentIndex,
        ticks: ComponentTicks,
        component: &mut ErasedComponent,
        component_range: &mut Option<Range<usize>>,
        entity_range: &mut Option<Range<usize>>,
    ) -> Result<()> {
        let Ok((client, mut updates, mut mutations, client_ticks, priority, visibility)) =
            self.clients.get_mut(sender)
        else {
            return Ok(());
        };
        if visibility
            .get(entity)
            .is_component_hidden(self.filter_registry, component_index)
        {
            return Ok(());
        }

        if let Some(entity_ticks) = client_ticks.entities.get(&entity)
            && entity_ticks.components.contains(component_index)
        {
            return write_mutation(
                entity,
                client,
                rule,
                component_id,
                component_index,
                ticks,
                self.server_tick,
                self.change_tick,
                self.type_registry,
                self.related_entities,
                self.dirty_queue,
                self.replication_storage,
                self.serialized,
                component,
                component_range,
                entity_range,
                &mut mutations,
                entity_ticks,
                &priority,
            );
        }

        write_insertion(
            entity,
            client,
            rule,
            component_id,
            component_index,
            ticks,
            self.server_tick,
            self.type_registry,
            self.replication_storage,
            self.serialized,
            component,
            component_range,
            entity_range,
            &mut updates,
        )
    }

    fn finish_candidate(
        &mut self,
        entity: Entity,
        target_senders: &EntityIndexSet,
        entity_range: &mut Option<Range<usize>>,
    ) -> Result<()> {
        for sender in target_senders {
            let Ok((client, mut updates, mut mutations, mut ticks, _, visibility)) =
                self.clients.get_mut(*sender)
            else {
                continue;
            };
            finish_sender_changes(
                entity,
                client,
                self.server_tick,
                self.change_tick,
                self.filter_registry,
                self.removal_buffer,
                self.serialized,
                entity_range,
                &mut updates,
                &mut mutations,
                &mut ticks,
                &visibility,
            )?;
        }
        Ok(())
    }
}

/// Collects component changes only for explicitly dirty entity/sender pairs.
fn collect_changes(
    archetypes: &Archetypes,
    entities: &Entities,
    query: ReplicationQuery,
    mut params: CollectChangesParams,
) -> Result<()> {
    params
        .replicated_archetypes
        .update(archetypes, &params.rules);
    collect_all_candidates(archetypes, entities, &query, &mut params)?;
    params.removal_buffer.clear();
    Ok(())
}

fn collect_all_candidates(
    archetypes: &Archetypes,
    entities: &Entities,
    query: &ReplicationQuery,
    params: &mut CollectChangesParams,
) -> Result<()> {
    let candidates = params.candidates.take();
    for (candidate, target_senders) in candidates {
        collect_candidate(
            candidate,
            &target_senders,
            archetypes,
            entities,
            query,
            params,
        )?;
    }
    Ok(())
}

fn collect_candidate(
    candidate: Entity,
    target_senders: &EntityIndexSet,
    archetypes: &Archetypes,
    entities: &Entities,
    query: &ReplicationQuery,
    params: &mut CollectChangesParams,
) -> Result<()> {
    let Some((archetype, replicated_archetype, entity)) = find_candidate(
        candidate,
        entities,
        archetypes,
        &params.replicated_archetypes,
    ) else {
        return Ok(());
    };

    let mut context = CandidateChangeContext {
        query,
        server_tick: **params.server_tick,
        change_tick: **params.change_tick,
        registry: &params.registry,
        filter_registry: &params.filter_registry,
        type_registry: &params.type_registry,
        related_entities: &params.related_entities,
        dirty_queue: &params.dirty_queue,
        replication_storage: &mut params.replication_storage,
        serialized: &mut params.serialized,
        removal_buffer: &params.removal_buffer,
        clients: &mut params.clients,
    };
    context.collect_candidate(archetype, replicated_archetype, entity, target_senders)
}

fn find_candidate<'a>(
    candidate: Entity,
    entities: &Entities,
    archetypes: &'a Archetypes,
    replicated_archetypes: &'a ReplicatedArchetypes,
) -> Option<(&'a Archetype, &'a ReplicatedArchetype, &'a ArchetypeEntity)> {
    let location = entities.get(candidate).ok()??;
    let replicated_archetype = replicated_archetypes.get(location.archetype_id)?;
    // SAFETY: the replicated archetype came from the candidate's current location.
    let archetype = unsafe { archetypes.get(replicated_archetype.id).unwrap_unchecked() };
    let entity = archetype.entities().get(location.archetype_row.index())?;
    Some((archetype, replicated_archetype, entity))
}

fn write_mutation(
    entity: Entity,
    client: Entity,
    rule: ComponentRule,
    component_id: ComponentId,
    component_index: ComponentIndex,
    ticks: ComponentTicks,
    server_tick: RepliconTick,
    change_tick: Tick,
    type_registry: &AppTypeRegistry,
    related_entities: &RelatedEntities,
    dirty_queue: &ReplicationDirtyQueue,
    replication_storage: &mut ReplicationStorage,
    serialized: &mut SerializedData,
    component: &mut ErasedComponent,
    component_range: &mut Option<Range<usize>>,
    entity_range: &mut Option<Range<usize>>,
    mutations: &mut Mutations,
    entity_ticks: &EntityTicks,
    priority: &PriorityMap,
) -> Result<()> {
    let changed = rule.mode != ReplicationMode::Once
        && ticks.is_changed(entity_ticks.system_tick, change_tick);
    if !changed {
        return Ok(());
    }

    dirty_queue.mark_for_sender(entity, client);
    if !mutation_priority_ready(entity, server_tick, entity_ticks, priority) {
        return Ok(());
    }

    trace!(
        "writing `{:?}` mutation for `{entity}` for client `{client}`",
        rule.fns_id,
    );
    if !mutations.entity_added() {
        let graph_index = related_entities.graph_index(entity);
        let range = serialized.write_cached_entity(entity_range, entity)?;
        mutations.add_entity(entity, graph_index, range);
    }

    let mut ctx = SerializeCtx {
        entity,
        component_id,
        last_changed: ticks.changed,
        server_tick,
        diff_cursor: None,
        type_registry,
        storage: replication_storage,
    };
    let range = serialize_mutation(
        &mut ctx,
        component_index,
        component,
        component_range,
        serialized,
        entity_ticks,
        mutations,
    )?;
    mutations.add_component(range);
    Ok(())
}

fn mutation_priority_ready(
    entity: Entity,
    server_tick: RepliconTick,
    entity_ticks: &EntityTicks,
    priority: &PriorityMap,
) -> bool {
    let base_priority = priority.get(&entity).copied().unwrap_or(1.0);
    let tick_diff = server_tick - entity_ticks.server_tick;
    base_priority * (tick_diff as f32) >= 1.0
}

fn serialize_mutation(
    ctx: &mut SerializeCtx,
    component_index: ComponentIndex,
    component: &mut ErasedComponent,
    component_range: &mut Option<Range<usize>>,
    serialized: &mut SerializedData,
    entity_ticks: &EntityTicks,
    mutations: &mut Mutations,
) -> Result<Range<usize>> {
    let diff_cursor = entity_ticks.diff_cursor(component_index);
    if diff_cursor.is_none() {
        return serialized.write_cached_component(ctx, component_range, component);
    }

    ctx.diff_cursor = diff_cursor;
    let range = serialized.write_component(ctx, component)?;
    if let Some(cursor) = ctx.diff_cursor.take() {
        mutations.add_diff_cursor(component_index, cursor);
    }
    Ok(range)
}

fn write_insertion(
    entity: Entity,
    client: Entity,
    rule: ComponentRule,
    component_id: ComponentId,
    component_index: ComponentIndex,
    ticks: ComponentTicks,
    server_tick: RepliconTick,
    type_registry: &AppTypeRegistry,
    replication_storage: &mut ReplicationStorage,
    serialized: &mut SerializedData,
    component: &mut ErasedComponent,
    component_range: &mut Option<Range<usize>>,
    entity_range: &mut Option<Range<usize>>,
    updates: &mut Updates,
) -> Result<()> {
    trace!(
        "writing `{:?}` insertion for `{entity}` for client `{client}`",
        rule.fns_id,
    );
    if !updates.changed_entity_added() {
        let range = serialized.write_cached_entity(entity_range, entity)?;
        updates.add_changed_entity(range);
    }

    let mut ctx = SerializeCtx {
        entity,
        component_id,
        last_changed: ticks.changed,
        server_tick,
        diff_cursor: None,
        type_registry,
        storage: replication_storage,
    };
    let range = serialized.write_cached_component(&mut ctx, component_range, component)?;
    updates.add_inserted_component(range, component_index);
    Ok(())
}

fn finish_sender_changes(
    entity: Entity,
    client: Entity,
    server_tick: RepliconTick,
    change_tick: Tick,
    filter_registry: &FilterRegistry,
    removal_buffer: &RemovalBuffer,
    serialized: &mut SerializedData,
    entity_range: &mut Option<Range<usize>>,
    updates: &mut Updates,
    mutations: &mut Mutations,
    ticks: &mut ClientTicks,
    visibility: &ClientVisibility,
) -> Result<()> {
    if visibility.get(entity).is_hidden(filter_registry) {
        return Ok(());
    }

    let entity_ticks = ticks.entities.entry(entity);
    let new_for_client = matches!(entity_ticks, Entry::Vacant(_));
    let has_atomic_update =
        new_for_client || updates.changed_entity_added() || removal_buffer.contains_key(&entity);
    if has_atomic_update {
        if mutations.entity_added() {
            trace!("merging mutations for `{entity}` with updates for client `{client}`");
            updates.take_added_entity(mutations);
        }
        update_ticks(
            entity_ticks,
            change_tick,
            server_tick,
            updates.take_changed_components(),
        );
    }

    if new_for_client && !updates.changed_entity_added() {
        trace!("writing empty `{entity}` for client `{client}`");
        let range = serialized.write_cached_entity(entity_range, entity)?;
        updates.add_changed_entity(range);
    }
    Ok(())
}

fn update_ticks(
    entity_ticks: Entry<Entity, EntityTicks, EntityHash>,
    system_tick: Tick,
    server_tick: RepliconTick,
    components: ComponentMask,
) {
    match entity_ticks {
        Entry::Occupied(entry) => {
            let entity_ticks = entry.into_mut();
            entity_ticks.system_tick = system_tick;
            entity_ticks.server_tick = server_tick;
            entity_ticks.components |= &components;
        }
        Entry::Vacant(entry) => {
            entry.insert(EntityTicks::new(server_tick, system_tick, components));
        }
    }
}

/// Sends previously constructed [`Updates`] and [`Mutations`].
fn send_messages(
    mut split_buffer: Local<Vec<MutationsSplit>>,
    time: Res<Time<Real>>,
    server_tick: Res<ServerTick>,
    change_tick: Res<ServerChangeTick>,
    track_mutate_messages: Res<TrackMutateMessages>,
    mut serialized: ResMut<SerializedData>,
    mut messages: ResMut<ServerMessages>,
    mut clients: Query<(
        Entity,
        &mut Updates,
        &mut Mutations,
        &ConnectedClient,
        &mut ClientTicks,
    )>,
) -> Result<()> {
    let mut server_tick_range = None;
    for (client, updates, mut mutations, connected, mut ticks) in &mut clients {
        if !updates.is_empty() {
            ticks.update_tick = **server_tick;
            let server_tick_range =
                serialized.write_cached_tick(&mut server_tick_range, **server_tick)?;

            updates.send(&mut messages, client, &serialized, server_tick_range)?;
        }

        if !mutations.is_empty() || **track_mutate_messages {
            let server_tick_range =
                serialized.write_cached_tick(&mut server_tick_range, **server_tick)?;

            mutations.send(
                &mut messages,
                client,
                &mut ticks,
                &mut split_buffer,
                &serialized,
                **track_mutate_messages,
                server_tick_range,
                **server_tick,
                **change_tick,
                time.elapsed(),
                connected.max_size,
            )?;
        }
    }

    serialized.clear();

    Ok(())
}

// The storage resource may be unavailable while receiving replication, and the
// client may have marked `Replicated` as a required component for `Remote`.
// Cleanup is handled manually in the receive logic.
fn cleanup_storage(remove: On<Remove, Replicated>, mut storage: If<ResMut<ReplicationStorage>>) {
    storage.entities.remove(&remove.entity);
}

fn reset(
    mut commands: Commands,
    mut messages: ResMut<ServerMessages>,
    mut server_tick: ResMut<ServerTick>,
    mut related_entities: ResMut<RelatedEntities>,
    clients: Query<Entity, With<ConnectedClient>>,
    mut message_buffer: ResMut<MessageBuffer>,
) {
    messages.clear();
    *server_tick = Default::default();
    message_buffer.clear();
    related_entities.clear();
    for client in &clients {
        commands.entity(client).despawn();
    }
}

/// Set with replication and event systems related to server.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ServerSystems {
    /// Systems that receive packets from the messaging backend and update [`ServerState`].
    ///
    /// Used by the messaging backend.
    ///
    /// Runs in [`PreUpdate`].
    ReceivePackets,
    /// Systems that read data from [`ServerMessages`].
    ///
    /// Runs in [`PreUpdate`].
    Receive,
    /// Systems that build the initial graph with all related entities registered via
    /// [`SyncRelatedAppExt::sync_related_entities`].
    ///
    /// The graph is kept in sync with observers.
    ///
    /// Runs in [`OnEnter`] for [`ServerState::Running`].
    ReadRelations,
    /// System that increments [`ServerTick`].
    ///
    /// Runs in [`ServerPlugin::tick_schedule`].
    IncrementTick,
    /// Systems that write data to [`ServerMessages`].
    ///
    /// Runs in [`PostUpdate`] if [`ServerTick`] changes.
    Send,
    /// Systems that send packets to the messaging backend.
    ///
    /// Used by the messaging backend.
    ///
    /// Runs in [`PostUpdate`] if [`ServerTick`] changes.
    SendPackets,
}

/// System tick used for change detection as the current tick.
///
/// Used to share the same tick in [`collect_changes`] and [`send_messages`].
#[derive(Resource, Deref, DerefMut, Default)]
struct ServerChangeTick(Tick);

/// Buffer with all despawned entities.
///
/// We treat removals of [`Replicated`] component as despawns
/// to avoid missing events in case the server's tick policy is
/// not [`TickPolicy::EveryFrame`].
#[derive(Resource, Deref, DerefMut, Default)]
struct DespawnBuffer(Vec<Entity>);

/// Marker that enables replication and all events for a client.
///
/// Until authorization happened, the client and server can still exchange network events that are marked as
/// independent via [`ServerMessageAppExt::make_message_independent`] or [`ServerEventAppExt::make_event_independent`].
/// **All other events will be ignored**.
///
/// See also [`ConnectedClient`] and [`RepliconSharedPlugin::auth_method`].
#[derive(Component, Reflect, Default)]
#[component(immutable)]
#[require(ClientTicks, ClientVisibility, PriorityMap, Updates, Mutations)]
pub struct AuthorizedClient;

/// Controls how often mutations are sent for an authorized client.
///
/// Associates entities with a priority number configurable by the user.
/// If the priority is not set, it defaults to 1.0.
///
/// During replication, we multiply the difference between the last acknowledged tick
/// and [`ServerTick`] by the priority. If the result is greater than or equal to 1.0,
/// we send mutations for this entity.
///
/// This means the priority accumulates across server ticks until an entity is acknowledged,
/// at which point its priority is reset. As a result, even low-priority objects eventually
/// reach a high enough priority to be considered for replication.
///
/// For example, if the base priority is 0.5, mutations for an entity will be sent
/// no more often than once every 2 ticks. With the default priority of 1.0,
/// all unacknowledged mutations will be sent every tick.
///
/// All of this only affects mutations. For any component insertion or removal, the changes
/// will be sent using [`ServerChannel::Updates`](crate::shared::backend::channels::ServerChannel::Updates).
/// See its documentation for more details.
#[derive(Component, Reflect, Deref, DerefMut, Default, Debug, Clone)]
pub struct PriorityMap(EntityHashMap<f32>);
