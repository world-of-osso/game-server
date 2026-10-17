use std::sync::Mutex;

use bevy::ecs::entity::{EntityHashMap, EntityHashSet};
use bevy::platform::collections::HashMap;

use crate::auras::stealth::{DetectionGeometry, Stealth};
use bevy::prelude::*;
use bevy_replicon::server::visibility::client_visibility::ClientVisibility;
use lightyear::prelude::*;
use shared::components::{Level, Npc, Player, Position, Rotation, UnitScale};
use shared::death::{DeathState, ghost_visible};

use crate::maps::InMap;
use crate::networking::PlayerLink;

/// Entities beyond this distance (in yards) from a player are not replicated.
const INTEREST_RADIUS: f32 = 100.0;
/// Squared interest radius for distance comparison without sqrt.
const INTEREST_RADIUS_SQ: f32 = INTEREST_RADIUS * INTEREST_RADIUS;
const INTEREST_BUCKET_SIZE: f32 = INTEREST_RADIUS;

/// A 100 yd square of one map copy.
type InterestBucket = (InMap, i32, i32);

/// TC private object GUID for the source OnlyVisibleToSummoner flag.
#[derive(Component, Clone, Copy)]
pub(crate) struct PrivateObjectOwner(pub Entity);

/// TC private group GUID captured when the summon is created.
#[derive(Component, Clone, Copy)]
pub(crate) struct PrivateObjectGroup(pub crate::group::GroupId);

#[derive(Component, Default)]
pub(crate) struct NetworkVisibility {
    visible_senders: EntityHashSet,
}

impl NetworkVisibility {
    #[cfg(test)]
    pub(crate) fn is_visible(&self, sender: Entity) -> bool {
        self.visible_senders.contains(&sender)
    }

    /// The client connections this entity is replicated to.
    pub(crate) fn visible_senders(&self) -> impl Iterator<Item = Entity> + '_ {
        self.visible_senders.iter().copied()
    }

    pub(crate) fn gain_visibility(&mut self, sender: Entity) -> bool {
        self.visible_senders.insert(sender)
    }

    pub(crate) fn lose_visibility(&mut self, sender: Entity) -> bool {
        self.visible_senders.remove(&sender)
    }
}

#[derive(Default)]
struct InterestDirtyEntities {
    interest: EntityHashSet,
    replication: EntityHashSet,
    /// Units whose position changed since `creature_sight` last read them: TrinityCore
    /// marks one relocation for both visibility and AI (`Map::CreatureRelocation` →
    /// `AddToNotify(NOTIFY_VISIBILITY_CHANGED)`).
    relocated: EntityHashSet,
}

#[derive(Resource, Default)]
pub(crate) struct InterestDirtyQueue(Mutex<InterestDirtyEntities>);

impl InterestDirtyQueue {
    pub(crate) fn mark(&self, entity: Entity) {
        let mut dirty = self.0.lock().expect("interest dirty queue poisoned");
        dirty.interest.insert(entity);
        dirty.replication.insert(entity);
        dirty.relocated.insert(entity);
    }

    /// Units that moved since the last call (AI relocation notifies).
    pub(crate) fn drain_relocated(&self) -> Vec<Entity> {
        self.0
            .lock()
            .expect("interest dirty queue poisoned")
            .relocated
            .drain()
            .collect()
    }

    fn drain_interest(&self) -> Vec<Entity> {
        self.0
            .lock()
            .expect("interest dirty queue poisoned")
            .interest
            .drain()
            .collect()
    }

    fn drain_replication(&self) -> Vec<Entity> {
        self.0
            .lock()
            .expect("interest dirty queue poisoned")
            .replication
            .drain()
            .collect()
    }
}

/// Hide `entity` from every player (or show it again): its visibility is
/// re-evaluated in the next interest update.
pub(crate) fn set_hidden(world: &mut World, entity: Entity, hidden: bool) {
    let Some(mut index) = world.get_resource_mut::<InterestIndex>() else {
        return;
    };
    let changed = if hidden {
        index.hidden.insert(entity)
    } else {
        index.hidden.remove(&entity)
    };
    if changed && index.target_positions.contains_key(&entity) {
        index.dirty_targets.insert(entity);
    }
}

