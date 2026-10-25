//! The flight network from world.db `taxi_nodes`, `taxi_path` and
//! `taxi_path_node` (Retail 12.1.0.69933 DB2 `TaxiNodes` / `TaxiPath` /
//! `TaxiPathNode`, imported by scripts/db2.py). Positions are WoW coordinates.
//!
//! - A flight master's node is the nearest node on its continent that the player's
//!   team sees and flies from and that has a path (AzerothCore
//!   `ObjectMgr::GetNearestTaxiNode`, ObjectMgr.cpp:7154, which skips nodes outside
//!   `sTaxiNodesMask`; the map-flag check is Retail's per-team node visibility).
//! - A team sees nodes with its `TaxiNodes.Flags` map bit (`ShowOnAllianceMap` 0x1,
//!   `ShowOnHordeMap` 0x2) and flies from those with its mount
//!   (`MountCreatureID_1` Alliance, `_0` Horde, as `GetTaxiMountDisplayId`).
//! - Nodes with a `ConditionID` need PlayerCondition, which is not evaluated: they
//!   are never usable.
//! - Paths with a `Teleport` point (`TaxiPathNode.Flags` 0x1, a map change) are left
//!   out; `Delay` stops are not honored.
//! - A route is the shortest chain of paths, by flown distance, whose nodes are all
//!   known; its cost is the sum of `TaxiPath.Cost` copper.

use std::collections::{BTreeSet, BinaryHeap, HashMap};

use bevy::prelude::*;
use rusqlite::Connection;

use crate::terrain_height::wow_to_bevy;

/// `TaxiNodes.Flags` (Retail `TaxiNodeFlags`).
const SHOW_ON_ALLIANCE_MAP: u32 = 0x1;
const SHOW_ON_HORDE_MAP: u32 = 0x2;
/// `TaxiPathNode.Flags` (Retail `TaxiPathNodeFlags`).
const PATH_NODE_TELEPORT: u32 = 0x1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Team {
    Alliance,
    Horde,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaxiNode {
    pub name: String,
    pub continent: u32,
    pub position: Vec3,
    flags: u32,
    condition_id: u32,
    /// `MountCreatureID_0` (Horde), `_1` (Alliance).
    mount_creatures: [u32; 2],
}

#[derive(Debug, Clone, PartialEq)]
struct TaxiPath {
    to: u32,
    cost: u32,
    /// Flown points, WoW coordinates.
    points: Vec<Vec3>,
    length: f32,
}

#[derive(Resource, Debug, Clone, Default)]
pub(crate) struct TaxiNetwork {
    pub nodes: HashMap<u32, TaxiNode>,
    /// Paths by source node.
    paths: HashMap<u32, Vec<TaxiPath>>,
    /// Creature template display of each mount creature.
    mount_displays: HashMap<u32, u32>,
}

/// A flight: `nodes` from the start to the destination, `cost` in copper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaxiRoute {
    pub nodes: Vec<u32>,
    pub cost: u32,
}

impl TaxiNetwork {
    pub fn visible(&self, node: u32, team: Team) -> bool {
        let bit = match team {
            Team::Alliance => SHOW_ON_ALLIANCE_MAP,
            Team::Horde => SHOW_ON_HORDE_MAP,
        };
        self.nodes
            .get(&node)
            .is_some_and(|node| node.flags & bit != 0)
    }

    fn mount_creature(&self, node: u32, team: Team) -> Option<u32> {
        let node = self.nodes.get(&node)?;
        let creature = match team {
            Team::Horde => node.mount_creatures[0],
            Team::Alliance => node.mount_creatures[1],
        };
        (creature != 0).then_some(creature)
    }

    /// The node can start or end a flight for `team`.
    pub fn usable(&self, node: u32, team: Team) -> bool {
        self.visible(node, team)
            && self.mount_creature(node, team).is_some()
            && self.nodes[&node].condition_id == 0
    }

    pub fn mount_display(&self, node: u32, team: Team) -> Option<u32> {
        self.mount_displays
            .get(&self.mount_creature(node, team)?)
            .copied()
    }

    pub fn nearest_node(&self, continent: u32, position: Vec3, team: Team) -> Option<u32> {
        self.nodes
            .iter()
            .filter(|(id, node)| {
                node.continent == continent
                    && self.visible(**id, team)
                    && self.mount_creature(**id, team).is_some()
                    && self.paths.contains_key(id)
            })
            .min_by(|(_, a), (_, b)| {
                a.position
                    .distance_squared(position)
                    .total_cmp(&b.position.distance_squared(position))
            })
            .map(|(id, _)| *id)
    }

    /// Flight points of `continent` that `team` sees and that have a path.
    pub fn map_nodes(&self, continent: u32, team: Team) -> impl Iterator<Item = (u32, &TaxiNode)> {
        self.nodes.iter().filter_map(move |(&id, node)| {
            (node.continent == continent && self.visible(id, team) && self.paths.contains_key(&id))
                .then_some((id, node))
        })
    }

    fn path(&self, from: u32, to: u32) -> Option<&TaxiPath> {
        self.paths.get(&from)?.iter().find(|path| path.to == to)
    }

