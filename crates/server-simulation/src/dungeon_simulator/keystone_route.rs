//! Add the least-cost authored force packs to the retained boss route.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet};

use super::graph::{Graph, RouteStep};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(super) struct PlannedRoute {
    pub steps: Vec<RouteStep>,
    pub added_packs: Vec<u32>,
    pub planned_forces: u64,
}

#[derive(Clone, Copy)]
struct Pending {
    node: u32,
    distance: f32,
}
impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Pending {}
impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Pending {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .distance
            .total_cmp(&self.distance)
            .then(other.node.cmp(&self.node))
    }
}

fn adjacency(graph: &Graph) -> Result<BTreeMap<u32, Vec<(u32, f32)>>, String> {
    let mut paths = BTreeMap::<u32, Vec<(u32, f32)>>::new();
    for edge in graph.edges.iter().chain(&graph.teleports) {
        if !edge.yards.is_finite() || edge.yards < 0.0 {
            return Err(format!("invalid route edge {} -> {}", edge.from, edge.to));
        }
        paths
            .entry(edge.from)
            .or_default()
            .push((edge.to, edge.yards));
    }
    for edge in &graph.edges {
        paths
            .entry(edge.to)
            .or_default()
            .push((edge.from, edge.yards));
    }
    Ok(paths)
}

fn distances(paths: &BTreeMap<u32, Vec<(u32, f32)>>, start: u32) -> BTreeMap<u32, f32> {
    let mut best = BTreeMap::from([(start, 0.0)]);
    let mut pending = BinaryHeap::from([Pending {
        node: start,
        distance: 0.0,
    }]);
    while let Some(Pending { node, distance }) = pending.pop() {
        if distance > best[&node] {
            continue;
        }
        for &(next, length) in paths.get(&node).into_iter().flatten() {
            let candidate = distance + length;
            if best.get(&next).is_none_or(|value| candidate < *value) {
                best.insert(next, candidate);
                pending.push(Pending {
                    node: next,
                    distance: candidate,
                });
            }
        }
    }
    best
}