pub(crate) fn set_invisibility(
    world: &mut World,
    entity: Entity,
    state: crate::auras::Invisibility,
) {
    let Some(mut index) = world.get_resource_mut::<InterestIndex>() else {
        return;
    };
    index.invisibility.insert(entity, state);
    index.dirty_targets.insert(entity);
    if let Some(&connection) = index.connection_by_player.get(&entity) {
        index.dirty_observers.insert(connection);
    }
}

pub(crate) fn set_stealth(world: &mut World, entity: Entity, state: Stealth) {
    let Some(mut index) = world.get_resource_mut::<InterestIndex>() else {
        return;
    };
    index.stealth.insert(entity, state);
    index.dirty_targets.insert(entity);
    if let Some(&connection) = index.connection_by_player.get(&entity) {
        index.dirty_observers.insert(connection);
    }
}

pub(crate) fn mark_interest_position_changed(queue: Option<&InterestDirtyQueue>, entity: Entity) {
    if let Some(queue) = queue {
        queue.mark(entity);
    }
}

#[derive(Clone)]
struct InterestObserver {
    player: Entity,
    position: Position,
    /// `None` while the player is in a map transfer: it sees only itself.
    map: Option<InMap>,
    visible: EntityHashSet,
    group: Option<crate::group::GroupId>,
}

impl InterestObserver {
    /// A player always sees itself; others share its map copy within the radius.
    fn sees(&self, entity: Entity, position: &Position, map: Option<InMap>) -> bool {
        entity == self.player
            || (self.map.is_some()
                && map == self.map
                && distance_squared(position, &self.position) <= INTEREST_RADIUS_SQ)
    }
}

#[derive(Resource, Default)]
struct InterestIndex {
    buckets: HashMap<InterestBucket, EntityHashSet>,
    target_buckets: EntityHashMap<InterestBucket>,
    target_positions: EntityHashMap<(Position, Option<InMap>)>,
    observers: EntityHashMap<InterestObserver>,
    connection_by_player: EntityHashMap<Entity>,
    dirty_targets: EntityHashSet,
    dirty_observers: EntityHashSet,
    invisibility: EntityHashMap<crate::auras::Invisibility>,
    stealth: EntityHashMap<Stealth>,
    detection_geometry: EntityHashMap<DetectionGeometry>,
    /// Targets no player sees (decayed corpses).
    hidden: EntityHashSet,
    life_states: EntityHashMap<DeathState>,
    private_owners: EntityHashMap<Entity>,
    private_groups: EntityHashMap<crate::group::GroupId>,
}

impl InterestIndex {
    fn add_target(&mut self, entity: Entity, position: Position, map: Option<InMap>) {
        if let Some(map) = map {
            let bucket = interest_bucket(map, &position);
            self.buckets.entry(bucket).or_default().insert(entity);
            self.target_buckets.insert(entity, bucket);
        }
        self.target_positions.insert(entity, (position, map));
        self.dirty_targets.insert(entity);
    }

    /// Whether `observer` sees target `entity`: in its interest and not hidden.
    fn observer_sees(
        &self,
        observer: &InterestObserver,
        entity: Entity,
        position: &Position,
        map: Option<InMap>,
    ) -> bool {
        observer.sees(entity, position, map)
            && (entity == observer.player
                || (!self.hidden.contains(&entity)
                    && private_object_visible(
                        self.private_owners.get(&entity).copied(),
                        observer.player,
                    )
                    && self
                        .private_groups
                        .get(&entity)
                        .is_none_or(|group| Some(*group) == observer.group)
                    && self
                        .invisibility
                        .get(&observer.player)
                        .cloned()
                        .unwrap_or_default()
                        .detects(
                            &self.invisibility.get(&entity).cloned().unwrap_or_default(),
                            self.detection_geometry
                                .get(&entity)
                                .is_some_and(DetectionGeometry::is_unit),
                        )
                    && self
                        .stealth
                        .get(&observer.player)
                        .cloned()
                        .unwrap_or_default()
                        .detects(
                            &self.stealth.get(&entity).cloned().unwrap_or_default(),
                            self.detection_geometry
                                .get(&observer.player)
                                .copied()
                                .unwrap_or_default(),
                            observer.position,
                            *position,
                        )
                    && self.ghost_visible_to(observer.player, entity)))
    }