    /// Dijkstra over paths between usable nodes the player knows.
    pub fn route(
        &self,
        from: u32,
        to: u32,
        known: &BTreeSet<u32>,
        team: Team,
    ) -> Option<TaxiRoute> {
        let allowed = |node: u32| known.contains(&node) && self.usable(node, team);
        if from == to || !allowed(from) || !allowed(to) {
            return None;
        }
        let mut best: HashMap<u32, (f32, u32)> = HashMap::from([(from, (0.0, from))]);
        let mut queue = BinaryHeap::from([Reverse(from, 0.0)]);
        while let Some(Reverse(node, length)) = queue.pop() {
            if node == to {
                break;
            }
            if best
                .get(&node)
                .is_some_and(|(shortest, _)| length > *shortest)
            {
                continue;
            }
            for path in self.paths.get(&node).into_iter().flatten() {
                if !allowed(path.to) {
                    continue;
                }
                let next = length + path.length;
                if best
                    .get(&path.to)
                    .is_none_or(|(shortest, _)| next < *shortest)
                {
                    best.insert(path.to, (next, node));
                    queue.push(Reverse(path.to, next));
                }
            }
        }
        best.get(&to)?;
        let mut nodes = vec![to];
        while let Some(&last) = nodes.last()
            && last != from
        {
            nodes.push(best[&last].1);
        }
        nodes.reverse();
        let cost = nodes
            .windows(2)
            .map(|hop| self.path(hop[0], hop[1]).map_or(0, |path| path.cost))
            .sum();
        Some(TaxiRoute { nodes, cost })
    }

    /// The route's points in Bevy coordinates, hop after hop.
    pub fn flight_points(&self, nodes: &[u32]) -> Vec<Vec3> {
        nodes
            .windows(2)
            .filter_map(|hop| self.path(hop[0], hop[1]))
            .flat_map(|path| path.points.iter())
            .map(|point| {
                let (x, y, z) = wow_to_bevy(point.x, point.y, point.z);
                Vec3::new(x, y, z)
            })
            .collect()
    }
}

/// Min-heap entry: node and flown distance.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Reverse(u32, f32);

impl Eq for Reverse {}

impl Ord for Reverse {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.1.total_cmp(&self.1).then(other.0.cmp(&self.0))
    }
}

impl PartialOrd for Reverse {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub(crate) fn load_taxi_network(conn: &Connection) -> rusqlite::Result<TaxiNetwork> {
    let nodes = load_nodes(conn)?;
    let points = load_points(conn)?;
    let mut paths: HashMap<u32, Vec<TaxiPath>> = HashMap::new();
    let mut statement = conn.prepare("SELECT ID, FromTaxiNode, ToTaxiNode, Cost FROM taxi_path")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, u32>(3)?,
        ))
    })?;
    for row in rows {
        let (id, from, to, cost) = row?;
        let Some(Some(points)) = points.get(&id) else {
            continue;
        };
        if !nodes.contains_key(&from) || !nodes.contains_key(&to) {
            continue;
        }
        let length = points
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum();
        paths.entry(from).or_default().push(TaxiPath {
            to,
            cost,
            points: points.clone(),
            length,
        });
    }
    let mount_displays = load_mount_displays(conn, &nodes)?;
    Ok(TaxiNetwork {
        nodes,
        paths,
        mount_displays,
    })
}

fn load_nodes(conn: &Connection) -> rusqlite::Result<HashMap<u32, TaxiNode>> {
    let mut statement = conn.prepare(
        "SELECT ID, Name_lang, ContinentID, Pos_0, Pos_1, Pos_2, Flags, ConditionID, \
         MountCreatureID_0, MountCreatureID_1 FROM taxi_nodes",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            TaxiNode {
                name: row.get(1)?,
                continent: row.get(2)?,
                position: Vec3::new(row.get(3)?, row.get(4)?, row.get(5)?),
                flags: row.get(6)?,
                condition_id: row.get(7)?,
                mount_creatures: [row.get(8)?, row.get(9)?],
            },
        ))
    })?;
    rows.collect()
}

/// Points of each path in `NodeIndex` order; `None` for a path that teleports.
fn load_points(conn: &Connection) -> rusqlite::Result<HashMap<u32, Option<Vec<Vec3>>>> {
    let mut statement = conn.prepare(
        "SELECT PathID, Loc_0, Loc_1, Loc_2, Flags FROM taxi_path_node ORDER BY PathID, NodeIndex",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?),
            row.get::<_, u32>(4)?,
        ))
    })?;
    let mut points: HashMap<u32, Option<Vec<Vec3>>> = HashMap::new();
    for row in rows {
        let (path, point, flags) = row?;
        let entry = points.entry(path).or_insert_with(|| Some(Vec::new()));
        if flags & PATH_NODE_TELEPORT != 0 {
            *entry = None;
        }
        if let Some(list) = entry {
            list.push(point);
        }
    }
    Ok(points)
}

fn load_mount_displays(
    conn: &Connection,
    nodes: &HashMap<u32, TaxiNode>,
) -> rusqlite::Result<HashMap<u32, u32>> {
    let creatures: BTreeSet<u32> = nodes
        .values()
        .flat_map(|node| node.mount_creatures)
        .filter(|&creature| creature != 0)
        .collect();
    let mut statement = conn.prepare(
        "SELECT CreatureDisplayID FROM content_creature_template_model \
         WHERE CreatureID = ?1 ORDER BY Idx LIMIT 1",
    )?;
    let mut displays = HashMap::new();
    for creature in creatures {
        let display: Option<u32> = statement
            .query_row([creature], |row| row.get(0))
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                error => Err(error),
            })?;
        if let Some(display) = display {
            displays.insert(creature, display);
        }
    }
    Ok(displays)
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod tests;
