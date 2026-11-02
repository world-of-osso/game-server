//! Extract follower topology from the canonical population and baked navigation.
use super::{Edge, Graph, Pack, TransportEdge, source, topology};
use bevy::prelude::Vec3;
use navigation::{NavigationMap, NavigationPlan};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Furthest horizontal re-anchor of a boss players cannot attack. Temple of Sethraliss
/// (map 1877) Avatar: player and enemy samples stand 13-31 yd from the healed boss in
/// WCL M4cdBNJkTjavYgQV/11 pull 12, ax1vNFnfPdXrzRQ2/16 pull 9, ZgBCXQvcnhaAYT1r/5 pull 10.
const OBJECTIVE_ANCHOR_MAX_YARDS: f32 = 35.0;
/// `ChrRaces.FactionID` templates of Human (Alliance) and Orc (Horde).
const PLAYER_FACTION_TEMPLATES: [u32; 2] = [1, 2];

/// Audit record of a boss source pack moved off its unreachable spawn point.
#[derive(Debug, serde::Serialize)]
struct BossAnchor {
    entry: u32,
    pack: u64,
    original: [f32; 3],
    anchor: [f32; 3],
    horizontal_yards: f32,
    via_pack: u64,
    reason: &'static str,
}

/// Walk navigation legs until arrival or until a leg repeats; `follow` ends a proven
/// disconnect at the polygon closest to `end` instead of failing.
fn route_legs(
    nav: &mut NavigationMap,
    mut start: Vec3,
    end: Vec3,
    follow: bool,
) -> Result<(Vec<Vec3>, bool), String> {
    let mut plan = NavigationPlan::default();
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    loop {
        let leg = if follow {
            match nav.follow_path(start, end, &mut plan) {
                // Detour's closest point to `end` is `start` itself (query.rs project_partial_end).
                Err(error) if error == "unreachable target (partial corridor made no progress)" => {
                    if result.is_empty() {
                        result.push(start);
                    }
                    return Ok((result, false));
                }
                leg => leg?,
            }
        } else {
            nav.path_with_plan(start, end, &mut plan)?
        };
        if leg.collision_drop.is_some() {
            return Err("navigation path requires a registered collision drop".into());
        }
        let endpoint = *leg.points.last().ok_or("empty navigation segment")?;
        result.extend(leg.points);
        if leg.complete {
            return Ok((result, true));
        }
        if endpoint == start || !seen.insert(endpoint.to_array().map(f32::to_bits)) {
            return Ok((result, false));
        }
        start = endpoint;
    }
}

fn complete_route(nav: &mut NavigationMap, start: Vec3, end: Vec3) -> Result<Vec<Vec3>, String> {
    match route_legs(nav, start, end, false)? {
        (points, true) => Ok(points),
        _ => Err("navigation segment repeated without reaching destination".into()),
    }
}

/// The walk from `start` to the reachable point closest to `end`.
fn closest_route(nav: &mut NavigationMap, start: Vec3, end: Vec3) -> Result<Vec<Vec3>, String> {
    route_legs(nav, start, end, true).map(|(points, _)| points)
}

/// Whether players can attack the spawn: runtime faction reaction and immunity flags
/// (TC Object.cpp `IsValidAttackTarget`).
fn player_can_attack(
    data: &crate::world_data::WorldData,
    factions: &server::faction::FactionTemplates,
    difficulty: u32,
    guid: u64,
    entry: u32,
) -> Result<bool, String> {
    use shared::components::UnitFlags;
    let template = data
        .template_for(entry, difficulty)
        .ok_or_else(|| format!("boss{entry} has no creature template"))?;
    let flags = data
        .spawns
        .iter()
        .find(|spawn| spawn.guid == guid)
        .and_then(|spawn| spawn.unit_flags)
        .unwrap_or(template.unit_flags);
    let immune = UnitFlags::NON_ATTACKABLE
        | UnitFlags::NON_ATTACKABLE_2
        | UnitFlags::IMMUNE_TO_PC
        | UnitFlags::NOT_SELECTABLE;
    if flags & immune != 0 {
        return Ok(false);
    }
    let creature = server::faction::UnitFaction {
        template: u32::from(template.faction),
        is_player: false,
    };
    Ok(PLAYER_FACTION_TEMPLATES.iter().any(|&template| {
        let player = server::faction::UnitFaction {
            template,
            is_player: true,
        };
        factions.can_attack(player, creature)
    }))
}