    /// Shared policy intentionally governs this realm, not TC's group/corpse exceptions.
    fn ghost_visible_to(&self, observer: Entity, target: Entity) -> bool {
        ghost_visible(
            self.life_states.get(&observer).copied().unwrap_or_default(),
            self.life_states.get(&target).copied().unwrap_or_default(),
            false,
        )
    }

    fn update_life_state(&mut self, entity: Entity, state: DeathState) {
        if self.life_states.insert(entity, state) == Some(state) {
            return;
        }
        self.dirty_targets.insert(entity);
        if let Some(&connection) = self.connection_by_player.get(&entity) {
            self.dirty_observers.insert(connection);
        }
    }

    fn remove_target(&mut self, entity: Entity) {
        self.life_states.remove(&entity);
        self.private_owners.remove(&entity);
        self.private_groups.remove(&entity);
        self.invisibility.remove(&entity);
        self.stealth.remove(&entity);
        self.detection_geometry.remove(&entity);
        if let Some(bucket) = self.target_buckets.remove(&entity) {
            let remove_bucket = self.buckets.get_mut(&bucket).is_some_and(|entities| {
                entities.remove(&entity);
                entities.is_empty()
            });
            if remove_bucket {
                self.buckets.remove(&bucket);
            }
        }
        self.target_positions.remove(&entity);
        self.dirty_targets.remove(&entity);
        for observer in self.observers.values_mut() {
            observer.visible.remove(&entity);
        }
    }

    fn add_observer(
        &mut self,
        connection: Entity,
        player: Entity,
        position: Position,
        map: Option<InMap>,
    ) {
        self.connection_by_player.insert(player, connection);
        self.observers.insert(
            connection,
            InterestObserver {
                player,
                position,
                map,
                visible: EntityHashSet::default(),
                group: None,
            },
        );
        self.dirty_observers.insert(connection);
    }

    fn remove_observer(&mut self, connection: Entity) {
        if let Some(observer) = self.observers.remove(&connection) {
            self.connection_by_player.remove(&observer.player);
        }
        self.dirty_observers.remove(&connection);
    }

    fn update_position(&mut self, entity: Entity, position: Position, map: Option<InMap>) {
        if self.target_positions.contains_key(&entity) {
            self.update_target_position(entity, position, map);
        }
        if let Some(&connection) = self.connection_by_player.get(&entity)
            && let Some(observer) = self.observers.get_mut(&connection)
        {
            observer.position = position;
            observer.map = map;
            self.dirty_observers.insert(connection);
        }
    }

    fn update_target_position(&mut self, entity: Entity, position: Position, map: Option<InMap>) {
        let next_bucket = map.map(|map| interest_bucket(map, &position));
        let previous_bucket = match next_bucket {
            Some(next_bucket) => self.target_buckets.insert(entity, next_bucket),
            None => self.target_buckets.remove(&entity),
        };
        if previous_bucket != next_bucket {
            if let Some(previous_bucket) = previous_bucket {
                let remove_bucket =
                    self.buckets
                        .get_mut(&previous_bucket)
                        .is_some_and(|entities| {
                            entities.remove(&entity);
                            entities.is_empty()
                        });
                if remove_bucket {
                    self.buckets.remove(&previous_bucket);
                }
            }
            if let Some(next_bucket) = next_bucket {
                self.buckets.entry(next_bucket).or_default().insert(entity);
            }
        }
        self.target_positions.insert(entity, (position, map));
        self.dirty_targets.insert(entity);
    }

    fn nearby_targets(&self, map: Option<InMap>, position: &Position) -> EntityHashSet {
        let mut candidates = EntityHashSet::default();
        let Some(map) = map else {
            return candidates;
        };
        let (_, bucket_x, bucket_z) = interest_bucket(map, position);
        for dz in -1..=1 {
            for dx in -1..=1 {
                if let Some(entities) = self.buckets.get(&(map, bucket_x + dx, bucket_z + dz)) {
                    candidates.extend(entities.iter().copied());
                }
            }
        }
        candidates
    }
}

