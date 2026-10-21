//! Frozen 2D topology. Only the explicit extraction test opens navigation data.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Spawn {
    pub guid: u32,
    pub entry: u32,
    pub position: [f32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Pack {
    pub id: u32,
    pub position: [f32; 2],
    pub spawns: Vec<Spawn>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Edge {
    pub from: u32,
    pub to: u32,
    pub yards: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct RouteStep {
    pub pack: u32,
    pub anchor: [f32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Graph {
    pub map_id: u32,
    pub entrance: [f32; 2],
    pub entrance_pack: u32,
    pub packs: Vec<Pack>,
    pub edges: Vec<Edge>,
    pub unreachable: Vec<u32>,
    pub route: Vec<RouteStep>,
}

#[derive(Resource)]
pub(crate) struct Space(pub(super) Graph);

impl Space {
    pub(crate) fn route(&self, from: Vec3, to: Vec3) -> Result<Vec<Vec3>, String> {
        let nearest = |point: Vec3| {
            self.0
                .packs
                .iter()
                .min_by(|a, b| {
                    let distance = |pack: &Pack| {
                        Vec2::from_array(pack.position).distance(Vec2::new(point.x, point.z))
                    };
                    distance(a).total_cmp(&distance(b)).then(a.id.cmp(&b.id))
                })
                .map(|pack| pack.id)
                .ok_or("frozen graph has no packs")
        };
        let start = nearest(from)?;
        let goal = nearest(to)?;
        let mut distances = BTreeMap::from([(start, 0.0_f32)]);
        let mut previous = BTreeMap::new();
        let mut visited = BTreeSet::new();
        while let Some((&node, &distance)) = distances
            .iter()
            .filter(|(node, _)| !visited.contains(*node))
            .min_by(|a, b| a.1.total_cmp(b.1).then(a.0.cmp(b.0)))
        {
            if node == goal {
                break;
            }
            visited.insert(node);
            for edge in &self.0.edges {
                let next = if edge.from == node {
                    edge.to
                } else if edge.to == node {
                    edge.from
                } else {
                    continue;
                };
                let total = distance + edge.yards;
                if distances.get(&next).is_none_or(|known| total < *known) {
                    distances.insert(next, total);
                    previous.insert(next, node);
                }
            }
        }
        if !distances.contains_key(&goal) {
            return Err(format!(
                "frozen graph cannot reach pack{goal} from pack{start}"
            ));
        }
        let mut nodes = vec![goal];
        while *nodes.last().unwrap() != start {
            nodes.push(previous[nodes.last().unwrap()]);
        }
        nodes.reverse();
        let mut points: Vec<_> = nodes
            .into_iter()
            .skip(1)
            .map(|id| {
                let pack = self.0.packs.iter().find(|pack| pack.id == id).unwrap();
                Vec3::new(pack.position[0], 0.0, pack.position[1])
            })
            .collect();
        points.push(to);
        Ok(points)
    }
}

pub(super) fn graph_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/dungeons/stockade-34.json")
}

pub(super) fn read_graph() -> Result<Graph, String> {
    let path = graph_path();
    let bytes =
        std::fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn source_packs(data: &crate::world_data::WorldData) -> Vec<Pack> {
    let mut packs = BTreeMap::<u32, Vec<Spawn>>::new();
    for &index in &data.spawns_by_map[&34] {
        let spawn = &data.spawns[index];
        if !spawn.spawns_at(crate::maps::store::DIFFICULTY_NORMAL) {
            continue;
        }
        let id = data
            .formations
            .get(&spawn.guid)
            .map_or(spawn.guid, |formation| formation.leader_guid);
        packs.entry(id).or_default().push(Spawn {
            guid: spawn.guid,
            entry: spawn.template_entry,
            position: [spawn.position_x, -spawn.position_y],
        });
    }
    packs
        .into_iter()
        .map(|(id, mut spawns)| {
            spawns.sort_by_key(|spawn| spawn.guid);
            let position = spawns
                .iter()
                .find(|spawn| spawn.guid == id)
                .expect("formation leader is a Normal source spawn")
                .position;
            Pack {
                id,
                position,
                spawns,
            }
        })
        .collect()
}

fn source_point(data: &crate::world_data::WorldData, guid: u32) -> Vec3 {
    let spawn = data.spawns.iter().find(|spawn| spawn.guid == guid).unwrap();
    Vec3::new(spawn.position_x, spawn.position_z, -spawn.position_y)
}

fn query_edges(data: &crate::world_data::WorldData, packs: &[Pack]) -> Vec<Edge> {
    let directory = crate::test_data::world_db_path()
        .parent()
        .unwrap()
        .join("ground/navmesh/34");
    let mut nav = ::navigation::NavigationMap::open(&directory).expect("source map34 nav bake");
    let mut edges = Vec::new();
    for (index, from) in packs.iter().enumerate() {
        for to in &packs[index + 1..] {
            let start = source_point(data, from.id);
            let end = source_point(data, to.id);
            let Ok(points) = nav.path(start, end) else {
                continue;
            };
            let yards: f32 = points
                .windows(2)
                .map(|pair| pair[0].distance(pair[1]))
                .sum();
            if yards.is_finite() && yards > 0.0 {
                edges.push(Edge {
                    from: from.id,
                    to: to.id,
                    yards,
                });
            }
        }
    }
    edges.sort_by(|a, b| {
        a.yards
            .total_cmp(&b.yards)
            .then(a.from.cmp(&b.from))
            .then(a.to.cmp(&b.to))
    });
    edges
}

/// Minimum spanning forest plus four closest neighbours keeps the frozen input small.
fn select_edges(packs: &[Pack], candidates: Vec<Edge>) -> Vec<Edge> {
    let mut roots: BTreeMap<_, _> = packs.iter().map(|pack| (pack.id, pack.id)).collect();
    let mut counts = BTreeMap::<u32, usize>::new();
    let mut selected = Vec::new();
    for edge in candidates {
        let from = roots[&edge.from];
        let to = roots[&edge.to];
        let connects = from != to;
        let neighbour = counts.get(&edge.from).copied().unwrap_or(0) < 4
            || counts.get(&edge.to).copied().unwrap_or(0) < 4;
        if connects {
            for root in roots.values_mut().filter(|root| **root == to) {
                *root = from;
            }
        }
        if connects || neighbour {
            *counts.entry(edge.from).or_default() += 1;
            *counts.entry(edge.to).or_default() += 1;
            selected.push(edge);
        }
    }
    selected.sort_by_key(|edge| (edge.from, edge.to));
    selected
}

fn unreachable(packs: &[Pack], edges: &[Edge], entrance: u32) -> Vec<u32> {
    let mut reached = BTreeSet::from([entrance]);
    loop {
        let before = reached.len();
        for edge in edges {
            if reached.contains(&edge.from) {
                reached.insert(edge.to);
            }
            if reached.contains(&edge.to) {
                reached.insert(edge.from);
            }
        }
        if reached.len() == before {
            break;
        }
    }
    packs
        .iter()
        .filter(|pack| !reached.contains(&pack.id))
        .map(|pack| pack.id)
        .collect()
}

#[test]
fn frozen_graph_contains_an_explicit_unique_anchored_pull_order() {
    let bytes = std::fs::read(graph_path()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let route = value["route"]
        .as_array()
        .expect("Stockade requires an authored route, not runtime nearest-pack selection");
    assert!(!route.is_empty());
    let graph = read_graph().unwrap();
    let mut seen = BTreeSet::new();
    for step in route {
        let pack = step["pack"].as_u64().unwrap() as u32;
        assert!(seen.insert(pack), "duplicate authored pack{pack}");
        assert!(graph.packs.iter().any(|entry| entry.id == pack));
        let anchor = step["anchor"].as_array().unwrap();
        assert_eq!(anchor.len(), 2);
        assert!(
            anchor
                .iter()
                .all(|coordinate| coordinate.as_f64().is_some_and(f64::is_finite))
        );
    }
}

#[test]
fn azure_follower_graph_covers_native_normal_population_and_has_an_authored_route() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/azure-vault-2515.json");
    let bytes = std::fs::read(&path).expect("frozen Azure2515/205 graph");
    let graph: Graph = serde_json::from_slice(&bytes).unwrap();
    let data = crate::test_data::world_data();
    let expected: BTreeSet<_> = data.spawns_by_map[&2515]
        .iter()
        .map(|index| &data.spawns[*index])
        .filter(|spawn| spawn.spawns_at(crate::followers::population_difficulty(205)))
        .map(|spawn| spawn.guid)
        .collect();
    let actual: BTreeSet<_> = graph
        .packs
        .iter()
        .flat_map(|pack| &pack.spawns)
        .map(|spawn| spawn.guid)
        .collect();
    assert_eq!(graph.map_id, 2515);
    assert_eq!(
        actual, expected,
        "complete native205 alias uses Normal source population"
    );
    assert!(!graph.edges.is_empty(), "real frozen navigation distances");
    assert_eq!(
        graph.route.first().unwrap().pack,
        4294739577,
        "actual live opening Conjured Lasher"
    );
    let mut route_ids = BTreeSet::new();
    for step in &graph.route {
        assert!(
            route_ids.insert(step.pack),
            "no repeated authored pull step"
        );
        assert!(step.anchor.into_iter().all(f32::is_finite));
        assert!(graph.packs.iter().any(|pack| pack.id == step.pack));
    }
    for boss in [186644, 186739, 186737, 186738] {
        assert!(
            graph.packs.iter().any(|pack| route_ids.contains(&pack.id)
                && pack.spawns.iter().any(|spawn| spawn.entry == boss)),
            "authored route must include boss{boss}"
        );
    }
}

#[test]
#[ignore = "explicit one-time extraction from native world.db and nav bake"]
fn extract_stockade_graph() {
    let data = crate::test_data::world_data();
    let packs = source_packs(data);
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let entrance = maps.safe_locs[&3599].bevy_position();
    let nearest = packs
        .iter()
        .min_by(|a, b| {
            source_point(data, a.id)
                .distance(Vec3::new(entrance.x, entrance.y, entrance.z))
                .total_cmp(
                    &source_point(data, b.id)
                        .distance(Vec3::new(entrance.x, entrance.y, entrance.z)),
                )
        })
        .unwrap()
        .id;
    let edges = select_edges(&packs, query_edges(data, &packs));
    let graph = Graph {
        map_id: 34,
        entrance: [entrance.x, entrance.z],
        entrance_pack: nearest,
        unreachable: unreachable(&packs, &edges, nearest),
        route: read_graph()
            .expect("preserve the authored pull route during graph extraction")
            .route,
        packs,
        edges,
    };
    let path = graph_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec_pretty(&graph).unwrap()).unwrap();
    println!(
        "FROZEN {} packs {} edges unreachable={:?}: {}",
        graph.packs.len(),
        graph.edges.len(),
        graph.unreachable,
        path.display()
    );
}
