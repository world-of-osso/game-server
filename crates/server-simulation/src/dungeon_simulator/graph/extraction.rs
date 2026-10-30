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
        .max()
        .unwrap_or(0);
    let mut edges = Vec::new();
    let mut transports = Vec::new();
    for registration in registrations {
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
        let [(from, source_guid)] = candidates.as_slice() else {
            return Err(format!(
                "map{map} registered transport entry{} has {} matching source spawns",
                registration.entry,
                candidates.len()
            ));
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
            from: *from,
            to: next,
            yards: registration.distance,
        });
        transports.push(TransportEdge {
            from: *from,
            to: next,
            source_guid: *source_guid,
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
    let opening = nearest
        .into_iter()
        .find_map(|(_, id, point)| complete_route(&mut nav, entrance, point).ok().map(|_| id))
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
    let registrations = server::test_support::transport::read_registrations(map, db)?;
    let (teleports, transports) = append_registered_transports(map, &mut packs, &registrations)?;
    let edges = topology::connect_packs(&mut packs, |from, to| complete_route(&mut nav, from, to))?;
    let mut all_edges = edges.clone();
    all_edges.extend(teleports.iter().cloned());
    let unreachable = topology::unreachable(&packs, &all_edges, opening);
    let graph = Graph {
        map_id: map,
        entrance: [entrance.x, entrance.z],
        entrance_height: entrance.y,
        entrance_pack: opening,
        packs,
        edges,
        teleports,
        transports,
        unreachable,
        route: Vec::new(),
        follower_difficulty: source.difficulty,
        follower_boss_entries: bosses,
        follower_boss_sources: sources,
        follower_boss_bits: bits,
    };
    Ok((graph, route_bosses))
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
        source: Vec3::new(1.0, 3.0, 2.0),
        destination: Vec3::new(4.0, 30.0, 5.0),
        distance: 42.0,
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
    let requirements =
        serde_json::json!({(map.to_string()): {"boss_order": bosses, "gate_entries": {}}});
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