pub struct InterestPlugin;

/// The fixed-tick update of which entities each player sees.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct InterestUpdate;

impl Plugin for InterestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InterestDirtyQueue>()
            .init_resource::<InterestIndex>()
            .add_observer(on_network_visibility_added)
            .add_observer(on_private_object_owner_added)
            .add_observer(on_private_object_group_added)
            .add_observer(on_replication_client_added)
            .add_observer(on_network_visibility_removed)
            .add_observer(on_player_link_added)
            .add_observer(on_player_link_removed)
            .add_observer(refresh_woken_detection_geometry)
            .add_observer(refresh_added_detection_geometry)
            .add_systems(
                FixedUpdate,
                (
                    sync_detection_geometry,
                    sync_observer_groups,
                    update_interest,
                )
                    .chain()
                    .in_set(InterestUpdate),
            )
            .add_systems(
                PostUpdate,
                forward_position_changes_to_replication.before(ReplicationSystems::Send),
            );
    }
}

/// Replicon defaults to visible; no world entities are relevant before selection.
fn on_replication_client_added(
    trigger: On<Add, ClientVisibility>,
    targets: Query<Entity, With<NetworkVisibility>>,
    mut commands: Commands,
) {
    for entity in &targets {
        commands.lose_visibility(entity, trigger.entity);
    }
}

fn on_private_object_owner_added(
    trigger: On<Add, PrivateObjectOwner>,
    owners: Query<&PrivateObjectOwner>,
    mut index: ResMut<InterestIndex>,
) {
    let owner = owners.get(trigger.entity).expect("added private owner");
    index.private_owners.insert(trigger.entity, owner.0);
    index.dirty_targets.insert(trigger.entity);
}

fn on_private_object_group_added(
    trigger: On<Add, PrivateObjectGroup>,
    groups: Query<&PrivateObjectGroup>,
    mut index: ResMut<InterestIndex>,
) {
    let group = groups.get(trigger.entity).expect("added private group");
    index.private_groups.insert(trigger.entity, group.0);
    index.dirty_targets.insert(trigger.entity);
}

fn on_network_visibility_added(
    trigger: On<Add, NetworkVisibility>,
    positions: Query<(
        &Position,
        Option<&InMap>,
        Option<&DeathState>,
        Option<&PrivateObjectOwner>,
        Option<&PrivateObjectGroup>,
    )>,
    connections: Query<Entity, With<ClientVisibility>>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut commands: Commands,
    mut index: ResMut<InterestIndex>,
) {
    if let Ok((position, map, life_state, private_owner, private_group)) =
        positions.get(trigger.entity)
    {
        if let Some(owner) = private_owner {
            index.private_owners.insert(trigger.entity, owner.0);
        }
        if let Some(group) = private_group {
            index.private_groups.insert(trigger.entity, group.0);
        }
        if let Some(state) = life_state {
            index.update_life_state(trigger.entity, *state);
        }
        if map.is_none() {
            error!(
                "{} is replicated without an InMap: no player sees it",
                trigger.entity
            );
        }
        index.add_target(trigger.entity, *position, map.copied());
    }
    for connection in &connections {
        commands.lose_visibility(trigger.entity, connection);
        mark_sender_visibility_changed(replication_dirty.as_deref(), trigger.entity, connection);
    }
}

/// Taking `NetworkVisibility` off an entity hides it from every player that sees it:
/// the entity leaves those clients. Only for entities that stop replicating: one
/// that keeps `Replicate` without `NetworkVisibility` is shown to every client that
/// connects later, so hide those with `set_hidden`. Removing `Replicate` alone
/// marks nothing for the senders, so their copies would stay.
fn on_network_visibility_removed(
    trigger: On<Remove, NetworkVisibility>,
    states: Query<&NetworkVisibility>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut commands: Commands,
    mut index: ResMut<InterestIndex>,
) {
    let entity = trigger.entity;
    if let Ok(state) = states.get(entity) {
        for &sender in &state.visible_senders {
            commands.lose_visibility(entity, sender);
            mark_sender_visibility_changed(replication_dirty.as_deref(), entity, sender);
        }
    }
    index.remove_target(entity);
}

