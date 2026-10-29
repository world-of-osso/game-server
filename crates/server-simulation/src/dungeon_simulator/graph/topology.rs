//! Build walking edges from the baked navigation paths, retaining their corners.
use super::{Edge, Pack, Spawn};
use bevy::prelude::Vec3;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn source_packs(
    data: &crate::world_data::WorldData,
    map: u16,
    difficulty: u32,
) -> Result<Vec<Pack>, String> {
    let indexes = data
        .spawns_by_map
        .get(&map)
        .ok_or_else(|| format!("map{map} missing population"))?;
    let mut groups = BTreeMap::<u32, Vec<&crate::world_data::CreatureSpawn>>::new();
    for &index in indexes {
        let spawn = &data.spawns[index];
        if spawn.spawns_at(difficulty) {
            let leader = data
                .formations
                .get(&spawn.guid)
                .map_or(spawn.guid, |formation| formation.leader_guid);
            groups.entry(leader).or_default().push(spawn);
        }
    }
    if groups.is_empty() {
        return Err(format!(
            "map{map} missing population for difficulty{difficulty}"
        ));
    }
    groups
        .into_iter()
        .map(|(leader, mut members)| {
            members.sort_by_key(|spawn| spawn.guid);
            let anchor = members
                .iter()
                .find(|spawn| spawn.guid == leader)
                .ok_or_else(|| {
                    format!("map{map} formation leader{leader} absent from population")
                })?;
            Ok(Pack {
                id: leader,
                position: [anchor.position_x, -anchor.position_y],
                height: anchor.position_z,
                spawns: members
                    .iter()
                    .map(|spawn| Spawn {
                        guid: spawn.guid,
                        entry: spawn.template_entry,
                        position: [spawn.position_x, -spawn.position_y],
                    })
                    .collect(),
            })
        })
        .collect()
}

fn root(roots: &BTreeMap<u32, u32>, mut id: u32) -> u32 {
    while roots[&id] != id {
        id = roots[&id];
    }
    id
}

pub(super) fn connect_packs<F>(packs: &mut Vec<Pack>, mut path: F) -> Result<Vec<Edge>, String>
where
    F: FnMut(Vec3, Vec3) -> Result<Vec<Vec3>, String>,
{
    let mut pairs = Vec::new();
    for (index, a) in packs.iter().enumerate() {
        for b in &packs[index + 1..] {
            pairs.push((a.point().distance_squared(b.point()), a.id, b.id));
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let points: BTreeMap<_, _> = packs.iter().map(|pack| (pack.id, pack.point())).collect();
    let mut roots: BTreeMap<_, _> = points.keys().map(|id| (*id, *id)).collect();
    let mut neighbours = BTreeMap::<u32, usize>::new();
    let mut next = packs
        .iter()
        .flat_map(|pack| std::iter::once(pack.id).chain(pack.spawns.iter().map(|spawn| spawn.guid)))
        .max()
        .unwrap_or(0);
    let mut edges = Vec::new();
    for (_, a, b) in pairs {
        let (ra, rb) = (root(&roots, a), root(&roots, b));
        if ra == rb
            && neighbours.get(&a).copied().unwrap_or(0) >= 4
            && neighbours.get(&b).copied().unwrap_or(0) >= 4
        {
            continue;
        }
        let Ok(corners) = path(points[&a], points[&b]) else {
            continue;
        };
        if path(points[&b], points[&a]).is_err() {
            continue;
        }
        let corners: Vec<_> = corners
            .into_iter()
            .filter(|point| point.is_finite())
            .collect();
        if corners.is_empty() {
            continue;
        }
        if ra != rb {
            roots.insert(rb, ra);
        }
        *neighbours.entry(a).or_default() += 1;
        *neighbours.entry(b).or_default() += 1;
        let mut last = a;
        let mut previous = points[&a];
        for point in corners.into_iter().chain([points[&b]]) {
            if point.distance_squared(previous) < 0.000001 {
                continue;
            }
            let node = if point == points[&b] {
                b
            } else {
                next = next
                    .checked_add(1)
                    .ok_or("navigation waypoint identifiers exhausted")?;
                packs.push(Pack {
                    id: next,
                    position: [point.x, point.z],
                    height: point.y,
                    spawns: Vec::new(),
                });
                next
            };
            edges.push(Edge {
                from: last,
                to: node,
                yards: previous.distance(point),
            });
            previous = point;
            last = node;
        }
        if last != b {
            edges.push(Edge {
                from: last,
                to: b,
                yards: previous.distance(points[&b]),
            });
        }
    }
    Ok(edges)
}

pub(super) fn unreachable(packs: &[Pack], edges: &[Edge], entrance: u32) -> Vec<u32> {
    let mut reached = BTreeSet::from([entrance]);
    loop {
        let size = reached.len();
        for edge in edges {
            if reached.contains(&edge.from) {
                reached.insert(edge.to);
            }
            if reached.contains(&edge.to) {
                reached.insert(edge.from);
            }
        }
        if size == reached.len() {
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
fn edges_follow_real_path_corners_and_do_not_connect_unreachable_packs() {
    let pack = |id, x| Pack {
        id,
        position: [x, 0.0],
        height: 0.0,
        spawns: vec![],
    };
    let mut packs = vec![pack(1, 0.0), pack(2, 10.0), pack(3, 100.0)];
    let edges = connect_packs(&mut packs, |a, b| {
        if a.x > 50.0 || b.x > 50.0 {
            return Err("disconnected".into());
        }
        Ok(vec![a, Vec3::new(5.0, 0.0, 5.0), b])
    })
    .unwrap();
    assert_eq!(edges.len(), 2);
    assert_eq!(packs[3].point(), Vec3::new(5.0, 0.0, 5.0));
    assert_eq!(unreachable(&packs, &edges, 1), vec![3]);
    assert!(edges.iter().all(|edge| edge.yards > 7.0));
}
