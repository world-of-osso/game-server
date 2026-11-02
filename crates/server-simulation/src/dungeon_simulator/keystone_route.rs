//! Add the least-cost authored force packs to the retained boss route.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet};

use super::graph::{Graph, RouteStep};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct PullSamples {
    map_id: u32,
    segments: Vec<PullSegment>,
}

#[derive(Deserialize)]
struct PullSegment {
    before_encounter: u32,
    samples: Vec<PullSample>,
}

#[derive(Deserialize)]
struct PullSample {
    targets: usize,
}

#[derive(Deserialize)]
struct MappedPulls {
    map_id: u32,
    pulls: Vec<MappedPull>,
}

#[derive(Deserialize)]
struct MappedPull {
    pull: u32,
    source_guids: Vec<u64>,
}

#[derive(Deserialize)]
struct FirstDamageEvidence {
    maps: Vec<MapFirstDamage>,
}

#[derive(Deserialize)]
struct MapFirstDamage {
    map_id: u32,
    reports: Vec<ReportFirstDamage>,
}

#[derive(Deserialize)]
struct ReportFirstDamage {
    report: String,
    first_damage_ms: BTreeMap<u32, u64>,
}

/// Boss entries in WCL first-damage order; every registered report must agree.
fn wcl_first_damage_order(map_id: u32) -> Result<Option<Vec<u32>>, String> {
    let evidence: FirstDamageEvidence = serde_json::from_str(include_str!(
        "../../../../content/dungeons/wcl-boss-first-damage.json"
    ))
    .map_err(|error| format!("parse WCL boss first damage: {error}"))?;
    let Some(map) = evidence.maps.into_iter().find(|map| map.map_id == map_id) else {
        return Ok(None);
    };
    let mut order: Option<Vec<u32>> = None;
    for report in map.reports {
        let mut hits: Vec<_> = report.first_damage_ms.into_iter().collect();
        hits.sort_by_key(|&(entry, ms)| (ms, entry));
        if hits.windows(2).any(|pair| pair[0].1 == pair[1].1) {
            return Err(format!(
                "WCL report {} has simultaneous boss first damage",
                report.report
            ));
        }
        let entries: Vec<_> = hits.into_iter().map(|(entry, _)| entry).collect();
        if order.as_ref().is_some_and(|order| *order != entries) {
            return Err(format!(
                "WCL report {} disagrees on map{map_id} boss first damage",
                report.report
            ));
        }
        order = Some(entries);
    }
    order
        .map(Some)
        .ok_or_else(|| format!("map{map_id} WCL boss first damage has no reports"))
}

/// Listed bosses fill their own extracted slots in WCL order; other bosses keep theirs.
fn order_bosses(entries: &[u32], first_damage: &[u32]) -> Result<Vec<u32>, String> {
    if let Some(missing) = first_damage.iter().find(|entry| !entries.contains(entry)) {
        return Err(format!(
            "WCL first-damage boss{missing} is not a graph boss"
        ));
    }
    let mut listed = first_damage.iter().copied();
    Ok(entries
        .iter()
        .map(|entry| {
            if first_damage.contains(entry) {
                listed.next().expect("one listed entry per slot")
            } else {
                *entry
            }
        })
        .collect())
}

/// Apply the WCL evidence registered for a map; unlisted maps do not chain.
pub(super) fn wcl_chain_groups(
    graph: &Graph,
    definition: &server::mythic_plus_runtime::ChallengeDefinition,
    hostile_sources: &HashSet<u64>,
) -> Result<Option<Vec<Vec<u64>>>, String> {
    let segment_sources = [include_str!(
        "../../../../content/dungeons/den-wcl-pulls.json"
    )];
    for source in segment_sources {
        let samples: PullSamples = serde_json::from_str(source)
            .map_err(|error| format!("parse WCL pull samples: {error}"))?;
        if samples.map_id == graph.map_id {
            return segment_chain_groups(graph, definition, hostile_sources, samples).map(Some);
        }
    }
    let mapped_sources = [include_str!(
        "../../../../content/dungeons/ruby-wcl-pulls.json"
    )];
    for source in mapped_sources {
        let samples: MappedPulls = serde_json::from_str(source)
            .map_err(|error| format!("parse WCL pull membership: {error}"))?;
        if samples.map_id == graph.map_id {
            return mapped_chain_groups(graph, definition, hostile_sources, samples).map(Some);
        }
    }
    Ok(None)
}

