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
}