/// Append `corners` from pack `from` to pack `to` as walking edges.
fn chain_route(graph: &mut Graph, from: u64, to: u64, corners: &[Vec3]) {
    let point = |id| {
        graph
            .packs
            .iter()
            .find(|pack| pack.id == id)
            .unwrap()
            .point()
    };
    let (mut previous, target) = (point(from), point(to));
    let mut next = graph
        .packs
        .iter()
        .flat_map(|pack| std::iter::once(pack.id).chain(pack.spawns.iter().map(|spawn| spawn.guid)))
        .max()
        .unwrap_or(0);
    let mut last = from;
    for &corner in corners.iter().chain([&target]) {
        if corner.distance_squared(previous) < 0.000001 {
            continue;
        }
        let node = if corner == target {
            to
        } else {
            next += 1;
            graph.packs.push(Pack {
                id: next,
                position: [corner.x, corner.z],
                height: corner.y,
                spawns: Vec::new(),
            });
            next
        };
        graph.edges.push(Edge {
            from: last,
            to: node,
            yards: previous.distance(corner),
        });
        previous = corner;
        last = node;
    }
    if last != to {
        graph.edges.push(Edge {
            from: last,
            to,
            yards: previous.distance(target),
        });
    }
}

/// Move each unreachable boss source pack whose boss players cannot attack (a healed or
/// protected encounter objective) to the reachable point closest to its spawn. Spawn,
/// encounter binding and credit are unchanged. An attackable unreachable boss, or a
/// closest point beyond `OBJECTIVE_ANCHOR_MAX_YARDS`, is a data error.
fn anchor_objective_bosses(
    graph: &mut Graph,
    mut attackable: impl FnMut(u64, u32) -> Result<bool, String>,
    mut closest: impl FnMut(Vec3, Vec3) -> Result<Vec<Vec3>, String>,
) -> Result<Vec<BossAnchor>, String> {
    let sources: BTreeSet<_> = graph.follower_boss_sources.values().copied().collect();
    let mut anchors = Vec::new();
    for source in sources {
        let packs: Vec<_> = graph
            .packs
            .iter()
            .filter_map(|pack| {
                let spawn = pack.spawns.iter().find(|spawn| spawn.entry == source)?;
                Some((pack.id, pack.point(), spawn.guid))
            })
            .collect();
        let [(id, original, guid)] = packs[..] else {
            continue;
        };
        if !graph.unreachable.contains(&id) {
            continue;
        }
        if attackable(guid, source)? {
            return Err(format!(
                "attackable boss{source} source pack{id} is unreachable from the entrance"
            ));
        }
        let via = graph
            .packs
            .iter()
            .filter(|pack| !graph.unreachable.contains(&pack.id))
            .min_by(|a, b| {
                (a.point().distance_squared(original), a.id)
                    .partial_cmp(&(b.point().distance_squared(original), b.id))
                    .unwrap()
            })
            .map(|pack| (pack.id, pack.point()))
            .ok_or("no reachable pack")?;
        let corners = closest(via.1, original)?;
        let anchor = *corners.last().ok_or("empty closest route")?;
        let horizontal = (anchor - original).with_y(0.0).length();
        if horizontal > OBJECTIVE_ANCHOR_MAX_YARDS {
            return Err(format!(
                "boss{source} closest reachable point is {horizontal:.1} yd from its spawn, over {OBJECTIVE_ANCHOR_MAX_YARDS} yd"
            ));
        }
        let back = closest(anchor, via.1)?;
        if back.last().is_none_or(|end| end.distance(via.1) > 0.01) {
            return Err(format!(
                "boss{source} anchor {anchor:?} cannot walk back to pack{}",
                via.0
            ));
        }
        let pack = graph.packs.iter_mut().find(|pack| pack.id == id).unwrap();
        pack.position = [anchor.x, anchor.z];
        pack.height = anchor.y;
        chain_route(graph, via.0, id, &corners);
        anchors.push(BossAnchor {
            entry: source,
            pack: id,
            original: original.to_array(),
            anchor: anchor.to_array(),
            horizontal_yards: horizontal,
            via_pack: via.0,
            reason: "boss not attackable by players; spawn off the entrance navmesh component",
        });
        graph.unreachable = topology::unreachable(
            &graph.packs,
            &graph.edges,
            &graph.teleports,
            graph.entrance_pack,
        );
    }
    Ok(anchors)
}