/// Greedily insert the reachable pack with the lowest incremental nav yards per force.
/// A candidate may be inserted between boss-route steps or appended at the end.
pub(super) fn plan(
    graph: &Graph,
    weights: &HashMap<u32, u64>,
    required: u64,
    hostile_sources: &HashSet<u32>,
) -> Result<PlannedRoute, String> {
    let paths = adjacency(graph)?;
    let distances: BTreeMap<_, _> = graph
        .packs
        .iter()
        .map(|pack| (pack.id, distances(&paths, pack.id)))
        .collect();
    let distance = |from: u32, to: u32| distances.get(&from)?.get(&to).copied();
    let points = |pack: &super::graph::Pack| {
        pack.spawns
            .iter()
            .filter(|spawn| hostile_sources.contains(&spawn.guid))
            .map(|spawn| weights.get(&spawn.entry).copied().unwrap_or(0))
            .sum::<u64>()
    };
    let mut steps = graph.route.clone();
    let mut selected: BTreeSet<_> = steps.iter().map(|step| step.pack).collect();
    if selected
        .iter()
        .any(|id| !graph.packs.iter().any(|pack| pack.id == *id))
    {
        return Err("boss route references an absent pack".into());
    }
    let mut total = graph
        .packs
        .iter()
        .filter(|pack| selected.contains(&pack.id))
        .map(points)
        .sum::<u64>();
    let mut added_packs = Vec::new();
    while total < required {
        let mut best: Option<(f64, u32, usize, u64)> = None;
        for pack in graph
            .packs
            .iter()
            .filter(|pack| !selected.contains(&pack.id))
        {
            let force = points(pack);
            if force == 0 {
                continue;
            }
            for index in 0..=steps.len() {
                let from = if index == 0 {
                    graph.entrance_pack
                } else {
                    steps[index - 1].pack
                };
                let Some(first) = distance(from, pack.id) else {
                    continue;
                };
                let extra = if let Some(next) = steps.get(index) {
                    let (Some(second), Some(direct)) =
                        (distance(pack.id, next.pack), distance(from, next.pack))
                    else {
                        continue;
                    };
                    (first + second - direct).max(0.0)
                } else {
                    first
                };
                let candidate = (f64::from(extra) / force as f64, pack.id, index, force);
                if best.as_ref().is_none_or(|old| {
                    candidate
                        .0
                        .total_cmp(&old.0)
                        .then(candidate.1.cmp(&old.1))
                        .then(candidate.2.cmp(&old.2))
                        .is_lt()
                }) {
                    best = Some(candidate);
                }
            }
        }
        let Some((_, id, index, force)) = best else {
            return Err(format!(
                "reachable authored route supplies {total} of {required} forces"
            ));
        };
        let pack = graph
            .packs
            .iter()
            .find(|pack| pack.id == id)
            .expect("selected graph pack");
        steps.insert(
            index,
            RouteStep {
                pack: id,
                anchor: pack.point().to_array(),
            },
        );
        selected.insert(id);
        added_packs.push(id);
        total = total.saturating_add(force);
    }
    Ok(PlannedRoute {
        steps,
        added_packs,
        planned_forces: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn azure_route_selects_enough_authored_forces() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let graph: Graph = serde_json::from_slice(
            &std::fs::read(root.join("content/dungeons/azure-vault-2515.json")).unwrap(),
        )
        .unwrap();
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let definition = server::mythic_plus_catalog::load_azure(&db)
            .unwrap()
            .unwrap();
        let selected = plan(
            &graph,
            &definition.creature_weights,
            definition.forces_required,
            &graph
                .packs
                .iter()
                .flat_map(|pack| pack.spawns.iter().map(|spawn| spawn.guid))
                .collect(),
        )
        .unwrap();
        assert!(selected.planned_forces >= definition.forces_required);
        assert!(!selected.added_packs.is_empty());
        let retained: Vec<_> = selected
            .steps
            .iter()
            .filter(|step| !selected.added_packs.contains(&step.pack))
            .map(|step| step.pack)
            .collect();
        assert_eq!(
            retained,
            graph.route.iter().map(|step| step.pack).collect::<Vec<_>>()
        );
    }

    #[test]
    fn adds_lowest_detour_per_force_without_reordering_boss_route() {
        let graph: Graph = serde_json::from_value(serde_json::json!({
            "map_id": 1, "entrance": [0,0], "entrance_height": 0, "entrance_pack": 1,
            "packs": [
                {"id":1,"position":[0,0],"height":0,"spawns":[]},
                {"id":2,"position":[1,0],"height":0,"spawns":[{"guid":2,"entry":20,"position":[1,0]}]},
                {"id":3,"position":[4,0],"height":0,"spawns":[{"guid":3,"entry":30,"position":[4,0]}]},
                {"id":4,"position":[10,0],"height":0,"spawns":[]}
            ],
            "edges":[{"from":1,"to":4,"yards":10},{"from":1,"to":2,"yards":1},{"from":2,"to":4,"yards":10},{"from":1,"to":3,"yards":4},{"from":3,"to":4,"yards":7}],
            "teleports":[],"unreachable":[],"route":[{"pack":1,"anchor":[0,0,0]},{"pack":4,"anchor":[10,0,0]}]
        })).unwrap();
        let eligible = HashSet::from([2, 3]);
        let selected = plan(&graph, &HashMap::from([(20, 5), (30, 10)]), 10, &eligible).unwrap();
        assert_eq!(selected.added_packs, vec![3]);
        assert_eq!(selected.planned_forces, 10);
        assert_eq!(
            selected
                .steps
                .iter()
                .map(|step| step.pack)
                .collect::<Vec<_>>(),
            vec![1, 3, 4]
        );
        assert!(
            plan(&graph, &HashMap::from([(20, 5), (30, 10)]), 16, &eligible)
                .unwrap_err()
                .contains("15 of 16")
        );
        let hostile_only = plan(
            &graph,
            &HashMap::from([(20, 5), (30, 10)]),
            5,
            &HashSet::from([2]),
        )
        .unwrap();
        assert_eq!(hostile_only.added_packs, vec![2]);
        assert_eq!(hostile_only.planned_forces, 5);
        assert!(
            plan(
                &graph,
                &HashMap::from([(20, 5), (30, 10)]),
                6,
                &HashSet::from([2])
            )
            .unwrap_err()
            .contains("5 of 6")
        );
    }
}
