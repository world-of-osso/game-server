//! Explicit topology extraction; the generic corridor author supplies the pull route.
use super::*;

#[test]
#[ignore = "explicit extraction from staged2515 navigation; runtime never opens nav"]
fn extract_azure_graph() {
    let data = crate::test_data::world_data();
    let mut packs = source_packs(data, 2515);
    let final_poi = crate::npc_interaction::azure_translocation::FINAL_POI_WOW;
    packs.push(Pack {
        id: 7116,
        position: [final_poi.x, -final_poi.y],
        height: final_poi.z,
        spawns: Vec::new(),
    });
    let directory = std::path::Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-dungeons-2026-10-21/staging/navmesh/2515",
    );
    let edges = select_edges(&packs, query_edges(data, &packs, directory));
    let pack_for = |guid| {
        packs
            .iter()
            .find(|pack| pack.id == guid || pack.spawns.iter().any(|spawn| spawn.guid == guid))
            .unwrap()
            .id
    };
    let teleports = data.spawns_by_map[&2515]
        .iter()
        .map(|index| &data.spawns[*index])
        .filter_map(|spawn| {
            crate::npc_interaction::azure_translocation::LINKS
                .iter()
                .find(|link| link.entry == spawn.template_entry)
                .map(|link| Edge {
                    from: pack_for(spawn.guid),
                    to: pack_for(link.destination),
                    yards: 0.0,
                })
        })
        .collect::<Vec<_>>();
    let opening = packs
        .iter()
        .filter(|pack| pack.spawns.iter().any(|spawn| spawn.entry == 196102))
        .min_by(|a, b| {
            a.point()
                .distance_squared(Vec3::new(-5050.48, 594.216, -1024.56))
                .total_cmp(
                    &b.point()
                        .distance_squared(Vec3::new(-5050.48, 594.216, -1024.56)),
                )
        })
        .unwrap()
        .id;
    let mut combined = edges.clone();
    combined.extend(teleports.clone());
    let graph = Graph {
        map_id: 2515,
        entrance: [-5050.48, -1024.56],
        entrance_height: 594.216,
        entrance_pack: opening,
        unreachable: unreachable(&packs, &combined, opening),
        packs,
        edges,
        teleports,
        route: Vec::new(),
    };
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/azure-vault-2515.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&graph).unwrap()).unwrap();
    println!(
        "AZURE_TOPOLOGY packs={} walk_edges={} teleports={} unreachable={:?}; capture native corridor inputs then author the route",
        graph.packs.len(),
        graph.edges.len(),
        graph.teleports.len(),
        graph.unreachable
    );
}