pub(crate) fn asset_root() -> std::path::PathBuf {
    std::env::var_os("DUNGEON_SIMULATOR_ASSETS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/home/osso/Projects/world-of-osso/game-engine/data".into())
}

fn boss_sources(
    map: u32,
    bosses: &[(u32, u32)],
    packs: &[super::Pack],
    accessories: &[(u32, u32)],
) -> Result<BTreeMap<u32, u32>, String> {
    let present = |entry| {
        packs
            .iter()
            .any(|pack| pack.spawns.iter().any(|spawn| spawn.entry == entry))
    };
    bosses
        .iter()
        .map(|(_, entry)| {
            if present(*entry) {
                return Ok((*entry, *entry));
            }
            let vehicles: Vec<_> = accessories
                .iter()
                .filter_map(|(vehicle, rider)| {
                    (*rider == *entry && present(*vehicle)).then_some(*vehicle)
                })
                .collect();
            match vehicles.as_slice() {
                [vehicle] => Ok((*entry, *vehicle)),
                [] => Err(format!(
                    "map{map} bound boss{entry} missing from population and vehicle accessories"
                )),
                _ => Err(format!(
                    "map{map} bound boss{entry} has ambiguous vehicle sources: {vehicles:?}"
                )),
            }
        })
        .collect()
}

fn append_registered_transports(
    map: u32,
    packs: &mut Vec<Pack>,
    registrations: &[server::test_support::transport::Registration],
) -> Result<(Vec<Edge>, Vec<TransportEdge>), String> {
    let mut next = packs
        .iter()
        .flat_map(|pack| std::iter::once(pack.id).chain(pack.spawns.iter().map(|spawn| spawn.guid)))
        .chain(
            registrations
                .iter()
                .filter_map(|registration| registration.source_guid.map(u64::from)),
        )
        .max()
        .unwrap_or(0);
    let mut edges = Vec::new();
    let mut transports = Vec::new();
    for registration in registrations {
        let (from, source_guid) =
            if registration.action == server::test_support::transport::Action::TaxiProximity {
                next = next
                    .checked_add(1)
                    .ok_or("transport waypoint identifiers exhausted")?;
                let point = registration.source;
                packs.push(Pack {
                    id: next,
                    position: [point.x, point.z],
                    height: point.y,
                    spawns: Vec::new(),
                });
                (next, 0)
            } else if let Some(guid) = registration.source_guid {
                next = next
                    .checked_add(1)
                    .ok_or("transport waypoint identifiers exhausted")?;
                let point = registration.source;
                packs.push(Pack {
                    id: next,
                    position: [point.x, point.z],
                    height: point.y,
                    spawns: Vec::new(),
                });
                (next, u64::from(guid))
            } else {
                let candidates: Vec<_> = packs
                    .iter()
                    .flat_map(|pack| {
                        pack.spawns
                            .iter()
                            .filter(move |spawn| {
                                spawn.entry == registration.entry
                                    && pack.point().distance(registration.source) < 1.0
                            })
                            .map(move |spawn| (pack.id, spawn.guid))
                    })
                    .collect();
                let [(from, guid)] = candidates.as_slice() else {
                    return Err(format!(
                        "map{map} registered transport entry{} has {} matching source spawns",
                        registration.entry,
                        candidates.len()
                    ));
                };
                (*from, *guid)
            };
        next = next
            .checked_add(1)
            .ok_or("transport waypoint identifiers exhausted")?;
        let destination = registration.destination;
        packs.push(Pack {
            id: next,
            position: [destination.x, destination.z],
            height: destination.y,
            spawns: Vec::new(),
        });
        edges.push(Edge {
            from,
            to: next,
            yards: registration.distance,
        });
        transports.push(TransportEdge {
            from,
            to: next,
            source_guid,
            radius: registration.radius,
            departure: registration.departure.map(|point| point.to_array()),
            landing: destination.to_array(),
            action: registration.action,
        });
    }
    Ok((edges, transports))
}

