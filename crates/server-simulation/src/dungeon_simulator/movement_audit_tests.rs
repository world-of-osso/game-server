//! Compare the actual zero-time mover route with an independent native query.
use super::*;
use serde_json::json;

#[test]
#[ignore = "explicit exhaustive populated-pack movement audit"]
fn populated_pack_movement_matches_detour() {
    let graph = super::super::graph::read_graph().expect("audit graph");
    let nav_root = std::env::var_os("DUNGEON_SIMULATOR_NAVMESH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| super::super::graph::extraction::asset_root().join("navmesh"));
    let mut reference =
        navigation::NavigationMap::open(&nav_root.join(graph.map_id.to_string())).unwrap();
    let mut world = World::new();
    world.insert_resource(NativeMovement::open(&nav_root, graph.map_id).unwrap());
    let actor = world
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let packs: Vec<_> = graph
        .packs
        .iter()
        .filter(|pack| !pack.spawns.is_empty())
        .collect();
    let mut compared = 0_u64;
    let mut both_unreachable = 0_u64;
    let mut same_position = 0_u64;
    let mut mismatches = Vec::new();
    let mut max_error = 0.0_f32;
    for a in &packs {
        for b in &packs {
            if a.id == b.id {
                continue;
            }
            let from = a.point();
            let to = b.point();
            if from == to {
                same_position += 1;
                continue;
            }
            world.entity_mut(actor).remove::<(Travel, BlockedWalk)>();
            world.entity_mut(actor).insert((
                Position {
                    x: from.x,
                    y: from.y,
                    z: from.z,
                },
                MoveIntent(to),
            ));
            let actual = move_one(&mut world, actor, 0.0);
            let expected = reference.complete_walk_path(from, to);
            match (actual, expected) {
                (Ok(()), Ok(points)) => {
                    compared += 1;
                    let mut previous = from;
                    let expected_yards: f32 = points.iter().copied().map(|point| {
                        let distance = previous.distance(point);
                        previous = point;
                        distance
                    }).sum();
                    let current = point(world.get::<Position>(actor).unwrap());
                    let travel = world.get::<Travel>(actor).unwrap();
                    let actual_yards: f32 = from.distance(current) + travel.legs.iter().map(|(_, distance)| distance).sum::<f32>();
                    let error = (actual_yards - expected_yards).abs();
                    max_error = max_error.max(error);
                    let skip_first = usize::from(points.first().is_some_and(|point| from.distance(*point) <= 0.0001));
                    let expected_corners = &points[skip_first..];
                    let corners_match = expected_corners.len() == travel.legs.len()
                        && expected_corners.iter().zip(&travel.legs).all(|(expected, (actual, _))| expected.distance(*actual) < 0.0001);
                    if error > 0.01 || !corners_match {
                        mismatches.push(json!({"from": a.id, "to": b.id, "mover_yards": actual_yards, "detour_yards": expected_yards, "corners_match": corners_match}));
                    }
                }
                (Err(_), Err(_)) => both_unreachable += 1,
                (actual, expected) => mismatches.push(json!({"from": a.id, "to": b.id, "mover": format!("{actual:?}"), "detour": format!("{expected:?}")})),
            }
        }
    }
    let summary = json!({"map": graph.map_id, "scope": "ordered populated-pack pairs", "packs": packs.len(), "compared": compared, "both_unreachable": both_unreachable, "same_position": same_position, "max_abs_error_yards": max_error, "mismatches": mismatches});
    let output = std::env::var_os("MOVEMENT_AUDIT_OUTPUT").expect("audit output path");
    std::fs::write(output, serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
    assert_eq!(
        summary["mismatches"].as_array().unwrap().len(),
        0,
        "{summary}"
    );
}