fn on_player_link_added(
    trigger: On<Add, PlayerLink>,
    links: Query<&PlayerLink>,
    positions: Query<(&Position, Option<&InMap>, Option<&DeathState>)>,
    mut index: ResMut<InterestIndex>,
) {
    let Ok(link) = links.get(trigger.entity) else {
        return;
    };
    let Ok((position, map, life_state)) = positions.get(link.0) else {
        return;
    };
    if let Some(state) = life_state {
        index.update_life_state(link.0, *state);
    }
    index.add_observer(trigger.entity, link.0, *position, map.copied());
}

fn on_player_link_removed(
    trigger: On<Remove, PlayerLink>,
    mut visibility: Query<&mut NetworkVisibility>,
    mut index: ResMut<InterestIndex>,
) {
    for mut state in &mut visibility {
        state.lose_visibility(trigger.entity);
    }
    index.remove_observer(trigger.entity);
}

fn forward_position_changes_to_replication(
    dirty_queue: Res<InterestDirtyQueue>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
) {
    let Some(replication_dirty) = replication_dirty else {
        return;
    };
    for entity in dirty_queue.drain_replication() {
        replication_dirty.mark(entity);
    }
}

type DetectionGeometryData = (
    Entity,
    Option<&'static Level>,
    Option<&'static Rotation>,
    Option<&'static UnitScale>,
    Has<Player>,
    Has<Npc>,
    Option<&'static shared::protocol::GameObjectInfo>,
);

type ActiveDetectionGeometry<'w, 's> =
    Query<'w, 's, DetectionGeometryData, Without<crate::grid_activity::GridDormant>>;

fn sync_detection_geometry(units: ActiveDetectionGeometry, mut index: ResMut<InterestIndex>) {
    // Only observer geometry controls stealth range/facing; targets need their unit role.
    // Read current observer components to preserve in-place rotation/level/scale edits.
    let changed: Vec<_> = index
        .observers
        .values()
        .filter_map(|observer| units.get(observer.player).ok())
        .map(|(entity, level, rotation, scale, player, npc, object)| {
            (
                entity,
                DetectionGeometry::from_parts((level, rotation, scale, player, npc, object)),
            )
        })
        .filter(|(entity, geometry)| index.detection_geometry.get(entity) != Some(geometry))
        .collect();
    for (entity, geometry) in changed {
        publish_detection_geometry(&mut index, entity, geometry);
    }
}

fn refresh_added_detection_geometry(
    event: On<
        Add,
        (
            Player,
            Npc,
            shared::protocol::GameObjectInfo,
            NetworkVisibility,
        ),
    >,
    units: ActiveDetectionGeometry,
    mut index: ResMut<InterestIndex>,
) {
    if let Ok((entity, level, rotation, scale, player, npc, object)) = units.get(event.entity) {
        publish_detection_geometry(
            &mut index,
            entity,
            DetectionGeometry::from_parts((level, rotation, scale, player, npc, object)),
        );
    }
}

fn refresh_woken_detection_geometry(
    trigger: On<Remove, crate::grid_activity::GridDormant>,
    units: Query<DetectionGeometryData>,
    mut index: ResMut<InterestIndex>,
) {
    if let Ok((entity, level, rotation, scale, player, npc, object)) = units.get(trigger.entity) {
        publish_detection_geometry(
            &mut index,
            entity,
            DetectionGeometry::from_parts((level, rotation, scale, player, npc, object)),
        );
    }
}

fn publish_detection_geometry(
    index: &mut InterestIndex,
    entity: Entity,
    geometry: DetectionGeometry,
) {
    index.detection_geometry.insert(entity, geometry);
    index.dirty_targets.insert(entity);
    if let Some(&connection) = index.connection_by_player.get(&entity) {
        index.dirty_observers.insert(connection);
    }
}

