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
    connections: Query<(Entity, &PlayerLink)>,
    positions: Query<&Position>,
    mut visibility_query: Query<
        (Entity, &Position, &mut ReplicationState),
        With<NetworkVisibility>,
    >,
    mut tick_count: Local<u64>,
) {
    *tick_count += 1;
    if *tick_count % UPDATE_INTERVAL_TICKS != 0 {
        return;
    }

    let players: Vec<(Entity, Position)> = collect_player_positions(&connections, &positions);
    if players.is_empty() {
        return;
    }

    for (_entity, entity_pos, mut state) in visibility_query.iter_mut() {
        update_entity_visibility(entity_pos, &mut state, &players);
    }
}

/// Collect connection entity + player position pairs.
fn collect_player_positions(
    connections: &Query<(Entity, &PlayerLink)>,
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
        // Skip self — player entities shouldn't filter themselves out.
        if dist_sq < 0.001 {
            continue;
        }
        if dist_sq <= INTEREST_RADIUS_SQ {
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
}