fn extract(
    map: u32,
    db: &rusqlite::Connection,
    nav_dir: &Path,
    difficulty: Option<u32>,
) -> Result<(Graph, Vec<u32>, Vec<BossAnchor>), String> {
    let source = source::read_source(db, map, difficulty)?;
    navigation::hierarchy::PortalGraph::read_artifact(nav_dir)
        .map_err(|error| format!("map{map} invalid navmesh portal graph: {error}"))?;
    let map16 =
        u16::try_from(map).map_err(|_| format!("map{map} is outside population map range"))?;
    let mut packs =
        topology::source_packs(crate::test_data::world_data(), map16, source.difficulty)?;
    let entrance = Vec3::new(source.entrance[0], source.entrance[2], -source.entrance[1]);
    let mut nav =
        NavigationMap::open(nav_dir).map_err(|error| format!("map{map} navmesh: {error}"))?;
    let mut nearest: Vec<_> = packs
        .iter()
        .map(|pack| {
            (
                entrance.distance_squared(pack.point()),
                pack.id,
                pack.point(),
            )
        })
        .collect();
    nearest.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let (opening, entrance_route) = nearest
        .into_iter()
        .find_map(|(_, id, point)| {
            complete_route(&mut nav, entrance, point)
                .ok()
                .map(|route| (id, route))
        })
        .ok_or_else(|| format!("map{map} entrance cannot reach the population on navmesh"))?;
    let bosses: Vec<_> = source.bosses.iter().map(|(_, entry)| *entry).collect();
    let accessories = server::test_support::read_vehicle_accessories(&asset_root(), db)
        .map_err(|error| format!("map{map} vehicle accessory catalog: {error}"))?;
    let sources = boss_sources(map, &source.bosses, &packs, &accessories)?;
    let route_bosses = bosses.iter().map(|entry| sources[entry]).collect();
    let bits = source
        .bosses
        .iter()
        .map(|(bit, entry)| (*entry, *bit))
        .collect();
    let mut registrations = server::test_support::transport::read_registrations(map, db)?;
    if let Some((source, drop)) = nav.registered_collision_drop() {
        registrations.push(server::test_support::transport::Registration {
            entry: 0,
            source_guid: Some(0),
            source,
            destination: drop.landing,
            distance: source.distance(drop.departure) + drop.departure.distance(drop.landing),
            radius: navigation::AGENT_RADIUS,
            departure: Some(drop.departure),
            action: server::test_support::transport::Action::CollisionDrop,
        });
    }
    let (teleports, transports) = append_registered_transports(map, &mut packs, &registrations)?;
    let mut edges =
        topology::connect_packs(&mut packs, |from, to| complete_route(&mut nav, from, to))?;
    let entrance_pack =
        topology::append_entrance_route(&mut packs, &mut edges, entrance, opening, entrance_route)?;
    let unreachable = topology::unreachable(&packs, &edges, &teleports, entrance_pack);
    let mut graph = Graph {
        map_id: map,
        entrance: [entrance.x, entrance.z],
        entrance_height: entrance.y,
        entrance_pack,
        packs,
        edges,
        teleports,
        transports,
        flights: Vec::new(),
        unreachable,
        route: Vec::new(),
        follower_difficulty: source.difficulty,
        follower_boss_entries: bosses,
        follower_boss_sources: sources,
        follower_boss_bits: bits,
        follower_encounter_actor_bits: BTreeMap::new(),
    };
    if map == 2662 {
        let (paths, deck_start) = super::super::flight::build_paths(&graph, &mut nav, db)?;
        let deck_end = *paths
            .first()
            .and_then(|path| path.first())
            .ok_or("Dawnbreaker replay lacks a flight takeoff")?;
        let deck_end_pack = append_dawnbreaker_deck(&mut graph, deck_start, deck_end)?;
        for (index, path) in paths.into_iter().enumerate() {
            append_flight_path(
                &mut graph,
                &path,
                (index == 0).then_some(deck_end_pack),
                |from, to| complete_route(&mut nav, from, to),
            )?;
        }
        graph.unreachable = topology::unreachable(
            &graph.packs,
            &graph.edges,
            &graph.teleports,
            graph.entrance_pack,
        );
    }
    let factions = server::faction::load_faction_templates(db)
        .map_err(|error| format!("map{map} faction templates: {error}"))?;
    let data = crate::test_data::world_data();
    let difficulty = graph.follower_difficulty;
    let anchors = anchor_objective_bosses(
        &mut graph,
        |guid, entry| player_can_attack(data, &factions, difficulty, guid, entry),
        |from, to| closest_route(&mut nav, from, to),
    )
    .map_err(|error| format!("map{map} {error}"))?;
    Ok((graph, route_bosses, anchors))
}