type VisibilityTargets<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        Option<&'static InMap>,
        &'static mut NetworkVisibility,
    ),
>;

/// Group changes alter privacy even when neither viewer nor target moved.
fn sync_observer_groups(
    groups: Option<Res<crate::group::GroupRegistry>>,
    characters: Query<&crate::persistence::ClientId>,
    mut index: ResMut<InterestIndex>,
) {
    let InterestIndex {
        observers,
        dirty_observers,
        ..
    } = &mut *index;
    for (connection, observer) in observers {
        let group = characters.get(observer.player).ok().and_then(|character| {
            groups
                .as_ref()
                .and_then(|groups| groups.group_id(character.0))
        });
        if observer.group != group {
            observer.group = group;
            dirty_observers.insert(*connection);
        }
    }
}

fn update_interest(
    positions: Query<(&Position, Option<&InMap>)>,
    life_states: Query<(Entity, &DeathState), Changed<DeathState>>,
    dirty_queue: Res<InterestDirtyQueue>,
    replication_dirty: Option<Res<ReplicationDirtyQueue>>,
    mut commands: Commands,
    mut index: ResMut<InterestIndex>,
    mut visibility_query: VisibilityTargets,
) {
    for (entity, state) in &life_states {
        index.update_life_state(entity, *state);
    }
    for entity in dirty_queue.drain_interest() {
        if let Ok((position, map)) = positions.get(entity) {
            index.update_position(entity, *position, map.copied());
        }
    }

    let dirty_observers = index.dirty_observers.drain().collect::<Vec<_>>();
    for connection in dirty_observers {
        update_observer_visibility(
            connection,
            &mut index,
            &mut visibility_query,
            replication_dirty.as_deref(),
            &mut commands,
        );
    }

    let dirty_targets = index.dirty_targets.drain().collect::<Vec<_>>();
    for entity in dirty_targets {
        update_target_visibility(
            entity,
            &mut index,
            &mut visibility_query,
            replication_dirty.as_deref(),
            &mut commands,
        );
    }
}

fn update_observer_visibility(
    connection: Entity,
    index: &mut InterestIndex,
    visibility_query: &mut VisibilityTargets,
    replication_dirty: Option<&ReplicationDirtyQueue>,
    commands: &mut Commands,
) {
    let Some(observer) = index.observers.get(&connection).cloned() else {
        return;
    };
    let mut candidates = index.nearby_targets(observer.map, &observer.position);
    candidates.extend(observer.visible.iter().copied());
    candidates.insert(observer.player);
    let mut visible = EntityHashSet::default();

    for entity in candidates {
        let Ok((position, map, mut state)) = visibility_query.get_mut(entity) else {
            continue;
        };
        if index.observer_sees(&observer, entity, position, map.copied()) {
            visible.insert(entity);
            if state.gain_visibility(connection) {
                commands.gain_visibility(entity, connection);
                mark_sender_visibility_changed(replication_dirty, entity, connection);
            }
        } else if state.lose_visibility(connection) {
            commands.lose_visibility(entity, connection);
            mark_sender_visibility_changed(replication_dirty, entity, connection);
        }
    }

    if let Some(current) = index.observers.get_mut(&connection) {
        current.visible = visible;
    }
}

fn update_target_visibility(
    entity: Entity,
    index: &mut InterestIndex,
    visibility_query: &mut VisibilityTargets,
    replication_dirty: Option<&ReplicationDirtyQueue>,
    commands: &mut Commands,
) {
    let Some((position, map)) = index.target_positions.get(&entity).copied() else {
        return;
    };
    let Ok((_position, _map, mut state)) = visibility_query.get_mut(entity) else {
        index.dirty_targets.insert(entity);
        return;
    };

    let visibility: Vec<_> = index
        .observers
        .iter()
        .map(|(&connection, observer)| {
            (
                connection,
                index.observer_sees(observer, entity, &position, map),
            )
        })
        .collect();
    for (connection, should_be_visible) in visibility {
        let observer = index
            .observers
            .get_mut(&connection)
            .expect("indexed observer");
        apply_target_visibility_transition(
            &mut state,
            observer,
            entity,
            connection,
            should_be_visible,
            replication_dirty,
            commands,
        );
    }
}

