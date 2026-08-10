use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Position;

use crate::networking::PlayerLink;

/// Entities beyond this distance (in yards) from a player are not replicated.
const INTEREST_RADIUS: f32 = 100.0;
/// Squared interest radius for distance comparison without sqrt.
const INTEREST_RADIUS_SQ: f32 = INTEREST_RADIUS * INTEREST_RADIUS;
const INTEREST_BUCKET_SIZE: f32 = INTEREST_RADIUS;

type InterestBucket = (i32, i32);

#[derive(Resource, Default)]
pub(crate) struct InterestDirtyQueue(Mutex<HashSet<Entity>>);

impl InterestDirtyQueue {
    pub(crate) fn mark(&self, entity: Entity) {
        self.0
            .lock()
            .expect("interest dirty queue poisoned")
            .insert(entity);
    }

    fn drain(&self) -> Vec<Entity> {
        self.0
            .lock()
            .expect("interest dirty queue poisoned")
            .drain()
            .collect()
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
    visible: HashSet<Entity>,
}

#[derive(Resource, Default)]
struct InterestIndex {
    buckets: HashMap<InterestBucket, HashSet<Entity>>,
    target_buckets: HashMap<Entity, InterestBucket>,
    target_positions: HashMap<Entity, Position>,
    observers: HashMap<Entity, InterestObserver>,
    connection_by_player: HashMap<Entity, Entity>,
    dirty_targets: HashSet<Entity>,
    dirty_observers: HashSet<Entity>,
}

impl InterestIndex {
    fn add_target(&mut self, entity: Entity, position: Position) {
        let bucket = interest_bucket(&position);
        self.buckets.entry(bucket).or_default().insert(entity);
        self.target_buckets.insert(entity, bucket);
        self.target_positions.insert(entity, position);
        self.dirty_targets.insert(entity);
    }

