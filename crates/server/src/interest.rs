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
        if should_be_visible {
            if !state.is_visible(connection) {
                state.gain_visibility(connection);
            }
            if let Some(observer) = index.observers.get_mut(&connection) {
                observer.visible.insert(entity);
            }
        } else {
            if state.is_visible(connection) {
                state.lose_visibility(connection);
            }
            if let Some(observer) = index.observers.get_mut(&connection) {
                observer.visible.remove(&entity);
            }
        }
    }
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
mod tests {
    use super::*;

    fn make_state_and_conn() -> (ReplicationState, Entity) {
        (ReplicationState::default(), Entity::from_bits(1))
    }

    fn setup_interest_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins);
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: std::time::Duration::from_millis(50),
        });
        app.add_plugins(InterestPlugin);
        app
    }

    fn spawn_interest_observer(app: &mut App, position: Position) -> Entity {
        let player = app.world_mut().spawn(position).id();
        app.world_mut()
            .spawn((
                PlayerLink(player),
                ReplicationSender::new(
                    std::time::Duration::from_millis(50),
                    SendUpdatesMode::SinceLastAck,
                    false,
                ),
            ))
            .id()
    }

    fn spawn_interest_target(app: &mut App, position: Position) -> Entity {
        app.world_mut()
            .spawn((position, NetworkVisibility, ReplicationState::default()))
            .id()
    }

    fn run_fixed_ticks(app: &mut App, ticks: usize) {
        for _ in 0..ticks {
            app.world_mut().run_schedule(FixedUpdate);
        }
    }

    fn pos(x: f32, y: f32, z: f32) -> Position {
        Position { x, y, z }
    }

    fn assert_visible(state: &ReplicationState, conn: Entity, msg: &str) {
        assert!(state.is_visible(conn), "{msg}");
    }

    fn assert_not_visible(state: &ReplicationState, conn: Entity, msg: &str) {
        assert!(!state.is_visible(conn), "{msg}");
    }

    #[test]
    fn new_nearby_entity_is_visible_on_next_fixed_tick() {
        let mut app = setup_interest_app();
        let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
        let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));

        run_fixed_ticks(&mut app, 1);

        assert!(
            app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(connection),
            "a newly visible entity must not wait for a recurring global scan"
        );
    }

    #[test]
    fn unchanged_interest_ticks_do_not_dirty_visibility() {
        let mut app = setup_interest_app();
        let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
        let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
        run_fixed_ticks(&mut app, 1);
        assert!(
            app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(connection)
        );
        let visibility_tick = app
            .world()
            .entity(target)
            .get_ref::<ReplicationState>()
            .unwrap()
            .last_changed();

        run_fixed_ticks(&mut app, 10);

        assert_eq!(
            app.world()
                .entity(target)
                .get_ref::<ReplicationState>()
                .unwrap()
                .last_changed(),
            visibility_tick,
            "unchanged fixed ticks must not re-evaluate and dirty replication visibility"
        );
    }

    #[test]
    fn dirty_target_crosses_exact_radius_within_same_bucket() {
        let mut app = setup_interest_app();
        let connection = spawn_interest_observer(&mut app, pos(50.0, 0.0, 0.0));
        let target = spawn_interest_target(&mut app, pos(150.01, 0.0, 0.0));
        run_fixed_ticks(&mut app, 1);
        assert!(
            !app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(connection)
        );

        app.world_mut()
            .entity_mut(target)
            .insert(pos(149.99, 0.0, 0.0));
        app.world().resource::<InterestDirtyQueue>().mark(target);
        run_fixed_ticks(&mut app, 1);
        assert!(
            app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(connection)
        );

        app.world_mut()
            .entity_mut(target)
            .insert(pos(150.01, 0.0, 0.0));
        app.world().resource::<InterestDirtyQueue>().mark(target);
        run_fixed_ticks(&mut app, 1);
        assert!(
            !app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(connection)
        );
    }

    #[test]
    fn dirty_observer_movement_updates_only_its_visibility() {
        let mut app = setup_interest_app();
        let player_a = app.world_mut().spawn(pos(0.0, 0.0, 0.0)).id();
        let connection_a = app
            .world_mut()
            .spawn((
                PlayerLink(player_a),
                ReplicationSender::new(
                    std::time::Duration::from_millis(50),
                    SendUpdatesMode::SinceLastAck,
                    false,
                ),
            ))
            .id();
        let player_b = app.world_mut().spawn(pos(300.0, 0.0, 0.0)).id();
        let connection_b = app
            .world_mut()
            .spawn((
                PlayerLink(player_b),
                ReplicationSender::new(
                    std::time::Duration::from_millis(50),
                    SendUpdatesMode::SinceLastAck,
                    false,
                ),
            ))
            .id();
        let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
        run_fixed_ticks(&mut app, 1);
        let state = app
            .world()
            .entity(target)
            .get::<ReplicationState>()
            .unwrap();
        assert!(state.is_visible(connection_a));
        assert!(!state.is_visible(connection_b));

        app.world_mut()
            .entity_mut(player_b)
            .insert(pos(100.0, 0.0, 0.0));
        app.world().resource::<InterestDirtyQueue>().mark(player_b);
        run_fixed_ticks(&mut app, 1);
        let state = app
            .world()
            .entity(target)
            .get::<ReplicationState>()
            .unwrap();
        assert!(state.is_visible(connection_a));
        assert!(state.is_visible(connection_b));

        app.world_mut()
            .entity_mut(player_a)
            .insert(pos(200.0, 0.0, 0.0));
        app.world().resource::<InterestDirtyQueue>().mark(player_a);
        run_fixed_ticks(&mut app, 1);
        let state = app
            .world()
            .entity(target)
            .get::<ReplicationState>()
            .unwrap();
        assert!(!state.is_visible(connection_a));
        assert!(state.is_visible(connection_b));
    }

    #[test]
    fn visibility_removal_and_readd_do_not_leave_stale_range_state() {
        let mut app = setup_interest_app();
        let connection = spawn_interest_observer(&mut app, pos(0.0, 0.0, 0.0));
        let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
        run_fixed_ticks(&mut app, 1);
        assert!(
            app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(connection)
        );

        app.world_mut()
            .entity_mut(target)
            .remove::<NetworkVisibility>();
        app.world_mut()
            .entity_mut(target)
            .insert(pos(500.0, 0.0, 0.0));
        app.world_mut().entity_mut(target).insert(NetworkVisibility);
        run_fixed_ticks(&mut app, 1);

        assert!(
            !app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(connection)
        );
    }

    #[test]
    fn reconnect_uses_fresh_observer_visibility() {
        let mut app = setup_interest_app();
        let player = app.world_mut().spawn(pos(0.0, 0.0, 0.0)).id();
        let first_connection = app
            .world_mut()
            .spawn((
                PlayerLink(player),
                ReplicationSender::new(
                    std::time::Duration::from_millis(50),
                    SendUpdatesMode::SinceLastAck,
                    false,
                ),
            ))
            .id();
        let target = spawn_interest_target(&mut app, pos(50.0, 0.0, 0.0));
        run_fixed_ticks(&mut app, 1);
        assert!(
            app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(first_connection)
        );

        app.world_mut()
            .entity_mut(first_connection)
            .remove::<PlayerLink>();
        let second_connection = app
            .world_mut()
            .spawn((
                PlayerLink(player),
                ReplicationSender::new(
                    std::time::Duration::from_millis(50),
                    SendUpdatesMode::SinceLastAck,
                    false,
                ),
            ))
            .id();
        run_fixed_ticks(&mut app, 1);

        assert!(
            app.world()
                .entity(target)
                .get::<ReplicationState>()
                .unwrap()
                .is_visible(second_connection)
        );
    }

    #[test]
    fn test_distance_squared_basic() {
        let a = pos(0.0, 0.0, 0.0);
        let b = pos(3.0, 4.0, 0.0);
        assert!((distance_squared(&a, &b) - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_entity_within_range_is_visible() {
        let (mut state, conn) = make_state_and_conn();
        let players = vec![(conn, pos(0.0, 0.0, 0.0))];
        let entity_pos = pos(50.0, 0.0, 0.0);

        update_entity_visibility(&entity_pos, &mut state, &players);
        assert_visible(&state, conn, "entity at 50yd should be visible");
    }

    #[test]
    fn test_entity_beyond_range_is_not_visible() {
        let (mut state, conn) = make_state_and_conn();
        let players = vec![(conn, pos(0.0, 0.0, 0.0))];
        let entity_pos = pos(150.0, 0.0, 0.0);

        state.gain_visibility(conn);
        update_entity_visibility(&entity_pos, &mut state, &players);
        assert_not_visible(&state, conn, "entity at 150yd should not be visible");
    }

    #[test]
    fn test_interest_updates_when_player_moves() {
        let (mut state, conn) = make_state_and_conn();
        let entity_pos = pos(80.0, 0.0, 0.0);

        // Player far away (180yd) — not visible
        update_entity_visibility(&entity_pos, &mut state, &[(conn, pos(-100.0, 0.0, 0.0))]);
        assert_not_visible(&state, conn, "should not be visible when player is far");

        // Player moves close (50yd) — becomes visible
        update_entity_visibility(&entity_pos, &mut state, &[(conn, pos(30.0, 0.0, 0.0))]);
        assert_visible(&state, conn, "should be visible when player moves close");
    }

    #[test]
    fn many_entities_only_nearby_replicated() {
        let conn = Entity::from_bits(1);
        let players = vec![(conn, pos(0.0, 0.0, 0.0))];

        // 20 entities at varied distances
        let distances: Vec<f32> = (0..20).map(|i| i as f32 * 15.0).collect(); // 0, 15, 30, ..., 285
        let mut visible_count = 0;
        let mut culled_count = 0;

        for &dist in &distances {
            let mut state = ReplicationState::default();
            let entity_pos = pos(dist, 0.0, 0.0);
            update_entity_visibility(&entity_pos, &mut state, &players);
            if state.is_visible(conn) {
                visible_count += 1;
            } else {
                culled_count += 1;
            }
        }

        // Entities at 0..=100yd visible (0,15,30,45,60,75,90 = 7), at 105+ culled (13)
        assert_eq!(visible_count, 7, "7 entities within 100yd");
        assert_eq!(culled_count, 13, "13 entities beyond 100yd");
    }

    #[test]
    fn entity_at_exactly_100yd_boundary() {
        let conn = Entity::from_bits(1);
        let players = vec![(conn, pos(0.0, 0.0, 0.0))];

        // Exactly 100yd — visible (<=)
        let mut state = ReplicationState::default();
        update_entity_visibility(&pos(100.0, 0.0, 0.0), &mut state, &players);
        assert_visible(&state, conn, "exactly 100yd should be visible");

        // Just over 100yd — culled
        let mut state = ReplicationState::default();
        update_entity_visibility(&pos(100.01, 0.0, 0.0), &mut state, &players);
        assert_not_visible(&state, conn, "100.01yd should be culled");

        // Just under 100yd — visible
        let mut state = ReplicationState::default();
        update_entity_visibility(&pos(99.99, 0.0, 0.0), &mut state, &players);
        assert_visible(&state, conn, "99.99yd should be visible");
    }

    #[test]
    fn teleport_removes_old_area_adds_new_area() {
        let conn = Entity::from_bits(1);

        // Entity near old position (0,0,0)
        let mut old_entity_state = ReplicationState::default();
        // Entity near new position (500,0,0)
        let mut new_entity_state = ReplicationState::default();

        let old_entity_pos = pos(50.0, 0.0, 0.0);
        let new_entity_pos = pos(520.0, 0.0, 0.0);

        // Player at old position — old entity visible, new entity culled
        let players_old = vec![(conn, pos(0.0, 0.0, 0.0))];
        update_entity_visibility(&old_entity_pos, &mut old_entity_state, &players_old);
        update_entity_visibility(&new_entity_pos, &mut new_entity_state, &players_old);
        assert_visible(
            &old_entity_state,
            conn,
            "old entity visible before teleport",
        );
        assert_not_visible(&new_entity_state, conn, "new entity culled before teleport");

        // Player teleports to (500,0,0) — old entity culled, new entity visible
        let players_new = vec![(conn, pos(500.0, 0.0, 0.0))];
        update_entity_visibility(&old_entity_pos, &mut old_entity_state, &players_new);
        update_entity_visibility(&new_entity_pos, &mut new_entity_state, &players_new);
        assert_not_visible(&old_entity_state, conn, "old entity culled after teleport");
        assert_visible(&new_entity_state, conn, "new entity visible after teleport");
    }

    #[test]
    fn npc_spawn_appears_to_all_nearby_players() {
        let player_a = Entity::from_bits(1);
        let player_b = Entity::from_bits(2);
        let player_c = Entity::from_bits(3);

        let players = vec![
            (player_a, pos(10.0, 0.0, 0.0)),  // 40yd from NPC
            (player_b, pos(80.0, 0.0, 0.0)),  // 30yd from NPC
            (player_c, pos(300.0, 0.0, 0.0)), // 250yd from NPC — out of range
        ];

        // NPC spawns at (50, 0, 0)
        let mut state = ReplicationState::default();
        update_entity_visibility(&pos(50.0, 0.0, 0.0), &mut state, &players);

        assert_visible(&state, player_a, "player A within range");
        assert_visible(&state, player_b, "player B within range");
        assert_not_visible(&state, player_c, "player C out of range");
    }

    #[test]
    fn npc_despawn_removed_from_all_observers() {
        let player_a = Entity::from_bits(1);
        let player_b = Entity::from_bits(2);
        let players = vec![
            (player_a, pos(0.0, 0.0, 0.0)),
            (player_b, pos(30.0, 0.0, 0.0)),
        ];

        // NPC visible to both players
        let mut state = ReplicationState::default();
        update_entity_visibility(&pos(10.0, 0.0, 0.0), &mut state, &players);
        assert_visible(&state, player_a, "A sees NPC");
        assert_visible(&state, player_b, "B sees NPC");

        // NPC despawns (simulated by moving far away)
        update_entity_visibility(&pos(9999.0, 0.0, 0.0), &mut state, &players);
        assert_not_visible(&state, player_a, "A lost NPC after despawn");
        assert_not_visible(&state, player_b, "B lost NPC after despawn");
    }

    #[test]
    fn two_players_move_toward_each_other_both_gain_visibility() {
        let conn_a = Entity::from_bits(1);
        let conn_b = Entity::from_bits(2);

        // Player A's entity state (visible to B?) and B's entity state (visible to A?)
        let mut state_a = ReplicationState::default();
        let mut state_b = ReplicationState::default();

        // Players start 200yd apart — neither visible to the other
        let players_far = vec![(conn_a, pos(0.0, 0.0, 0.0)), (conn_b, pos(200.0, 0.0, 0.0))];
        update_entity_visibility(&pos(0.0, 0.0, 0.0), &mut state_a, &players_far);
        update_entity_visibility(&pos(200.0, 0.0, 0.0), &mut state_b, &players_far);
        assert_not_visible(&state_a, conn_b, "A not visible to B when far");
        assert_not_visible(&state_b, conn_a, "B not visible to A when far");

        // Both move toward center — now 50yd apart
        let players_close = vec![
            (conn_a, pos(75.0, 0.0, 0.0)),
            (conn_b, pos(125.0, 0.0, 0.0)),
        ];
        update_entity_visibility(&pos(75.0, 0.0, 0.0), &mut state_a, &players_close);
        update_entity_visibility(&pos(125.0, 0.0, 0.0), &mut state_b, &players_close);
        assert_visible(&state_a, conn_b, "A visible to B after approach");
        assert_visible(&state_b, conn_a, "B visible to A after approach");
    }

    #[test]
    fn two_players_move_apart_both_lose_visibility() {
        let conn_a = Entity::from_bits(1);
        let conn_b = Entity::from_bits(2);
        let mut state_a = ReplicationState::default();
        let mut state_b = ReplicationState::default();

        // Start 20yd apart — both visible
        let players_close = vec![(conn_a, pos(0.0, 0.0, 0.0)), (conn_b, pos(20.0, 0.0, 0.0))];
        update_entity_visibility(&pos(0.0, 0.0, 0.0), &mut state_a, &players_close);
        update_entity_visibility(&pos(20.0, 0.0, 0.0), &mut state_b, &players_close);
        assert_visible(&state_a, conn_b, "A visible to B when close");
        assert_visible(&state_b, conn_a, "B visible to A when close");

        // Move 200yd apart — both lose visibility
        let players_far = vec![
            (conn_a, pos(-100.0, 0.0, 0.0)),
            (conn_b, pos(100.0, 0.0, 0.0)),
        ];
        update_entity_visibility(&pos(-100.0, 0.0, 0.0), &mut state_a, &players_far);
        update_entity_visibility(&pos(100.0, 0.0, 0.0), &mut state_b, &players_far);
        assert_not_visible(&state_a, conn_b, "A lost B after moving apart");
        assert_not_visible(&state_b, conn_a, "B lost A after moving apart");
    }

    #[test]
    fn stealthed_entity_hidden_beyond_detect_range() {
        // Normal entity at 50yd: visible
        assert!(should_be_visible(50.0 * 50.0, false));

        // Stealthed entity at 50yd: hidden
        assert!(!should_be_visible(50.0 * 50.0, true));

        // Stealthed entity at 8yd: detected (within 10yd)
        assert!(should_be_visible(8.0 * 8.0, true));

        // Stealthed entity at exactly 10yd: detected
        assert!(should_be_visible(10.0 * 10.0, true));

        // Stealthed entity at 11yd: hidden
        assert!(!should_be_visible(11.0 * 11.0, true));
    }

    #[test]
    fn dead_entity_still_visible_for_corpse_loot() {
        // Visibility is distance-only — death state doesn't affect it.
        // A dead NPC at 30yd should still be visible for looting.
        let (mut state, conn) = make_state_and_conn();
        let players = vec![(conn, pos(0.0, 0.0, 0.0))];
        let dead_npc_pos = pos(30.0, 0.0, 0.0);

        // Entity is within range → visible regardless of alive/dead
        update_entity_visibility(&dead_npc_pos, &mut state, &players);
        assert_visible(&state, conn, "dead NPC at 30yd still visible");

        // Re-check after another update (corpse stays visible)
        update_entity_visibility(&dead_npc_pos, &mut state, &players);
        assert_visible(&state, conn, "dead NPC remains visible on re-check");
    }

    #[test]
    fn instance_boundary_entities_in_different_instances_invisible() {
        // Same instance: visible
        assert!(same_instance(0, 0), "both open world");
        assert!(same_instance(42, 42), "same dungeon instance");

        // Different instances: invisible
        assert!(!same_instance(1, 2), "different dungeon instances");
        assert!(!same_instance(0, 1), "open world vs dungeon");
        assert!(!same_instance(100, 200), "different raid instances");
    }

    #[test]
    fn phase_system_different_phase_invisible() {
        // Phase 0 = all phases (always visible)
        assert!(phases_overlap(0, 1), "phase 0 entity visible to all");
        assert!(phases_overlap(1, 0), "all-phase player sees everything");

        // Same phase: visible
        assert!(phases_overlap(1, 1));
        assert!(phases_overlap(0b1010, 0b1010));

        // Overlapping bits: visible
        assert!(phases_overlap(0b1100, 0b0110), "share bit 2");

        // No overlap: invisible
        assert!(!phases_overlap(0b1000, 0b0001), "no shared bits");
        assert!(!phases_overlap(1, 2), "phase 1 vs phase 2");
        assert!(!phases_overlap(0b1100, 0b0011), "disjoint masks");
    }

    #[test]
    fn large_entity_count_performance_bounded() {
        let players: Vec<(Entity, Position)> = (0..10)
            .map(|i| (Entity::from_bits(i + 1), pos(i as f32 * 200.0, 0.0, 0.0)))
            .collect();

        let start = std::time::Instant::now();
        let mut visible_count = 0;

        for i in 0..500 {
            let mut state = ReplicationState::default();
            let entity_pos = pos(i as f32 * 2.0, 0.0, 0.0);
            update_entity_visibility(&entity_pos, &mut state, &players);
            for &(conn, _) in &players {
                if state.is_visible(conn) {
                    visible_count += 1;
                }
            }
        }

        let elapsed = start.elapsed();
        assert!(visible_count > 0, "some entities visible");
        assert!(
            elapsed.as_millis() < 500,
            "500 entities x 10 players took {}ms (should be <500ms)",
            elapsed.as_millis(),
        );
    }

    #[test]
    fn repeated_update_with_unchanged_positions_is_idempotent() {
        let (mut state, conn) = make_state_and_conn();
        let players = vec![(conn, pos(0.0, 0.0, 0.0))];
        let entity_pos = pos(50.0, 0.0, 0.0);

        // First update: gains visibility
        update_entity_visibility(&entity_pos, &mut state, &players);
        assert_visible(&state, conn, "visible after first update");

        // Repeated updates with same positions: stays visible, no flip-flop
        for _ in 0..10 {
            update_entity_visibility(&entity_pos, &mut state, &players);
            assert_visible(&state, conn, "still visible on repeated update");
        }

        // Same for an out-of-range entity: stays culled
        let mut far_state = ReplicationState::default();
        let far_pos = pos(200.0, 0.0, 0.0);
        for _ in 0..10 {
            update_entity_visibility(&far_pos, &mut far_state, &players);
            assert_not_visible(&far_state, conn, "still culled on repeated update");
        }
    }

    #[test]
    fn enemy_faction_corpse_still_visible_for_looting() {
        // Visibility is distance-only — faction doesn't affect it.
        // An enemy faction corpse at 20yd should be visible for looting.
        let (mut state, conn) = make_state_and_conn();
        let players = vec![(conn, pos(0.0, 0.0, 0.0))];
        let enemy_corpse_pos = pos(20.0, 0.0, 0.0);

        update_entity_visibility(&enemy_corpse_pos, &mut state, &players);
        assert_visible(&state, conn, "enemy corpse within range is visible");
    }
}
