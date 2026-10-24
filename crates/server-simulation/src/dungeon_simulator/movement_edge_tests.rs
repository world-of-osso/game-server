//! Recorded Azure endpoints on one actual frozen walk edge, not invented geometry.
use super::*;
use serde::Deserialize;

const ROUNDING_YARDS: f32 = 0.01;

#[derive(Deserialize)]
struct RecordedPositions {
    edge_from: u32,
    edge_to: u32,
    healer: [f32; 3],
    owner: [f32; 3],
    sapling: [f32; 3],
    watcher: [f32; 3],
    watcher_native_radius: f32,
}

fn fixture() -> (Space, RecordedPositions) {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/azure-vault-2515.json");
    let graph = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let recorded =
        serde_json::from_str(include_str!("fixtures/azure1_sapling_positions.json")).unwrap();
    (Space(graph), recorded)
}

#[test]
fn recorded_mid_edge_follow_does_not_overshoot_into_the_idle_sapling() {
    let (space, recorded) = fixture();
    let from = Vec3::from_array(recorded.healer);
    let to = Vec3::from_array(recorded.owner);
    let center = Vec3::from_array(recorded.watcher);
    let mut points = space.route(from, to).unwrap();
    points.insert(0, from);
    let mut walk_yards = 0.0;
    for leg in points.windows(2) {
        let direction = leg[1] - leg[0];
        let fraction =
            ((center - leg[0]).dot(direction) / direction.length_squared()).clamp(0.0, 1.0);
        let clearance = center.distance(leg[0] + direction * fraction);
        assert!(
            clearance >= recorded.watcher_native_radius,
            "recorded safe edge section must not overshoot into native29001853's sphere: {clearance} < {}",
            recorded.watcher_native_radius
        );
        walk_yards += direction.length();
    }
    assert!(
        walk_yards <= from.distance(to) + ROUNDING_YARDS,
        "two endpoints on this same frozen edge must not walk past the destination and return"
    );
    assert_eq!(points.last(), Some(&to));
}

#[test]
fn recorded_mid_edge_retarget_consumes_only_the_remaining_frozen_walk_time() {
    let (space, recorded) = fixture();
    let from = Vec3::from_array(recorded.healer);
    let to = Vec3::from_array(recorded.sapling);
    let edge = space
        .0
        .edges
        .iter()
        .find(|edge| edge.from == recorded.edge_from && edge.to == recorded.edge_to)
        .unwrap();
    let node = |id| {
        space
            .0
            .packs
            .iter()
            .find(|pack| pack.id == id)
            .unwrap()
            .point()
    };
    let full_chord = node(edge.from).distance(node(edge.to));
    let remaining_yards = from.distance(to) / full_chord * edge.yards;
    let mut travel = Travel {
        destination: to,
        legs: read_legs(Some(&space), from, to).unwrap(),
    };
    let after = advance(from, &mut travel, remaining_yards + ROUNDING_YARDS, 0.0);
    assert!(
        after.distance(to) <= ROUNDING_YARDS,
        "recorded partial-edge retarget must arrive after its remaining native walk budget, not restart the full edge: {from:?} -> {after:?}, target {to:?}, budget {remaining_yards}"
    );
    assert!(travel.legs.is_empty());
}