/// Partition consecutive route packs by the sourced median for each boss-relative segment.
fn segment_chain_groups(
    graph: &Graph,
    definition: &server::mythic_plus_runtime::ChallengeDefinition,
    hostile_sources: &HashSet<u64>,
    samples: PullSamples,
) -> Result<Vec<Vec<u64>>, String> {
    if graph.map_id != samples.map_id || definition.map_id != samples.map_id {
        return Err("Den pull evidence does not match challenge map".into());
    }
    if samples.segments.len() != definition.bosses.len()
        || samples
            .segments
            .iter()
            .zip(&definition.bosses)
            .any(|(segment, boss)| segment.before_encounter != boss.encounter_id)
    {
        return Err("Den pull evidence does not match ordered challenge encounters".into());
    }
    let targets: Vec<_> = samples
        .segments
        .iter()
        .map(|segment| {
            let mut counts: Vec<_> = segment
                .samples
                .iter()
                .map(|sample| sample.targets)
                .collect();
            counts.sort_unstable();
            match counts.as_slice() {
                [] => Err("Den pull segment has no samples".to_string()),
                [..] if counts.iter().any(|count| *count == 0) => {
                    Err("Den pull sample has zero targets".to_string())
                }
                _ => Ok((counts[(counts.len() - 1) / 2] + counts[counts.len() / 2]).div_ceil(2)),
            }
        })
        .collect::<Result<_, _>>()?;
    let mut steps = Vec::new();
    for step in &graph.route {
        let pack = graph
            .packs
            .iter()
            .find(|pack| pack.id == step.pack)
            .ok_or_else(|| format!("route references absent pack {}", step.pack))?;
        let boss = graph.boss_entries().iter().find(|entry| {
            let source = graph
                .follower_boss_sources
                .get(entry)
                .copied()
                .unwrap_or(**entry);
            pack.spawns.iter().any(|spawn| spawn.entry == source)
        });
        let count = pack
            .spawns
            .iter()
            .filter(|spawn| hostile_sources.contains(&spawn.guid))
            .count();
        steps.push((step.pack, count, boss.is_some()));
    }
    group_chain_steps(&steps, &targets)
}

fn mapped_chain_groups(
    graph: &Graph,
    definition: &server::mythic_plus_runtime::ChallengeDefinition,
    hostile_sources: &HashSet<u64>,
    samples: MappedPulls,
) -> Result<Vec<Vec<u64>>, String> {
    if samples.map_id != graph.map_id || definition.map_id != graph.map_id {
        return Err("WCL pull membership does not match challenge map".into());
    }
    let mut observed = HashMap::new();
    for pull in samples.pulls {
        for guid in pull.source_guids {
            if observed.insert(guid, pull.pull).is_some() {
                return Err(format!("WCL source {guid} belongs to multiple pulls"));
            }
        }
    }
    let mut groups = Vec::new();
    let mut pending = Vec::new();
    let mut previous = None;
    for step in &graph.route {
        let pack = graph
            .packs
            .iter()
            .find(|pack| pack.id == step.pack)
            .ok_or_else(|| format!("route references absent pack {}", step.pack))?;
        let boss = graph.boss_entries().iter().any(|entry| {
            let source = graph
                .follower_boss_sources
                .get(entry)
                .copied()
                .unwrap_or(*entry);
            pack.spawns.iter().any(|spawn| spawn.entry == source)
        });
        let memberships: BTreeSet<_> = pack
            .spawns
            .iter()
            .filter(|spawn| hostile_sources.contains(&spawn.guid))
            .filter_map(|spawn| observed.get(&spawn.guid).copied())
            .collect();
        if memberships.len() > 1 {
            return Err(format!("route pack {} matches multiple WCL pulls", pack.id));
        }
        let pull = if boss {
            None
        } else {
            memberships.first().copied()
        };
        if pull.is_none() || pull != previous {
            if !pending.is_empty() {
                groups.push(std::mem::take(&mut pending));
            }
        }
        if pull.is_none() {
            groups.push(vec![pack.id]);
        } else {
            pending.push(pack.id);
        }
        previous = pull;
    }
    if !pending.is_empty() {
        groups.push(pending);
    }
    Ok(groups)
}