fn append_dawnbreaker_deck(graph: &mut Graph, start: Vec3, end: Vec3) -> Result<u64, String> {
    let id = graph
        .packs
        .iter()
        .flat_map(|pack| std::iter::once(pack.id).chain(pack.spawns.iter().map(|spawn| spawn.guid)))
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("deck waypoint identifiers exhausted")?;
    let destination = id
        .checked_add(1)
        .ok_or("deck waypoint identifiers exhausted")?;
    for (id, point) in [(id, start), (destination, end)] {
        graph.packs.push(Pack {
            id,
            position: [point.x, point.z],
            height: point.y,
            spawns: Vec::new(),
        });
    }
    graph.entrance = [start.x, start.z];
    graph.entrance_height = start.y;
    graph.entrance_pack = id;
    graph.teleports.push(Edge {
        from: id,
        to: destination,
        yards: start.distance(end),
    });
    graph.transports.push(TransportEdge {
        from: id,
        to: destination,
        source_guid: 0,
        radius: 0.001,
        departure: Some(start.to_array()),
        landing: end.to_array(),
        action: server::test_support::transport::Action::RecordedDawnbreakerDeck,
    });
    Ok(destination)
}

fn append_flight_path(
    graph: &mut Graph,
    path: &[Vec3],
    from_anchor: Option<u64>,
    mut walk: impl FnMut(Vec3, Vec3) -> Result<Vec<Vec3>, String>,
) -> Result<(), String> {
    let (Some(&start), Some(&end)) = (path.first(), path.last()) else {
        return Err("empty flight path".into());
    };
    let mut anchor = |point: Vec3| -> Result<u64, String> {
        let mut candidates: Vec<_> = graph
            .packs
            .iter()
            .map(|pack| (point.distance_squared(pack.point()), pack.id, pack.point()))
            .collect();
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let (pack, corners) = candidates
            .into_iter()
            .find_map(|(_, id, destination)| {
                walk(point, destination).ok().map(|corners| (id, corners))
            })
            .ok_or_else(|| {
                format!("flight endpoint {point:?} has no complete native walking route")
            })?;
        topology::append_entrance_route(&mut graph.packs, &mut graph.edges, point, pack, corners)
    };
    let from = match from_anchor {
        Some(id) => id,
        None => anchor(start)?,
    };
    let to = anchor(end)?;
    let yards = path.windows(2).map(|pair| pair[0].distance(pair[1])).sum();
    graph.teleports.push(Edge { from, to, yards });
    graph.flights.push(super::FlightEdge {
        from,
        to,
        points: path.iter().map(|point| point.to_array()).collect(),
    });
    Ok(())
}

