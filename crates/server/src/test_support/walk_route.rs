//! Walking endpoints retain their place along existing native-distance graph edges.
use super::{Edge, Graph};
use bevy::prelude::Vec3;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};

// Recorded f32 mover positions differ from their frozen edge by up to 0.002yd.
const EDGE_ROUNDING_YARDS: f32 = 0.01;
type Legs = VecDeque<(Vec3, f32)>;
type Adjacency = BTreeMap<u32, Vec<(u32, f32)>>;
type Distances = BTreeMap<u32, f32>;
type Previous = BTreeMap<u32, u32>;

struct Pending {
    yards: f32,
    node: u32,
}

impl PartialEq for Pending {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Pending {}

impl Ord for Pending {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .yards
            .total_cmp(&self.yards)
            .then(other.node.cmp(&self.node))
    }
}

impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Endpoint {
    point: Vec3,
    offset: f32,
    connections: Vec<(u32, f32)>,
    edge: Option<(usize, f32)>,
}

pub(super) fn read(graph: &Graph, from: Vec3, to: Vec3) -> Result<Legs, String> {
    let points: BTreeMap<_, _> = graph
        .packs
        .iter()
        .map(|pack| (pack.id, pack.point()))
        .collect();
    let origin = locate_endpoint(graph, &points, from)?;
    let destination = locate_endpoint(graph, &points, to)?;
    let adjacency = read_adjacency(graph);
    let (nodes, yards) = shortest_nodes(&adjacency, &origin, &destination)?;
    let direct = shared_edge_yards(graph, &origin, &destination).filter(|direct| *direct <= yards);
    let mut legs = Legs::new();
    append_leg(&mut legs, from, origin.point, origin.offset);
    if let Some(yards) = direct {
        append_leg(&mut legs, origin.point, destination.point, yards);
    } else {
        append_node_legs(
            &mut legs,
            &points,
            &adjacency,
            &nodes,
            &origin,
            &destination,
        );
    }
    append_leg(&mut legs, destination.point, to, destination.offset);
    Ok(legs)
}

fn locate_endpoint(
    graph: &Graph,
    points: &BTreeMap<u32, Vec3>,
    position: Vec3,
) -> Result<Endpoint, String> {
    let (&id, &point) = points
        .iter()
        .min_by(|a, b| {
            a.1.distance_squared(position)
                .total_cmp(&b.1.distance_squared(position))
                .then(a.0.cmp(b.0))
        })
        .ok_or("frozen graph has no packs")?;
    let node = Endpoint {
        point,
        offset: position.distance(point),
        connections: vec![(id, 0.0)],
        edge: None,
    };
    let mut closest = graph
        .edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| project_edge(points, index, edge, position))
        .chain(std::iter::once(node))
        .min_by(|a, b| {
            a.offset
                .total_cmp(&b.offset)
                .then(a.edge.is_some().cmp(&b.edge.is_some()))
        })
        .unwrap();
    if closest.offset <= EDGE_ROUNDING_YARDS {
        closest.point = position;
        closest.offset = 0.0;
    }
    Ok(closest)
}

fn project_edge(
    points: &BTreeMap<u32, Vec3>,
    index: usize,
    edge: &Edge,
    position: Vec3,
) -> Option<Endpoint> {
    let a = points[&edge.from];
    let direction = points[&edge.to] - a;
    let squared = direction.length_squared();
    if squared == 0.0 {
        return None;
    }
    let fraction = ((position - a).dot(direction) / squared).clamp(0.0, 1.0);
    let point = a + direction * fraction;
    Some(Endpoint {
        point,
        offset: position.distance(point),
        connections: vec![
            (edge.from, fraction * edge.yards),
            (edge.to, (1.0 - fraction) * edge.yards),
        ],
        edge: Some((index, fraction)),
    })
}

fn read_adjacency(graph: &Graph) -> Adjacency {
    let mut adjacency: Adjacency = graph
        .packs
        .iter()
        .map(|pack| (pack.id, Vec::new()))
        .collect();
    for edge in &graph.edges {
        adjacency
            .get_mut(&edge.from)
            .unwrap()
            .push((edge.to, edge.yards));
        adjacency
            .get_mut(&edge.to)
            .unwrap()
            .push((edge.from, edge.yards));
    }
    adjacency
}

fn shortest_nodes(
    adjacency: &Adjacency,
    from: &Endpoint,
    to: &Endpoint,
) -> Result<(Vec<u32>, f32), String> {
    let (distances, previous) = solve_distances(adjacency, &from.connections);
    let (goal, yards) = to
        .connections
        .iter()
        .filter_map(|(node, tail)| distances.get(node).map(|distance| (*node, distance + tail)))
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
        .ok_or("frozen walk graph cannot connect its endpoint components")?;
    let mut nodes = vec![goal];
    while let Some(&node) = previous.get(nodes.last().unwrap()) {
        nodes.push(node);
    }
    nodes.reverse();
    Ok((nodes, yards))
}

fn solve_distances(adjacency: &Adjacency, starts: &[(u32, f32)]) -> (Distances, Previous) {
    let mut distances: Distances = starts.iter().copied().collect();
    let mut previous = Previous::new();
    let mut pending: BinaryHeap<_> = starts
        .iter()
        .map(|&(node, yards)| Pending { yards, node })
        .collect();
    while let Some(Pending {
        yards: distance,
        node,
    }) = pending.pop()
    {
        if distance != distances[&node] {
            continue;
        }
        for &(next, yards) in &adjacency[&node] {
            let total = distance + yards;
            if distances.get(&next).is_none_or(|known| total < *known) {
                distances.insert(next, total);
                previous.insert(next, node);
                pending.push(Pending {
                    yards: total,
                    node: next,
                });
            }
        }
    }
    (distances, previous)
}

fn shared_edge_yards(graph: &Graph, from: &Endpoint, to: &Endpoint) -> Option<f32> {
    let (a, from_fraction) = from.edge?;
    let (b, to_fraction) = to.edge?;
    (a == b).then(|| (to_fraction - from_fraction).abs() * graph.edges[a].yards)
}

fn append_node_legs(
    legs: &mut Legs,
    points: &BTreeMap<u32, Vec3>,
    adjacency: &Adjacency,
    nodes: &[u32],
    from: &Endpoint,
    to: &Endpoint,
) {
    let first = nodes[0];
    let start_yards = from
        .connections
        .iter()
        .find(|(node, _)| *node == first)
        .unwrap()
        .1;
    append_leg(legs, from.point, points[&first], start_yards);
    for pair in nodes.windows(2) {
        let yards = adjacency[&pair[0]]
            .iter()
            .find(|(node, _)| *node == pair[1])
            .unwrap()
            .1;
        append_leg(legs, points[&pair[0]], points[&pair[1]], yards);
    }
    let last = *nodes.last().unwrap();
    let tail_yards = to
        .connections
        .iter()
        .find(|(node, _)| *node == last)
        .unwrap()
        .1;
    append_leg(legs, points[&last], to.point, tail_yards);
}

fn append_leg(legs: &mut Legs, from: Vec3, to: Vec3, yards: f32) {
    if from != to || yards > 0.0 {
        legs.push_back((to, yards));
    }
}
