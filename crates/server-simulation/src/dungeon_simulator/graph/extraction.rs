//! Extract follower topology from the canonical population and baked navigation.
use super::{Edge, Graph, Pack, TransportEdge, source, topology};
use bevy::prelude::Vec3;
use navigation::{NavigationMap, NavigationPlan};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn complete_route(
    nav: &mut NavigationMap,
    mut start: Vec3,
    end: Vec3,
) -> Result<Vec<Vec3>, String> {
    let mut plan = NavigationPlan::default();
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    loop {
        let leg = nav.path_with_plan(start, end, &mut plan)?;
        if leg.collision_drop.is_some() {
            return Err("navigation path requires a registered collision drop".into());
        }
        let endpoint = *leg.points.last().ok_or("empty navigation segment")?;
        result.extend(leg.points);
        if leg.complete {
            return Ok(result);
        }
        if endpoint == start || !seen.insert(endpoint.to_array().map(f32::to_bits)) {
            return Err("navigation segment repeated without reaching destination".into());
        }
        start = endpoint;
    }
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
                .filter_map(|registration| registration.source_guid),
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
                (next, guid)
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
) -> Result<(Graph, Vec<u32>), String> {
    let source = source::read_source(db, map)?;
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
        let paths = super::super::flight::build_paths(&graph, &mut nav)?;
        for path in paths {
            append_flight_path(&mut graph, &path, |from, to| {
                complete_route(&mut nav, from, to)
            })?;
        }
        graph.unreachable = topology::unreachable(
            &graph.packs,
            &graph.edges,
            &graph.teleports,
            graph.entrance_pack,
        );
    }
    Ok((graph, route_bosses))
}

fn append_flight_path(
    graph: &mut Graph,
    path: &[Vec3],
    mut walk: impl FnMut(Vec3, Vec3) -> Result<Vec<Vec3>, String>,
) -> Result<(), String> {
    let (Some(&start), Some(&end)) = (path.first(), path.last()) else {
        return Err("empty flight path".into());
    };
    let mut anchor = |point: Vec3| -> Result<u32, String> {
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
    let from = anchor(start)?;
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
    append_flight_path(&mut graph, &path, |a, b| {
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
    let (graph, bosses) = extract(map, &db, &Path::new(&nav_root).join(map.to_string()))
        .expect("follower topology inputs");
    let graph_path = super::graph_path();
    std::fs::write(&graph_path, serde_json::to_vec_pretty(&graph).unwrap()).unwrap();
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
