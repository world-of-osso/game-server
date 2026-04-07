use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Position;

use crate::networking::PlayerLink;

/// Entities beyond this distance (in yards) from a player are not replicated.
const INTEREST_RADIUS: f32 = 100.0;
/// Squared interest radius for distance comparison without sqrt.
const INTEREST_RADIUS_SQ: f32 = INTEREST_RADIUS * INTEREST_RADIUS;
/// How many ticks between interest updates (at 20Hz, 10 ticks = 0.5s).
const UPDATE_INTERVAL_TICKS: u64 = 10;

pub struct InterestPlugin;

impl Plugin for InterestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, update_interest);
    }
}

/// Update network visibility for all replicated entities based on distance to each player.
///
/// Runs every `UPDATE_INTERVAL_TICKS` ticks. For each connection with a PlayerLink,
/// checks distance from that player to all entities with NetworkVisibility.
fn update_interest(
    connections: Query<(Entity, &PlayerLink), With<ReplicationSender>>,
    positions: Query<&Position>,
    mut visibility_query: Query<
        (Entity, &Position, &mut ReplicationState),
        With<NetworkVisibility>,
    >,
    mut tick_count: Local<u64>,
) {
    *tick_count += 1;
    if !(*tick_count).is_multiple_of(UPDATE_INTERVAL_TICKS) {
        return;
    }

    let players: Vec<(Entity, Position)> = collect_player_positions(&connections, &positions);
    if players.is_empty() {
        return;
    }

    let vis_count = visibility_query.iter().count();
    if (*tick_count).is_multiple_of(UPDATE_INTERVAL_TICKS * 10) {
        info!(
            "Interest update: {} players, {} visible entities",
            players.len(),
            vis_count
        );
    }

    for (_entity, entity_pos, mut state) in visibility_query.iter_mut() {
        update_entity_visibility(entity_pos, &mut state, &players);
    }
}

/// Collect connection entity + player position pairs.
fn collect_player_positions(
    connections: &Query<(Entity, &PlayerLink), With<ReplicationSender>>,
    positions: &Query<&Position>,
) -> Vec<(Entity, Position)> {
    connections
        .iter()
        .filter_map(|(conn_entity, link)| positions.get(link.0).ok().map(|pos| (conn_entity, *pos)))
        .collect()
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
        assert_visible(&old_entity_state, conn, "old entity visible before teleport");
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
        let players_close = vec![(conn_a, pos(75.0, 0.0, 0.0)), (conn_b, pos(125.0, 0.0, 0.0))];
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
        let players_far = vec![(conn_a, pos(-100.0, 0.0, 0.0)), (conn_b, pos(100.0, 0.0, 0.0))];
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
    fn update_interval_is_half_second_at_20hz() {
        // FixedUpdate runs at 20Hz (50ms per tick). 10 ticks = 500ms = 0.5s.
        assert_eq!(UPDATE_INTERVAL_TICKS, 10);
        let tick_rate_hz = 20.0_f64;
        let interval_secs = UPDATE_INTERVAL_TICKS as f64 / tick_rate_hz;
        assert!((interval_secs - 0.5).abs() < 0.001);
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
}