#[test]
fn flight_path_connects_native_walk_components_in_one_direction() {
    let pack = |id, x| Pack {
        id,
        position: [x, 0.0],
        height: 0.0,
        spawns: vec![],
    };
    let mut graph = Graph {
        map_id: 2662,
        entrance: [0.0, 0.0],
        entrance_height: 0.0,
        entrance_pack: 1,
        packs: vec![pack(1, 0.0), pack(2, 100.0)],
        edges: vec![],
        teleports: vec![],
        transports: vec![],
        flights: vec![],
        unreachable: vec![2],
        route: vec![],
        follower_difficulty: 0,
        follower_boss_entries: vec![],
        follower_boss_sources: Default::default(),
        follower_boss_bits: Default::default(),
        follower_encounter_actor_bits: Default::default(),
    };
    let start = Vec3::ZERO;
    let end = Vec3::new(100.0, 0.0, 0.0);
    let path = [start, Vec3::new(50.0, 15.0, 0.0), end];
    append_flight_path(&mut graph, &path, None, |a, b| {
        if a.distance(b) > 1.0 {
            Err("disconnected".into())
        } else {
            Ok(vec![a, b])
        }
    })
    .unwrap();
    let flight = graph.flights[0].clone();
    assert_eq!(flight.points, path.map(|point| point.to_array()));
    let space = super::Space(graph);
    assert_eq!(
        space.next_translocation(start, end),
        Ok(Some((flight.from, flight.to)))
    );
    assert!(space.next_translocation(end, start).is_err());
}

#[test]
fn registered_transport_creates_a_directed_edge_to_the_native_landing() {
    use server::test_support::transport::{Action, Registration};
    let mut packs = vec![Pack {
        id: 11,
        position: [1.0, 2.0],
        height: 3.0,
        spawns: vec![super::Spawn {
            guid: 11,
            entry: 194667,
            position: [1.0, 2.0],
        }],
    }];
    let registration = Registration {
        entry: 194667,
        source_guid: None,
        source: Vec3::new(1.0, 3.0, 2.0),
        destination: Vec3::new(4.0, 30.0, 5.0),
        distance: 42.0,
        radius: 7.0,
        departure: None,
        action: Action::SpellClick,
    };
    let (edges, transports) =
        append_registered_transports(2521, &mut packs, &[registration]).unwrap();
    assert_eq!(edges[0].from, 11);
    assert_eq!(edges[0].to, 12);
    assert_eq!(edges[0].yards, 42.0);
    assert_eq!(packs[1].point(), Vec3::new(4.0, 30.0, 5.0));
    assert_eq!(transports[0].source_guid, 11);
    assert_eq!(transports[0].action, Action::SpellClick);
    let object = Registration {
        entry: 630573,
        source_guid: Some(81002923),
        source: Vec3::new(9.0, 3.0, 2.0),
        destination: Vec3::new(10.0, 30.0, 5.0),
        distance: 30.0,
        radius: 5.0,
        departure: None,
        action: Action::ObjectScript,
    };
    let (edges, transports) = append_registered_transports(2923, &mut packs, &[object]).unwrap();
    assert_eq!(transports[0].source_guid, 81002923);
    assert_eq!(transports[0].action, Action::ObjectScript);
    assert_eq!(edges[0].from, 81002924);
    assert_eq!(
        packs
            .iter()
            .find(|pack| pack.id == edges[0].from)
            .unwrap()
            .point(),
        Vec3::new(9.0, 3.0, 2.0)
    );
}