    fn remove_target(&mut self, entity: Entity) {
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

    fn add_observer(&mut self, connection: Entity, player: Entity, position: Position) {
        self.connection_by_player.insert(player, connection);
        self.observers.insert(
            connection,
            InterestObserver {
                player,
                position,
                visible: HashSet::new(),
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

    fn update_position(&mut self, entity: Entity, position: Position) {
        if self.target_positions.contains_key(&entity) {
            self.update_target_position(entity, position);
        }
        if let Some(&connection) = self.connection_by_player.get(&entity) {
            if let Some(observer) = self.observers.get_mut(&connection) {
                observer.position = position;
                self.dirty_observers.insert(connection);
            }
        }
    }

    fn update_target_position(&mut self, entity: Entity, position: Position) {
        let next_bucket = interest_bucket(&position);
        let previous_bucket = self.target_buckets.insert(entity, next_bucket);
        if previous_bucket != Some(next_bucket) {
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
            self.buckets.entry(next_bucket).or_default().insert(entity);
        }
        self.target_positions.insert(entity, position);
        self.dirty_targets.insert(entity);
    }

    fn nearby_targets(&self, position: &Position) -> HashSet<Entity> {
        let (bucket_x, bucket_z) = interest_bucket(position);
        let mut candidates = HashSet::new();
        for dz in -1..=1 {
            for dx in -1..=1 {
                if let Some(entities) = self.buckets.get(&(bucket_x + dx, bucket_z + dz)) {
                    candidates.extend(entities.iter().copied());
                }
            }
        }
        candidates
    }
}

pub struct InterestPlugin;

impl Plugin for InterestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InterestDirtyQueue>()
            .init_resource::<InterestIndex>()
            .add_observer(on_network_visibility_added)
            .add_observer(on_network_visibility_removed)
            .add_observer(on_player_link_added)
            .add_observer(on_player_link_removed)
            .add_systems(FixedUpdate, update_interest);
    }
}

fn on_network_visibility_added(
    trigger: On<Add, NetworkVisibility>,
    positions: Query<&Position>,
    mut index: ResMut<InterestIndex>,
) {
    if let Ok(position) = positions.get(trigger.entity) {
        index.add_target(trigger.entity, *position);
    }
}

fn on_network_visibility_removed(
    trigger: On<Remove, NetworkVisibility>,
    mut index: ResMut<InterestIndex>,
) {
    index.remove_target(trigger.entity);
}

fn on_player_link_added(
    trigger: On<Add, PlayerLink>,
    links: Query<&PlayerLink>,
    positions: Query<&Position>,
    mut index: ResMut<InterestIndex>,
) {
    let Ok(link) = links.get(trigger.entity) else {
        return;
    };
    let Ok(position) = positions.get(link.0) else {
        return;
    };
    index.add_observer(trigger.entity, link.0, *position);
}

fn on_player_link_removed(trigger: On<Remove, PlayerLink>, mut index: ResMut<InterestIndex>) {
    index.remove_observer(trigger.entity);
}

fn update_interest(
    positions: Query<&Position>,
    dirty_queue: Res<InterestDirtyQueue>,
    mut index: ResMut<InterestIndex>,
    mut visibility_query: Query<(&Position, &mut ReplicationState), With<NetworkVisibility>>,
) {
    for entity in dirty_queue.drain() {
        if let Ok(position) = positions.get(entity) {
            index.update_position(entity, *position);
        }
    }

    let dirty_observers = index.dirty_observers.drain().collect::<Vec<_>>();
    for connection in dirty_observers {
        update_observer_visibility(connection, &mut index, &mut visibility_query);
    }

    let dirty_targets = index.dirty_targets.drain().collect::<Vec<_>>();
    for entity in dirty_targets {
        update_target_visibility(entity, &mut index, &mut visibility_query);
    }
}

fn update_observer_visibility(
    connection: Entity,
    index: &mut InterestIndex,
    visibility_query: &mut Query<(&Position, &mut ReplicationState), With<NetworkVisibility>>,
) {
    let Some(observer) = index.observers.get(&connection).cloned() else {
        return;
    };
    let mut candidates = index.nearby_targets(&observer.position);
    candidates.extend(observer.visible.iter().copied());
    let mut visible = HashSet::new();

    for entity in candidates {
        let Ok((position, mut state)) = visibility_query.get_mut(entity) else {
            continue;
        };
        let should_be_visible =
            distance_squared(position, &observer.position) <= INTEREST_RADIUS_SQ;
        if should_be_visible {
            visible.insert(entity);
            if !state.is_visible(connection) {
                state.gain_visibility(connection);
            }
        } else if state.is_visible(connection) {
            state.lose_visibility(connection);
        }
    }

    if let Some(current) = index.observers.get_mut(&connection) {
        current.visible = visible;
    }
}

fn update_target_visibility(
    entity: Entity,
    index: &mut InterestIndex,
    visibility_query: &mut Query<(&Position, &mut ReplicationState), With<NetworkVisibility>>,
) {
    let Some(position) = index.target_positions.get(&entity).copied() else {
        return;
    };
    let observers = index
        .observers
        .iter()
        .map(|(&connection, observer)| (connection, observer.position))
        .collect::<Vec<_>>();
    let Ok((_position, mut state)) = visibility_query.get_mut(entity) else {
        index.dirty_targets.insert(entity);
        return;
    };

    for (connection, observer_position) in observers {
        let should_be_visible =
            distance_squared(&position, &observer_position) <= INTEREST_RADIUS_SQ;
        let Some(observer) = index.observers.get_mut(&connection) else {
            continue;
        };
        apply_target_visibility_transition(
            &mut state,
            observer,
            entity,
            connection,
            should_be_visible,
        );
    }
}

fn apply_target_visibility_transition(
    state: &mut ReplicationState,
    observer: &mut InterestObserver,
    entity: Entity,
    connection: Entity,
    should_be_visible: bool,
) {
    if should_be_visible {
        if !state.is_visible(connection) {
            state.gain_visibility(connection);
        }
        observer.visible.insert(entity);
        return;
    }
    if state.is_visible(connection) {
        state.lose_visibility(connection);
    }
    observer.visible.remove(&entity);
}

fn interest_bucket(position: &Position) -> InterestBucket {
    (
        (position.x / INTEREST_BUCKET_SIZE).floor() as i32,
        (position.z / INTEREST_BUCKET_SIZE).floor() as i32,
    )
}

/// Update visibility of a single entity for all players.
fn update_entity_visibility(
    entity_pos: &Position,
    state: &mut ReplicationState,
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