fn private_object_visible(owner: Option<Entity>, observer: Entity) -> bool {
    owner.is_none_or(|owner| owner == observer)
}

fn apply_target_visibility_transition(
    state: &mut NetworkVisibility,
    observer: &mut InterestObserver,
    entity: Entity,
    connection: Entity,
    should_be_visible: bool,
    replication_dirty: Option<&ReplicationDirtyQueue>,
    commands: &mut Commands,
) {
    if should_be_visible {
        if state.gain_visibility(connection) {
            commands.gain_visibility(entity, connection);
            mark_sender_visibility_changed(replication_dirty, entity, connection);
        }
        observer.visible.insert(entity);
        return;
    }
    if state.lose_visibility(connection) {
        commands.lose_visibility(entity, connection);
        mark_sender_visibility_changed(replication_dirty, entity, connection);
    }
    observer.visible.remove(&entity);
}

fn mark_sender_visibility_changed(
    replication_dirty: Option<&ReplicationDirtyQueue>,
    entity: Entity,
    connection: Entity,
) {
    if let Some(replication_dirty) = replication_dirty {
        replication_dirty.mark_for_sender(entity, connection);
    }
}

fn interest_bucket(map: InMap, position: &Position) -> InterestBucket {
    (
        map,
        (position.x / INTEREST_BUCKET_SIZE).floor() as i32,
        (position.z / INTEREST_BUCKET_SIZE).floor() as i32,
    )
}

/// Update visibility of a single entity for all players.
#[cfg(test)]
fn update_entity_visibility(
    entity_pos: &Position,
    state: &mut NetworkVisibility,
    players: &[(Entity, Position)],
) {
    for &(conn_entity, player_pos) in players {
        let dist_sq = distance_squared(entity_pos, &player_pos);
        // Player's own entity (dist ≈ 0) is always visible.
        if dist_sq < 0.001 || dist_sq <= INTEREST_RADIUS_SQ {
            state.gain_visibility(conn_entity);
        } else {
            state.lose_visibility(conn_entity);
        }
    }
}

#[cfg(test)]
/// Check if two entities share a phase (bitmask overlap).
///
/// WoW phases are bitmasks: entity is visible if `entity_phase & player_phase != 0`.
/// Phase 0 is treated as "all phases" (always visible).
fn phases_overlap(entity_phase: u32, player_phase: u32) -> bool {
    entity_phase == 0 || player_phase == 0 || (entity_phase & player_phase) != 0
}

#[cfg(test)]
/// Check if entities in different instances should be invisible to each other.
///
/// Entities must share the same instance_id to be visible. Instance 0 = open world.
fn same_instance(entity_instance: u32, player_instance: u32) -> bool {
    entity_instance == player_instance
}

#[cfg(test)]
/// Check if an entity should be visible considering distance and stealth.
///
/// Stealthed entities are hidden unless the observer is within detection range
/// (stealth detection range is much shorter than normal visibility).
fn should_be_visible(dist_sq: f32, stealthed: bool) -> bool {
    const STEALTH_DETECT_RANGE_SQ: f32 = 10.0 * 10.0; // 10yd detection range
    if stealthed {
        dist_sq <= STEALTH_DETECT_RANGE_SQ
    } else {
        dist_sq < 0.001 || dist_sq <= INTEREST_RADIUS_SQ
    }
}

/// Squared distance between two positions (avoids sqrt).
fn distance_squared(a: &Position, b: &Position) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
#[path = "interest_tests.rs"]
mod tests;

#[cfg(test)]
#[test]
fn idle_detection_full_population_is_bounded() {
    let mut app = App::new();
    app.init_resource::<InterestIndex>();
    app.add_systems(Update, sync_detection_geometry);
    crate::test_data::populate_idle_world(app.world_mut(), || {
        (
            Level(30),
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            UnitScale(1.0),
            Npc {
                template_id: 30,
                name: String::new(),
            },
        )
    });
    crate::test_data::assert_idle_updates_are_bounded(
        &mut app,
        "idle_detection_full_population_is_bounded",
    );
}