fn group_chain_steps(
    steps: &[(u64, usize, bool)],
    targets: &[usize],
) -> Result<Vec<Vec<u64>>, String> {
    if targets.is_empty() {
        return Err("no WCL chain targets".into());
    }
    let mut groups = Vec::new();
    let mut pending = Vec::new();
    let mut count = 0;
    let mut segment = 0;
    for &(pack, units, boss) in steps {
        if boss {
            if !pending.is_empty() {
                groups.push(std::mem::take(&mut pending));
            }
            groups.push(vec![pack]);
            count = 0;
            segment += 1;
            continue;
        }
        pending.push(pack);
        count += units;
        let target = targets[segment.min(targets.len() - 1)];
        if count >= target {
            groups.push(std::mem::take(&mut pending));
            count = 0;
        }
    }
    if !pending.is_empty() {
        groups.push(pending);
    }
    Ok(groups)
}

/// Count only living hostile sources the owner can attack when planning starts.
pub(super) fn attackable_sources(
    world: &bevy::prelude::World,
    owner: bevy::prelude::Entity,
    actors: &BTreeMap<u64, bevy::prelude::Entity>,
) -> HashSet<u64> {
    actors
        .iter()
        .filter_map(|(&guid, &actor)| {
            (crate::spell_cast::targets::is_alive(world, actor)
                && crate::spell_cast::targets::is_hostile(world, owner, actor)
                && crate::spell_cast::targets::relation_matches(
                    world,
                    owner,
                    actor,
                    crate::spell_cast::targets::Relation::Enemy,
                ))
            .then_some(guid)
        })
        .collect()
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct PlannedRoute {
    pub steps: Vec<RouteStep>,
    pub added_packs: Vec<u64>,
    pub planned_forces: u64,
}

#[derive(Clone, Copy)]
struct Pending {
    node: u64,
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

fn adjacency(graph: &Graph) -> Result<BTreeMap<u64, Vec<(u64, f32)>>, String> {
    let mut paths = BTreeMap::<u64, Vec<(u64, f32)>>::new();
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

fn distances(paths: &BTreeMap<u64, Vec<(u64, f32)>>, start: u64) -> BTreeMap<u64, f32> {
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

/// The boss route and its final boss pack.
fn boss_route(graph: &Graph) -> Result<(Vec<RouteStep>, Option<u64>), String> {
    let entries = match wcl_first_damage_order(graph.map_id)? {
        Some(order) => order_bosses(graph.boss_entries(), &order)?,
        None => graph.boss_entries().to_vec(),
    };
    let mut bosses = Vec::new();
    for entry in &entries {
        let source = graph
            .follower_boss_sources
            .get(entry)
            .copied()
            .unwrap_or(*entry);
        let packs: Vec<_> = graph
            .packs
            .iter()
            .filter(|pack| pack.spawns.iter().any(|spawn| spawn.entry == source))
            .collect();
        let [pack] = packs.as_slice() else {
            return Err(format!("boss{entry} has {} source packs", packs.len()));
        };
        bosses.push(RouteStep {
            pack: pack.id,
            anchor: pack.point().to_array(),
        });
    }
    let final_boss = bosses.last().map(|boss| boss.pack);
    if graph.route.is_empty() {
        if bosses.is_empty() {
            return Err("graph has no authored boss route or encounter bindings".into());
        }
        return Ok((bosses, final_boss));
    }
    let mut remaining = graph.route.iter();
    for boss in bosses {
        if !remaining.any(|step| step.pack == boss.pack) {
            return Err(format!(
                "authored route omits or reorders boss pack{}",
                boss.pack
            ));
        }
    }
    Ok((graph.route.clone(), final_boss))
}

/// Greedily insert the reachable pack with the lowest incremental nav yards per force.
/// A candidate may be inserted between route steps, never after the final boss:
/// retail parties finish forces before the end boss.
pub(super) fn plan(
    graph: &Graph,
    weights: &HashMap<u32, u64>,
    required: u64,
    hostile_sources: &HashSet<u64>,
) -> Result<PlannedRoute, String> {
    let paths = adjacency(graph)?;
    let distances: BTreeMap<_, _> = graph
        .packs
        .iter()
        .map(|pack| (pack.id, distances(&paths, pack.id)))
        .collect();
    let distance = |from: u64, to: u64| distances.get(&from)?.get(&to).copied();
    let points = |pack: &super::graph::Pack| {
        pack.spawns
            .iter()
            .filter(|spawn| hostile_sources.contains(&spawn.guid))
            .map(|spawn| weights.get(&spawn.entry).copied().unwrap_or(0))
            .sum::<u64>()
    };
    let (mut steps, final_boss) = boss_route(graph)?;
    let mut selected: BTreeSet<_> = steps.iter().map(|step| step.pack).collect();
    if selected
        .iter()
        .any(|id| !graph.packs.iter().any(|pack| pack.id == *id))
    {
        return Err("boss route references an absent pack".into());
    }
    let mut previous = graph.entrance_pack;
    for step in &steps {
        if distance(previous, step.pack).is_none() {
            return Err(format!(
                "boss route cannot reach pack{} from pack{previous}",
                step.pack
            ));
        }
        previous = step.pack;
    }
    let mut total = graph
        .packs
        .iter()
        .filter(|pack| selected.contains(&pack.id))
        .map(points)
        .sum::<u64>();
    let mut added_packs = Vec::new();
    while total < required {
        let mut best: Option<(f64, u64, usize, u64)> = None;
        let last = final_boss
            .and_then(|boss| steps.iter().position(|step| step.pack == boss))
            .unwrap_or(steps.len());
        for pack in graph
            .packs
            .iter()
            .filter(|pack| !selected.contains(&pack.id))
        {
            let force = points(pack);
            if force == 0 {
                continue;
            }
            for index in 0..=last {
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
    fn ruby_wcl_membership_groups_adjacent_packs_and_keeps_unmatched_separate() {
        use super::super::graph::{Pack, Spawn};
        let sources = [29001484, 29001531, 29001514, 90000001, 29001773, 29001776];
        let packs: Vec<_> = sources
            .iter()
            .map(|&guid| Pack {
                id: guid,
                position: [0.0, 0.0],
                height: 0.0,
                spawns: vec![Spawn {
                    guid,
                    entry: 1,
                    position: [0.0, 0.0],
                }],
            })
            .collect();
        let graph = Graph {
            map_id: 2521,
            entrance: [0.0, 0.0],
            entrance_height: 0.0,
            entrance_pack: sources[0],
            route: sources
                .iter()
                .map(|&pack| RouteStep {
                    pack,
                    anchor: [0.0; 3],
                })
                .collect(),
            packs,
            edges: vec![],
            teleports: vec![],
            transports: vec![],
            flights: vec![],
            unreachable: vec![],
            follower_difficulty: 23,
            follower_boss_entries: vec![],
            follower_boss_sources: BTreeMap::new(),
            follower_boss_bits: BTreeMap::new(),
            follower_encounter_actor_bits: BTreeMap::new(),
        };
        let definition = server::mythic_plus_runtime::ChallengeDefinition {
            map_id: 2521,
            challenge_id: 399,
            name: "Ruby Life Pools".into(),
            timers_ms: [0; 3],
            forces_required: 0,
            creature_weights: HashMap::new(),
            scenario_event_weights: HashMap::new(),
            bosses: vec![],
        };
        let groups = wcl_chain_groups(&graph, &definition, &sources.into_iter().collect())
            .unwrap()
            .unwrap();
        assert_eq!(
            groups,
            vec![
                vec![29001484, 29001531, 29001514],
                vec![90000001],
                vec![29001773, 29001776]
            ]
        );
    }

    #[test]
    fn chain_groups_keep_consecutive_packs_and_isolate_bosses() {
        let steps = [
            (10, 2, false),
            (11, 1, false),
            (12, 1, false),
            (20, 1, true),
            (21, 2, false),
            (22, 2, false),
            (23, 1, false),
            (30, 1, true),
        ];
        assert_eq!(
            group_chain_steps(&steps, &[3, 4]).unwrap(),
            vec![
                vec![10, 11],
                vec![12],
                vec![20],
                vec![21, 22],
                vec![23],
                vec![30]
            ]
        );
    }

    #[test]
    fn den_wcl_source_contains_the_observed_segment_medians() {
        let evidence: PullSamples = serde_json::from_str(include_str!(
            "../../../../content/dungeons/den-wcl-pulls.json"
        ))
        .unwrap();
        let actual: Vec<_> = evidence
            .segments
            .iter()
            .map(|segment| {
                let mut counts: Vec<_> = segment
                    .samples
                    .iter()
                    .map(|sample| sample.targets)
                    .collect();
                counts.sort_unstable();
                (segment.before_encounter, counts[counts.len() / 2])
            })
            .collect();
        assert_eq!(actual, [(3207, 18), (3208, 7), (3209, 16)]);
    }

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
    fn planning_excludes_immune_hostiles_but_keeps_attackable_hostiles() {
        use bevy::prelude::World;
        use shared::components::{Health, Npc, Player, UnitFactionTemplate, UnitFlags};
        let db = rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut world = World::new();
        world.insert_resource(crate::faction::load_faction_templates(&db).unwrap());
        let map = crate::maps::InMap {
            map_id: 2825,
            instance_id: 1,
        };
        let owner = world
            .spawn((
                map,
                Player {
                    name: "Hunter".into(),
                    race: 1,
                    class: 3,
                    appearance: Default::default(),
                },
                UnitFactionTemplate(3),
            ))
            .id();
        let make_npc = |world: &mut World, entry, flags| {
            world
                .spawn((
                    map,
                    Npc {
                        template_id: entry,
                        name: "Den enemy".into(),
                    },
                    UnitFactionTemplate(14),
                    UnitFlags(flags),
                    Health {
                        current: 100.0,
                        max: 100.0,
                    },
                ))
                .id()
        };
        let nanea = make_npc(&mut world, 244889, 0x340);
        let enemy = make_npc(&mut world, 241813, 0);
        assert!(crate::spell_cast::targets::is_hostile(&world, owner, nanea));
        let selected = attackable_sources(
            &world,
            owner,
            &BTreeMap::from([(31000461, nanea), (31000462, enemy)]),
        );
        assert_eq!(selected, HashSet::from([31000462]));
        world.entity_mut(nanea).insert(UnitFlags(0));
        let selected = attackable_sources(
            &world,
            owner,
            &BTreeMap::from([(31000461, nanea), (31000462, enemy)]),
        );
        assert_eq!(selected, HashSet::from([31000461, 31000462]));
    }

    #[test]
    fn derives_boss_order_when_extracted_graph_has_no_route() {
        let mut graph: Graph = serde_json::from_value(serde_json::json!({
            "map_id": 2825, "entrance": [0,0], "entrance_height": 0, "entrance_pack": 1,
            "packs": [
                {"id":1,"position":[0,0],"height":0,"spawns":[]},
                {"id":2,"position":[1,0],"height":0,"spawns":[{"guid":2,"entry":100,"position":[1,0]}]},
                {"id":3,"position":[2,0],"height":0,"spawns":[{"guid":3,"entry":200,"position":[2,0]}]},
                {"id":4,"position":[3,0],"height":0,"spawns":[{"guid":4,"entry":300,"position":[3,0]}]}
            ],
            "edges":[{"from":1,"to":2,"yards":1},{"from":2,"to":3,"yards":1},{"from":3,"to":4,"yards":1}],
            "teleports":[],"unreachable":[],"route":[],"follower_boss_entries":[100,200,300]
        })).unwrap();
        let result = plan(&graph, &HashMap::new(), 0, &HashSet::new()).unwrap();
        assert_eq!(
            result
                .steps
                .iter()
                .map(|step| step.pack)
                .collect::<Vec<_>>(),
            [2, 3, 4]
        );
        graph.packs.retain(|pack| pack.id != 3);
        assert!(
            plan(&graph, &HashMap::new(), 0, &HashSet::new())
                .unwrap_err()
                .contains("boss200 has 0 source packs")
        );
    }

    #[test]
    fn ruby_final_encounter_opens_on_the_wcl_first_damaged_boss() {
        let graph: Graph = serde_json::from_value(serde_json::json!({
            "map_id": 2521, "entrance": [0,0], "entrance_height": 0, "entrance_pack": 1,
            "packs": [
                {"id":1,"position":[0,0],"height":0,"spawns":[]},
                {"id":2,"position":[1,0],"height":0,"spawns":[{"guid":2,"entry":188252,"position":[1,0]}]},
                {"id":3,"position":[2,0],"height":0,"spawns":[{"guid":3,"entry":190484,"position":[2,0]}]},
                {"id":4,"position":[2,1],"height":0,"spawns":[{"guid":4,"entry":190485,"position":[2,1]}]}
            ],
            "edges":[{"from":1,"to":2,"yards":1},{"from":2,"to":3,"yards":1},{"from":3,"to":4,"yards":1}],
            "teleports":[],"unreachable":[],"route":[],
            "follower_boss_entries":[188252,190484,190485]
        }))
        .unwrap();
        let result = plan(&graph, &HashMap::new(), 0, &HashSet::new()).unwrap();
        assert_eq!(
            result
                .steps
                .iter()
                .map(|step| step.pack)
                .collect::<Vec<_>>(),
            [2, 4, 3],
            "Erkhart must open the final encounter before Kyrakka"
        );
    }

    #[test]
    fn no_force_pack_follows_the_final_boss() {
        let graph: Graph = serde_json::from_value(serde_json::json!({
            "map_id": 1, "entrance": [0,0], "entrance_height": 0, "entrance_pack": 1,
            "packs": [
                {"id":1,"position":[0,0],"height":0,"spawns":[]},
                {"id":2,"position":[10,0],"height":0,"spawns":[{"guid":2,"entry":100,"position":[10,0]}]},
                {"id":3,"position":[11,0],"height":0,"spawns":[{"guid":3,"entry":30,"position":[11,0]}]}
            ],
            "edges":[{"from":1,"to":2,"yards":10},{"from":2,"to":3,"yards":1}],
            "teleports":[],"unreachable":[],"route":[],"follower_boss_entries":[100]
        }))
        .unwrap();
        // Appending after the boss costs 1 yd; going past the boss and back costs 2 yd.
        let selected = plan(&graph, &HashMap::from([(30, 5)]), 5, &HashSet::from([3])).unwrap();
        assert_eq!(
            selected
                .steps
                .iter()
                .map(|step| step.pack)
                .collect::<Vec<_>>(),
            [3, 2],
            "the final boss must be the last route step"
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