#[test]
fn windrunner_taxi_proximity_extracts_an_actor_free_passage() {
    use server::test_support::transport::{Action, Registration};
    let mut packs = Vec::new();
    let registration = Registration {
        entry: 10308,
        source_guid: None,
        source: Vec3::new(5214.278, 100.709, 3126.375),
        destination: Vec3::new(5215.815, 302.012, 3079.736),
        distance: 500.0,
        radius: 20.0,
        departure: None,
        action: Action::TaxiProximity,
    };
    let (edges, transports) =
        append_registered_transports(2805, &mut packs, &[registration]).unwrap();
    assert_eq!(packs[0].point(), Vec3::new(5214.278, 100.709, 3126.375));
    assert_eq!(packs[1].point(), Vec3::new(5215.815, 302.012, 3079.736));
    assert_eq!((edges[0].from, edges[0].to), (packs[0].id, packs[1].id));
    assert_eq!(transports[0].action, Action::TaxiProximity);
}

#[cfg(test)]
fn objective_graph() -> Graph {
    let pack = |id, x, y, spawns| Pack {
        id,
        position: [x, 0.0],
        height: y,
        spawns,
    };
    let boss = super::Spawn {
        guid: 2,
        entry: 500,
        position: [20.0, 0.0],
    };
    Graph {
        map_id: 1877,
        entrance: [0.0, 0.0],
        entrance_height: 0.0,
        entrance_pack: 1,
        packs: vec![pack(1, 0.0, 0.0, vec![]), pack(2, 20.0, 8.0, vec![boss])],
        edges: vec![],
        teleports: vec![],
        transports: vec![],
        flights: vec![],
        unreachable: vec![2],
        route: vec![],
        follower_difficulty: 8,
        follower_boss_entries: vec![500],
        follower_boss_sources: BTreeMap::from([(500, 500)]),
        follower_boss_bits: Default::default(),
        follower_encounter_actor_bits: Default::default(),
    }
}

/// Detour's closest reachable point to the boss is `edge`; walks back to start are complete.
#[cfg(test)]
fn closest_at(edge: Vec3) -> impl FnMut(Vec3, Vec3) -> Result<Vec<Vec3>, String> {
    move |from, to| Ok(vec![from, if to == Vec3::ZERO { to } else { edge }])
}

#[test]
fn non_attackable_unreachable_boss_is_anchored_at_the_closest_reachable_point() {
    let mut graph = objective_graph();
    let anchors = anchor_objective_bosses(
        &mut graph,
        |_, _| Ok(false),
        closest_at(Vec3::new(13.0, 0.0, 0.0)),
    )
    .unwrap();
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].original, [20.0, 8.0, 0.0]);
    assert_eq!(anchors[0].anchor, [13.0, 0.0, 0.0]);
    assert_eq!(anchors[0].horizontal_yards, 7.0);
    assert_eq!(anchors[0].via_pack, 1);
    assert_eq!(graph.packs[1].point(), Vec3::new(13.0, 0.0, 0.0));
    assert_eq!(graph.packs[1].spawns[0].position, [20.0, 0.0]);
    assert_eq!(
        (graph.edges[0].from, graph.edges[0].to, graph.edges[0].yards),
        (1, 2, 13.0)
    );
    assert!(graph.unreachable.is_empty());
}

#[test]
fn objective_anchor_at_the_nearest_reachable_pack_still_connects_it() {
    let mut graph = objective_graph();
    let anchors =
        anchor_objective_bosses(&mut graph, |_, _| Ok(false), |from, _| Ok(vec![from])).unwrap();
    assert_eq!(anchors[0].anchor, [0.0, 0.0, 0.0]);
    assert_eq!(anchors[0].horizontal_yards, 20.0);
    assert_eq!((graph.edges[0].from, graph.edges[0].to), (1, 2));
    assert!(graph.unreachable.is_empty());
}

#[test]
fn attackable_unreachable_boss_stays_an_error() {
    let mut graph = objective_graph();
    let error = anchor_objective_bosses(
        &mut graph,
        |_, _| Ok(true),
        closest_at(Vec3::new(13.0, 0.0, 0.0)),
    )
    .unwrap_err();
    assert!(
        error.contains("attackable boss500 source pack2 is unreachable"),
        "{error}"
    );
    assert_eq!(graph.packs[1].point(), Vec3::new(20.0, 8.0, 0.0));
    assert_eq!(graph.unreachable, vec![2]);
}

