//! Offline portal hierarchy and bounded abstract route search.
mod artifact;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Stable Detour tile identity and polygon index, independent of runtime tile load order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolygonKey {
    pub tile_x: i32,
    pub tile_z: i32,
    pub layer: i32,
    pub polygon: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortalNode {
    pub polygon: PolygonKey,
    pub cluster: u32,
    pub position: [f32; 3],
    pub edges: Vec<PortalEdge>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PortalEdge {
    pub destination: u32,
    pub cost: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PortalGraph {
    pub version: u32,
    pub tile_digest: [u8; 32],
    pub clusters: Vec<PolygonKey>,
    pub nodes: Vec<PortalNode>,
}

#[derive(Clone, Copy, Debug)]
struct QueueEntry {
    node: usize,
    estimate: f32,
}
impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.estimate == other.estimate
    }
}
impl Eq for QueueEntry {}
impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimate
            .total_cmp(&self.estimate)
            .then_with(|| other.node.cmp(&self.node))
    }
}

impl PortalGraph {
    /// Keep existing graph artifacts consistent with the runtime border-link limit.
    pub(crate) fn prune_high_cross_tile_edges(&mut self) {
        let nodes: Vec<_> = self
            .nodes
            .iter()
            .map(|node| (node.polygon, node.position[1]))
            .collect();
        for node in &mut self.nodes {
            node.edges.retain(|edge| {
                let (other, height) = nodes[edge.destination as usize];
                let same_tile = node.polygon.tile_x == other.tile_x
                    && node.polygon.tile_z == other.tile_z
                    && node.polygon.layer == other.layer;
                same_tile || (node.position[1] - height).abs() <= crate::AGENT_CLIMB
            });
        }
    }

    /// Find the lowest-cost abstract route from actual start-leg costs to goal-leg costs.
    pub fn route(
        &self,
        starts: &[(u32, f32)],
        goals: &[(u32, f32)],
        target: Vec3,
    ) -> Result<Option<Vec<u32>>, String> {
        if !target.is_finite() {
            return Err("non-finite hierarchy target".into());
        }
        let mut goal_cost = vec![f32::INFINITY; self.nodes.len()];
        for &(node, cost) in goals {
            let entry = goal_cost
                .get_mut(node as usize)
                .ok_or("invalid hierarchy goal")?;
            if !cost.is_finite() || cost < 0.0 {
                return Err("invalid hierarchy goal cost".into());
            }
            *entry = entry.min(cost);
        }
        let mut distance = vec![f32::INFINITY; self.nodes.len()];
        let mut previous = vec![None; self.nodes.len()];
        let mut queue = BinaryHeap::new();
        for &(node, cost) in starts {
            let index = node as usize;
            let point = Vec3::from_array(
                self.nodes
                    .get(index)
                    .ok_or("invalid hierarchy start")?
                    .position,
            );
            if !cost.is_finite() || cost < 0.0 || !point.is_finite() {
                return Err("invalid hierarchy start cost or point".into());
            }
            if cost < distance[index] {
                distance[index] = cost;
                queue.push(QueueEntry {
                    node: index,
                    estimate: cost + point.distance(target),
                });
            }
        }
        let mut best = None;
        let mut best_cost = f32::INFINITY;
        while let Some(entry) = queue.pop() {
            let node = &self.nodes[entry.node];
            if entry.estimate
                > distance[entry.node] + Vec3::from_array(node.position).distance(target)
            {
                continue;
            }
            if entry.estimate >= best_cost {
                break;
            }
            let finish = distance[entry.node] + goal_cost[entry.node];
            if finish < best_cost {
                best = Some(entry.node);
                best_cost = finish;
            }
            for edge in &node.edges {
                let next = edge.destination as usize;
                let destination = self.nodes.get(next).ok_or("invalid hierarchy edge")?;
                if !edge.cost.is_finite()
                    || edge.cost < 0.0
                    || !Vec3::from_array(destination.position).is_finite()
                {
                    return Err("invalid hierarchy edge cost or point".into());
                }
                let candidate = distance[entry.node] + edge.cost;
                if candidate < distance[next] {
                    distance[next] = candidate;
                    previous[next] = Some(entry.node);
                    queue.push(QueueEntry {
                        node: next,
                        estimate: candidate
                            + Vec3::from_array(destination.position).distance(target),
                    });
                }
            }
        }
        let Some(mut node) = best else {
            return Ok(None);
        };
        let mut route = vec![node as u32];
        while let Some(parent) = previous[node] {
            node = parent;
            route.push(node as u32);
        }
        route.reverse();
        Ok(Some(route))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(x: f32, edges: &[(u32, f32)]) -> PortalNode {
        PortalNode {
            polygon: PolygonKey {
                tile_x: 0,
                tile_z: 0,
                layer: 0,
                polygon: 0,
            },
            cluster: 0,
            position: [x, 0.0, 0.0],
            edges: edges
                .iter()
                .map(|&(destination, cost)| PortalEdge { destination, cost })
                .collect(),
        }
    }
    #[test]
    fn abstract_route_uses_connected_lower_cost_portals() {
        let graph = PortalGraph {
            nodes: vec![
                node(0.0, &[(1, 2.0), (2, 6.0)]),
                node(1.0, &[(3, 8.0)]),
                node(2.0, &[(3, 2.0)]),
                node(3.0, &[]),
            ],
            ..Default::default()
        };
        assert_eq!(
            graph
                .route(&[(0, 0.0)], &[(3, 0.0)], Vec3::new(3.0, 0.0, 0.0))
                .unwrap(),
            Some(vec![0, 2, 3])
        );
        assert_eq!(
            graph.route(&[(3, 0.0)], &[(0, 0.0)], Vec3::ZERO).unwrap(),
            None
        );
    }
    #[test]
    fn abstract_route_rejects_invalid_graph_data() {
        let graph = PortalGraph {
            nodes: vec![node(0.0, &[(2, 1.0)])],
            ..Default::default()
        };
        assert!(graph.route(&[(0, 0.0)], &[(0, 5.0)], Vec3::ZERO).is_err());
    }
}