#[test]
fn objective_boss_beyond_the_anchor_bound_stays_an_error() {
    let mut graph = objective_graph();
    let error = anchor_objective_bosses(
        &mut graph,
        |_, _| Ok(false),
        closest_at(Vec3::new(-16.0, 0.0, 0.0)),
    )
    .unwrap_err();
    assert!(
        error.contains("36.0 yd from its spawn, over 35 yd"),
        "{error}"
    );
    assert_eq!(graph.packs[1].point(), Vec3::new(20.0, 8.0, 0.0));
    assert_eq!(graph.unreachable, vec![2]);
}

#[test]
fn boss_source_uses_the_spawned_vehicle_for_its_bound_accessory() {
    let packs = vec![super::Pack {
        id: 10,
        position: [1.0, 2.0],
        height: 3.0,
        spawns: vec![super::Spawn {
            guid: 10,
            entry: 210153,
            position: [1.0, 2.0],
        }],
    }];
    let sources = boss_sources(2651, &[(3, 210149)], &packs, &[(210153, 210149)]).unwrap();
    assert_eq!(sources[&210149], 210153);
    assert!(
        boss_sources(2651, &[(3, 210149)], &packs, &[])
            .unwrap_err()
            .contains("bound boss210149 missing")
    );
}

#[test]
#[ignore = "requires Azure native population and world database"]
fn azure_generic_extraction_registers_each_native_book() {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut packs = topology::source_packs(crate::test_data::world_data(), 2515, 205).unwrap();
    let registrations = server::test_support::transport::read_registrations(2515, &db).unwrap();
    let (edges, transports) =
        append_registered_transports(2515, &mut packs, &registrations).unwrap();
    assert_eq!(edges.len(), 9);
    assert_eq!(transports.len(), 9);
    assert!(
        transports
            .iter()
            .all(|edge| edge.action == server::test_support::transport::Action::AzureBook)
    );
    assert!(edges.iter().all(|edge| edge.yards == 0.0));
}

#[test]
#[ignore = "explicit follower topology extraction from world.db and navmesh"]
fn extract_follower_topology() {
    let map: u32 = std::env::var("DUNGEON_SIMULATOR_MAP")
        .expect("map id")
        .parse()
        .expect("numeric map id");
    let nav_root = std::env::var_os("DUNGEON_SIMULATOR_NAVMESH").expect("shared navmesh root");
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let difficulty = std::env::var("DUNGEON_SIMULATOR_DIFFICULTY")
        .ok()
        .map(|value| value.parse().expect("numeric difficulty id"));
    let (graph, bosses, anchors) = extract(
        map,
        &db,
        &Path::new(&nav_root).join(map.to_string()),
        difficulty,
    )
    .expect("follower topology inputs");
    let graph_path = super::graph_path();
    let mut json = serde_json::to_value(&graph).unwrap();
    json["boss_anchors"] = serde_json::to_value(&anchors).unwrap();
    std::fs::write(&graph_path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    for anchor in &anchors {
        println!("BOSS_ANCHOR {}", serde_json::to_string(anchor).unwrap());
    }
    let gates = server::test_support::objectives::boss_gate_entries(map);
    let requirements =
        serde_json::json!({(map.to_string()): {"boss_order": bosses, "gate_entries": gates}});
    let path = graph_path.with_file_name(format!(
        "{}-requirements.json",
        graph_path.file_stem().unwrap().to_string_lossy()
    ));
    std::fs::write(&path, serde_json::to_vec_pretty(&requirements).unwrap()).unwrap();
    println!(
        "FOLLOWER_TOPOLOGY map={map} packs={} edges={} unreachable={} graph={}",
        graph.packs.len(),
        graph.edges.len(),
        graph.unreachable.len(),
        graph_path.display()
    );
}
